use logos::Logos;
use markdown::mdast::Html;
use snailquote::unescape;

use crate::MdtResult;
use crate::Position;
use crate::tokens::Token;
use crate::tokens::TokenGroup;

/// Raw tokens produced by logos for flat tokenization of HTML node content.
#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"")]
enum RawToken {
	#[token("<!--")]
	HtmlCommentOpen,
	#[token("-->")]
	HtmlCommentClose,
	#[token("{=")]
	ConsumerTag,
	#[token("{@")]
	ProviderTag,
	#[token("{~")]
	InlineTag,
	#[token("{/")]
	CloseTag,
	#[token("}")]
	BraceClose,
	#[token("|")]
	Pipe,
	#[token(":")]
	ArgumentDelimiter,
	#[token("\n")]
	Newline,
	#[regex(r"[ \t\r]")]
	Whitespace,
	#[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
	Ident,
	#[regex(r#""([^"\\]|\\.)*""#)]
	DoubleQuotedString,
	#[regex(r"'([^'\\]|\\.)*'")]
	SingleQuotedString,
	#[regex(r"[0-9]+(\.[0-9]+)?([eE][+-]?[0-9]+)?")]
	Number,
}

/// Context states for the simplified state machine that drives
/// context-dependent token processing.
enum LexerContext {
	/// The lexer is currently outside of any tags.
	Outside,
	/// The lexer is currently inside an html comment.
	HtmlComment,
	/// The lexer is currently inside a consumer, provider or closing tag.
	Tag,
	/// The lexer is currently inside a filter.
	Filter,
}

/// Walks the logos token stream with context-dependent rules, building
/// `TokenGroup` objects.
struct TokenWalker<'a> {
	/// The source text of the current HTML node.
	source: &'a str,
	/// The collected raw tokens and their byte spans.
	raw_tokens: Vec<(Result<RawToken, ()>, std::ops::Range<usize>)>,
	/// Current index into `raw_tokens`.
	cursor: usize,
	/// The current position tracker (line/column/offset).
	position: Position,
	/// The current token group being built.
	token_group: TokenGroup,
	/// The context stack for the state machine.
	stack: Vec<LexerContext>,
	/// Collected valid groups.
	groups: Vec<TokenGroup>,
}

impl<'a> TokenWalker<'a> {
	fn new(source: &'a str, start_position: Position) -> Self {
		let raw_tokens: Vec<_> = RawToken::lexer(source).spanned().collect();

		Self {
			source,
			raw_tokens,
			cursor: 0,
			position: start_position,
			token_group: TokenGroup {
				tokens: vec![],
				position: start_position,
			},
			stack: vec![LexerContext::Outside],
			groups: vec![],
		}
	}

	/// Get the text slice for the current raw token.
	fn current_slice(&self) -> &'a str {
		let (_, span) = &self.raw_tokens[self.cursor];
		&self.source[span.clone()]
	}

	/// Advance the position tracker through a given text slice and move cursor
	/// forward.
	fn advance_cursor(&mut self) {
		let slice = self.current_slice();
		self.position.start.advance_str(slice);
		self.cursor += 1;
	}

	/// Add a token to the current token group, then advance the cursor.
	fn push_token(&mut self, token: Token, update_start: bool) {
		if update_start {
			self.token_group.position.start = self.position.start;
			self.token_group.position.end = self.position.start;
		}

		self.token_group.position.advance_end(&token);
		self.token_group.tokens.push(token);
		self.advance_cursor();
	}

	/// Finalize the current token group: if valid, push to groups. Then reset.
	fn push_token_group(&mut self) {
		let group = std::mem::replace(
			&mut self.token_group,
			TokenGroup {
				tokens: vec![],
				position: self.position,
			},
		);

		if group.is_valid() {
			self.groups.push(group);
		}

		self.stack = vec![LexerContext::Outside];
	}

	/// Reset the current token group without pushing, and reset context.
	fn reset_token_group(&mut self) {
		self.token_group = TokenGroup {
			tokens: vec![],
			position: self.position,
		};
		self.stack = vec![LexerContext::Outside];
	}

	/// When invalid content is encountered inside an HTML comment, scan forward
	/// to find `HtmlCommentClose` (`-->`), skipping everything up to and
	/// including it.
	fn exit_comment_block(&mut self) {
		self.reset_token_group();

		while self.cursor < self.raw_tokens.len() {
			let (result, _) = &self.raw_tokens[self.cursor];
			if matches!(result, Ok(RawToken::HtmlCommentClose)) {
				self.advance_cursor();
				return;
			}
			self.advance_cursor();
		}
	}

	/// Process a string token (double or single quoted). Strips quotes and
	/// unescapes if needed.
	fn process_string(&mut self, delimiter: u8) {
		let slice = self.current_slice();
		// Strip surrounding quotes
		let inner = &slice[1..slice.len() - 1];

		let has_escapes = inner.contains('\\');
		let value = if has_escapes {
			if let Ok(unescaped) = unescape(inner) {
				unescaped
			} else {
				self.exit_comment_block();
				return;
			}
		} else {
			inner.to_string()
		};

		let token = Token::String(value, delimiter);
		self.push_token(token, false);
	}

	/// Process a number token. Determines if it's a float or int.
	fn process_number(&mut self) {
		let slice = self.current_slice();
		let is_float = slice.contains('.') || slice.contains('e') || slice.contains('E');

		if is_float {
			match slice.parse::<f64>() {
				Ok(v) => self.push_token(Token::Float(v), false),
				Err(_) => self.exit_comment_block(),
			}
		} else {
			match slice.parse::<i64>() {
				Ok(v) => self.push_token(Token::Int(v), false),
				Err(_) => self.exit_comment_block(),
			}
		}
	}

	/// Main processing loop: walk the raw token stream with context-dependent
	/// rules.
	fn process(&mut self) {
		while self.cursor < self.raw_tokens.len() {
			let (result, _) = &self.raw_tokens[self.cursor];

			// Handle logos errors (unrecognized bytes): advance past them.
			let Ok(raw) = result else {
				match self.stack.last() {
					Some(LexerContext::Outside) => {
						self.advance_cursor();
					}
					Some(LexerContext::HtmlComment | LexerContext::Tag | LexerContext::Filter) => {
						self.exit_comment_block();
					}
					None => break,
				}
				continue;
			};

			match self.stack.last() {
				Some(LexerContext::Outside) => {
					match raw {
						RawToken::HtmlCommentOpen => {
							self.stack.push(LexerContext::HtmlComment);
							self.push_token(Token::HtmlCommentOpen, true);
						}
						_ => {
							// Outside context: skip everything that isn't a comment open
							self.advance_cursor();
						}
					}
				}
				Some(LexerContext::HtmlComment) => {
					match raw {
						RawToken::HtmlCommentClose => {
							self.stack.pop();
							self.push_token(Token::HtmlCommentClose, false);
							self.push_token_group();
						}
						RawToken::ConsumerTag => {
							self.stack.push(LexerContext::Tag);
							self.push_token(Token::ConsumerTag, false);
						}
						RawToken::ProviderTag => {
							self.stack.push(LexerContext::Tag);
							self.push_token(Token::ProviderTag, false);
						}
						RawToken::InlineTag => {
							self.stack.push(LexerContext::Tag);
							self.push_token(Token::InlineTag, false);
						}
						RawToken::CloseTag => {
							self.stack.push(LexerContext::Tag);
							self.push_token(Token::CloseTag, false);
						}
						RawToken::Newline => {
							self.push_token(Token::Newline, false);
						}
						RawToken::Whitespace => {
							let byte = self.current_slice().as_bytes()[0];
							self.push_token(Token::Whitespace(byte), false);
						}
						_ => {
							self.exit_comment_block();
						}
					}
				}
				Some(LexerContext::Tag) => {
					match raw {
						RawToken::BraceClose => {
							self.push_token(Token::BraceClose, false);
							self.stack.pop();
						}
						RawToken::Pipe => {
							self.stack.push(LexerContext::Filter);
							self.push_token(Token::Pipe, false);
						}
						RawToken::Ident => {
							let ident = self.current_slice().to_string();
							self.push_token(Token::Ident(ident), false);
						}
						// Accept argument delimiters and strings for block
						// arguments (e.g., {=name:"arg1":"arg2"}).
						RawToken::ArgumentDelimiter => {
							self.push_token(Token::ArgumentDelimiter, false);
						}
						RawToken::DoubleQuotedString => {
							self.process_string(b'"');
						}
						RawToken::SingleQuotedString => {
							self.process_string(b'\'');
						}
						RawToken::Newline => {
							self.push_token(Token::Newline, false);
						}
						RawToken::Whitespace => {
							let byte = self.current_slice().as_bytes()[0];
							self.push_token(Token::Whitespace(byte), false);
						}
						_ => {
							self.exit_comment_block();
						}
					}
				}
				Some(LexerContext::Filter) => {
					match raw {
						RawToken::BraceClose => {
							self.push_token(Token::BraceClose, false);
							// Pop Filter, then pop Tag
							self.stack.pop();
							self.stack.pop();
						}
						RawToken::Pipe => {
							self.push_token(Token::Pipe, false);
						}
						RawToken::ArgumentDelimiter => {
							self.push_token(Token::ArgumentDelimiter, false);
						}
						RawToken::DoubleQuotedString => {
							self.process_string(b'"');
						}
						RawToken::SingleQuotedString => {
							self.process_string(b'\'');
						}
						RawToken::Number => {
							self.process_number();
						}
						RawToken::Ident => {
							let ident = self.current_slice().to_string();
							self.push_token(Token::Ident(ident), false);
						}
						RawToken::Newline => {
							self.push_token(Token::Newline, false);
						}
						RawToken::Whitespace => {
							let byte = self.current_slice().as_bytes()[0];
							self.push_token(Token::Whitespace(byte), false);
						}
						_ => {
							self.exit_comment_block();
						}
					}
				}
				None => break,
			}
		}
	}
}

#[allow(clippy::unnecessary_wraps)]
pub fn tokenize(nodes: Vec<Html>) -> MdtResult<Vec<TokenGroup>> {
	let mut groups = Vec::new();

	for node in nodes {
		let Some(position) = node.position.map(Into::into) else {
			continue;
		};

		let position: Position = position;

		let mut walker = TokenWalker::new(&node.value, position);
		walker.process();
		groups.extend(walker.groups);
	}

	Ok(groups)
}

pub fn memstr(haystack: &[u8], needle: &[u8]) -> Option<usize> {
	haystack
		.windows(needle.len())
		.position(|window| window == needle)
}

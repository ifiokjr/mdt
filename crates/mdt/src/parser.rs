use markdown::ParseOptions;
use markdown::mdast::Html;
use markdown::mdast::Node;
use markdown::to_mdast;

use super::MdtError;
use super::MdtResult;
use crate::Position;
use crate::lexer::tokenize;
use crate::patterns::closing_pattern;
use crate::patterns::consumer_pattern;
use crate::patterns::provider_pattern;
use crate::tokens::Token;
use crate::tokens::TokenGroup;

/// Parse markdown content and return all blocks (provider and consumer) found
/// within it.
pub fn parse(content: impl AsRef<str>) -> MdtResult<Vec<Block>> {
	let content = content.as_ref();
	let html_nodes = get_html_nodes(content)?;
	let token_groups = tokenize(html_nodes)?;
	build_blocks_from_groups(&token_groups)
}

/// Build blocks from already-tokenized groups. This is the shared logic used
/// by both markdown parsing and source file scanning.
pub fn build_blocks_from_groups(token_groups: &[TokenGroup]) -> MdtResult<Vec<Block>> {
	build_blocks_inner(token_groups, false)
}

/// Like `build_blocks_from_groups`, but silently discards unclosed blocks
/// instead of returning an error. Used for source files where HTML comments
/// may appear in string literals without matching close tags.
pub fn build_blocks_from_groups_lenient(token_groups: &[TokenGroup]) -> MdtResult<Vec<Block>> {
	build_blocks_inner(token_groups, true)
}

fn build_blocks_inner(token_groups: &[TokenGroup], lenient: bool) -> MdtResult<Vec<Block>> {
	let mut pending: Vec<BlockCreator> = vec![];
	let mut blocks: Vec<Block> = vec![];

	for group in token_groups {
		match classify_group(group) {
			GroupKind::Provider { name, transformers } => {
				pending.push(BlockCreator {
					name,
					r#type: BlockType::Provider,
					opening: group.position,
					closing: None,
					transformers,
				});
			}
			GroupKind::Consumer { name, transformers } => {
				pending.push(BlockCreator {
					name,
					r#type: BlockType::Consumer,
					opening: group.position,
					closing: None,
					transformers,
				});
			}
			GroupKind::Close { name } => {
				// Find the most recent matching open block (search from the end)
				let pos = pending.iter().rposition(|bc| bc.name == name);
				if let Some(idx) = pos {
					let mut creator = pending.remove(idx);
					creator.closing = Some(group.position);
					blocks.push(creator.into_block()?);
				}
				// If no matching open block is found, silently ignore the close
				// tag. This keeps parsing lenient.
			}
			GroupKind::Unknown => {}
		}
	}

	// Any remaining unclosed blocks are errors in strict mode,
	// silently discarded in lenient mode.
	if !lenient {
		if let Some(creator) = pending.into_iter().next() {
			return Err(MdtError::MissingClosingTag(creator.name));
		}
	}

	Ok(blocks)
}

pub fn get_html_nodes(content: impl AsRef<str>) -> MdtResult<Vec<Html>> {
	let options = ParseOptions::gfm();
	let mdast =
		to_mdast(content.as_ref(), &options).map_err(|e| MdtError::Markdown(e.to_string()))?;
	let mut html_nodes = vec![];
	collect_html(&mdast, &mut html_nodes);

	Ok(html_nodes)
}

fn collect_html(node: &Node, nodes: &mut Vec<Html>) {
	match node {
		Node::Html(html) => nodes.push(html.clone()),
		_ => {
			if let Some(node) = node.children() {
				for child in node {
					collect_html(child, nodes);
				}
			}
		}
	}
}

enum GroupKind {
	Provider {
		name: String,
		transformers: Vec<Transformer>,
	},
	Consumer {
		name: String,
		transformers: Vec<Transformer>,
	},
	Close {
		name: String,
	},
	Unknown,
}

/// Classify a token group as a provider, consumer, close tag, or unknown.
fn classify_group(group: &TokenGroup) -> GroupKind {
	if group.matches_pattern(&provider_pattern()).unwrap_or(false) {
		let (name, transformers) = extract_name_and_transformers(group, &Token::ProviderTag);
		return GroupKind::Provider { name, transformers };
	}

	if group.matches_pattern(&consumer_pattern()).unwrap_or(false) {
		let (name, transformers) = extract_name_and_transformers(group, &Token::ConsumerTag);
		return GroupKind::Consumer { name, transformers };
	}

	if group.matches_pattern(&closing_pattern()).unwrap_or(false) {
		let name = extract_close_name(group);
		return GroupKind::Close { name };
	}

	GroupKind::Unknown
}

/// Extract the block name and transformers from a provider or consumer token
/// group.
fn extract_name_and_transformers(
	group: &TokenGroup,
	tag_token: &Token,
) -> (String, Vec<Transformer>) {
	let mut name = String::new();
	let mut transformers = Vec::new();
	let mut found_tag = false;
	let mut found_name = false;

	let mut iter = group.tokens.iter().peekable();

	while let Some(token) = iter.next() {
		if !found_tag {
			if token.same_type(tag_token) {
				found_tag = true;
			}
			continue;
		}

		if !found_name {
			if let Token::Ident(ident) = token {
				name.clone_from(ident);
				found_name = true;
			}
			continue;
		}

		// After the name, look for pipe-delimited transformers
		if matches!(token, Token::Pipe) {
			if let Some(transformer) = parse_transformer(&mut iter) {
				transformers.push(transformer);
			}
		}
	}

	(name, transformers)
}

/// Parse a single transformer from the token stream (after the pipe).
fn parse_transformer(
	iter: &mut std::iter::Peekable<std::slice::Iter<'_, Token>>,
) -> Option<Transformer> {
	// Skip whitespace
	while let Some(Token::Whitespace(_) | Token::Newline) = iter.peek() {
		iter.next();
	}

	// Next should be an identifier (the transformer name)
	let transformer_name = match iter.next() {
		Some(Token::Ident(name)) => name.clone(),
		_ => return None,
	};

	let transformer_type = match transformer_name.as_str() {
		"trim" => TransformerType::Trim,
		"trimStart" | "trim_start" => TransformerType::TrimStart,
		"trimEnd" | "trim_end" => TransformerType::TrimEnd,
		"wrap" => TransformerType::Wrap,
		"indent" => TransformerType::Indent,
		"codeblock" | "codeBlock" | "code_block" => TransformerType::CodeBlock,
		"code" => TransformerType::Code,
		"replace" => TransformerType::Replace,
		"prefix" => TransformerType::Prefix,
		_ => return None,
	};

	let mut args = Vec::new();

	// Collect arguments: `:value` pairs
	loop {
		// Skip whitespace
		while let Some(Token::Whitespace(_) | Token::Newline) = iter.peek() {
			iter.next();
		}

		match iter.peek() {
			Some(Token::ArgumentDelimiter) => {
				iter.next(); // consume ':'

				// Skip whitespace
				while let Some(Token::Whitespace(_) | Token::Newline) = iter.peek() {
					iter.next();
				}

				match iter.next() {
					Some(Token::String(s, _)) => args.push(Argument::String(s.clone())),
					Some(Token::Int(n)) => args.push(Argument::Number(*n as f64)),
					Some(Token::Float(n)) => args.push(Argument::Number(*n)),
					Some(Token::Ident(s)) if s == "true" => args.push(Argument::Boolean(true)),
					Some(Token::Ident(s)) if s == "false" => args.push(Argument::Boolean(false)),
					_ => break,
				}
			}
			_ => break,
		}
	}

	Some(Transformer {
		r#type: transformer_type,
		args,
	})
}

/// Extract the block name from a close tag token group.
fn extract_close_name(group: &TokenGroup) -> String {
	for token in &group.tokens {
		if let Token::CloseTag = token {
			// The name is the next Ident token after CloseTag
			let mut found_close = false;
			for t in &group.tokens {
				if found_close {
					if let Token::Ident(name) = t {
						return name.clone();
					}
				}
				if matches!(t, Token::CloseTag) {
					found_close = true;
				}
			}
		}
	}
	String::new()
}

struct BlockCreator {
	name: String,
	r#type: BlockType,
	opening: Position,
	closing: Option<Position>,
	transformers: Vec<Transformer>,
}

impl BlockCreator {
	pub fn into_block(self) -> MdtResult<Block> {
		let Some(closing) = self.closing else {
			return Err(MdtError::MissingClosingTag(self.name));
		};

		let block = Block {
			name: self.name,
			r#type: self.r#type,
			opening: self.opening,
			closing,
			transformers: self.transformers,
		};

		Ok(block)
	}
}

#[derive(Debug, Clone)]
pub struct Block {
	/// The name of the block used for matching providers to consumers.
	pub name: String,
	pub r#type: BlockType,
	pub opening: Position,
	pub closing: Position,
	pub transformers: Vec<Transformer>,
}

#[derive(Debug, Clone)]
pub struct Transformer {
	pub r#type: TransformerType,
	pub args: Vec<Argument>,
}

#[derive(Debug, Clone)]
pub enum Argument {
	String(String),
	Number(f64),
	Boolean(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformerType {
	/// Trim all whitespace from the start and end of the content.
	Trim,
	/// Trim all whitespace from the start of the content.
	TrimStart,
	/// Trim all whitespace from the end of the content.
	TrimEnd,
	/// Wrap the content in the given string.
	Wrap,
	/// Indent each line with the given string.
	Indent,
	/// Wrap the content in a codeblock with the provided language string.
	CodeBlock,
	/// Wrap the content with inline code `` `content` ``.
	Code,
	/// Replace all instances of the given string with the replacement string.
	Replace,
	/// Add a prefix string before the content.
	Prefix,
}

impl std::fmt::Display for TransformerType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Trim => write!(f, "trim"),
			Self::TrimStart => write!(f, "trimStart"),
			Self::TrimEnd => write!(f, "trimEnd"),
			Self::Wrap => write!(f, "wrap"),
			Self::Indent => write!(f, "indent"),
			Self::CodeBlock => write!(f, "codeBlock"),
			Self::Code => write!(f, "code"),
			Self::Replace => write!(f, "replace"),
			Self::Prefix => write!(f, "prefix"),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
	/// These are the blocks that are used to provide a value to any consumers.
	/// Their names can be referenced by consumers to hoist content. They should
	/// only exist within the confines of a `*.t.md` file.
	///
	/// ```md
	/// <!-- {@exampleProvider} -->
	/// <!-- {/exampleProvider} -->
	/// ```
	Provider,
	/// Consumers are blocks that have their content hoisted from a provider
	/// with the same name. They will be updated to the latest content whenever
	/// the `mdt` command is run.
	///
	/// ```md
	/// <!-- {=exampleConsumer} -->
	/// <!-- {/exampleConsumer} -->
	/// ```
	Consumer,
}

impl std::fmt::Display for BlockType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Provider => write!(f, "provider"),
			Self::Consumer => write!(f, "consumer"),
		}
	}
}

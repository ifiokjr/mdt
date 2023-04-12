use markdown::mdast::Html;
use snailquote::unescape;

use crate::MdtResult;
use crate::Position;
use crate::Token;
use crate::TokenGroup;

struct TokenizerState {
  /// The remaining html nodes
  nodes: Vec<Html>,
  /// The resolved token groups.
  groups: Vec<TokenGroup>,
  /// The current position
  position: Option<Position>,
  ///  The current node being used.
  node: Option<Html>,
  /// The current token group
  token_group: Option<TokenGroup>,
  /// The current remaining content
  content: Option<String>,
  /// Whether we are currently inside an html comment.
  stack: Vec<LexerContext>,
}

impl TokenizerState {
  fn advance(&mut self, steps: usize) -> Option<String> {
    let (skipped, remaining) = match self.content.as_ref() {
      Some(content) => {
        let (skipped, remaining) = content.split_at(steps);
        self
          .position
          .iter_mut()
          .for_each(|position| position.advance_start(skipped));
        let remaining = if remaining.is_empty() {
          None
        } else {
          Some(remaining.to_string())
        };
        (Some(skipped.to_string()), remaining)
      }
      None => (None, None),
    };

    self.content = remaining;
    skipped
  }

  /// Should be called before advance as it uses the current position.
  fn update_token_group(&mut self, token: Token, update_start: bool) {
    if let Some(group) = self.token_group.as_mut() {
      if update_start {
        if let Some(position) = self.position.as_ref() {
          group.position.start = position.start;
          group.position.end = position.start;
        }
      }

      group.position.advance_end(&token);
      group.tokens.push(token);
    }
  }

  fn whitespace(&mut self, byte: u8) {
    let token = Token::Whitespace(byte);
    self.update_token_group(token, false);
    self.advance(1);
  }

  fn newline(&mut self) {
    let token = Token::Newline;
    self.update_token_group(token, false);
    self.advance(1);
  }

  /// Collect the next identifier starting from the current character. Returns
  /// false if no identifier is found.
  fn collect_identifier(&mut self) -> bool {
    let Some(content) = self.content.as_ref() else {
      return false;
    };

    let ident_length = lex_identifier(content);

    if ident_length == 0 {
      return false;
    }

    let ident = self.advance(ident_length);

    let Some(ident) = ident else {
      return false;
    };

    let token = Token::Ident(ident);

    self.update_token_group(token, false);

    true
  }

  fn collect_string(&mut self, delimiter: u8) -> bool {
    let Some(content) = self.content.as_ref() else {
      return false;
    };

    let mut escaped = false;
    let mut has_escapes = false;
    let length = content
      .as_bytes()
      .iter()
      .skip(1)
      .take_while(|&&byte| {
        match (escaped, byte) {
          (true, _) => {
            escaped = false;
            true
          }
          (_, b'\\') => {
            escaped = true;
            has_escapes = true;
            true
          }
          (_, c) if c == delimiter => false,
          _ => true,
        }
      })
      .count();
    if escaped || content.as_bytes().get(length + 1) != Some(&delimiter) {
      return false;
    }

    let Some(string) = self
    .advance(length + 2) else {
      return false;
    };

    let Some (mut string) = string.get(1..string.len() - 1).map(|string| string.to_string()) else {
      return false;
    };

    if has_escapes {
      string = match unescape(&string).ok() {
        Some(unescaped) => unescaped,
        None => return false,
      };
    }

    let token = Token::String(string, delimiter);
    self.update_token_group(token, false);
    true
  }

  fn collect_number(&mut self) -> bool {
    let Some(content) = self.content.as_ref() else {
      return false;
    };

    #[derive(Copy, Clone)]
    enum State {
      Integer,      // 123
      Fraction,     // .123
      Exponent,     // E | e
      ExponentSign, // +|-
    }

    let mut state = State::Integer;
    let mut length = content.chars().take_while(|&c| c.is_ascii_digit()).count();
    for ch in content.chars().skip(length) {
      state = match (ch, state) {
        ('.', State::Integer) => State::Fraction,
        ('E' | 'e', State::Integer | State::Fraction) => State::Exponent,
        ('+' | '-', State::Exponent) => State::ExponentSign,
        ('0'..='9', State::Exponent) => State::ExponentSign,
        ('0'..='9', state) => state,
        _ => break,
      };
      length += 1;
    }
    let is_float = !matches!(state, State::Integer);

    let num = self.advance(length);

    let Some(num) = num else {
      return false;
    };

    let token = if is_float {
      num.parse().map(Token::Float).ok()
    } else {
      num.parse().map(Token::Int).ok()
    };

    if let Some(token) = token {
      self.update_token_group(token, false);
      true
    } else {
      false
    }
  }

  /// Push the token group to the groups `Vec` and reset the token group.
  fn push_token_group(&mut self) {
    let Some(group) = self.token_group.take() else {
      return;
    };

    println!("Pushing token group: {:?}", group.tokens);

    if group.is_valid() {
      self.groups.push(group);
    }

    self.reset_token_group();
  }

  fn reset_token_group(&mut self) {
    self.token_group = self.position.as_ref().map(|position| {
      TokenGroup {
        tokens: vec![],
        position: *position,
      }
    });

    self.stack = vec![LexerContext::Outside];
  }

  fn reset_current_node(&mut self) {
    self.node = remove_first(&mut self.nodes);
    // self.node = node.clone();
    self.position = self
      .node
      .as_ref()
      .and_then(|node| node.position.clone())
      .map(Into::into);
    self.content = self.node.as_ref().map(|node| node.value.clone());
    self.reset_token_group();
  }

  /// Call this when there is no longer any chance of the tokens being valid.
  fn exit_comment_block(&mut self) {
    self.reset_token_group();

    let Some(content) = self.content.as_ref() else {
      return;
    };

    let token = Token::HtmlCommentClose;

    let steps = if let Some(steps) = memstr(content.as_bytes(), token.to_string().as_bytes()) {
      steps + token.increment()
    } else {
      content.len()
    };

    self.advance(steps);
  }
}

pub fn tokenize(nodes: Vec<Html>) -> MdtResult<Vec<TokenGroup>> {
  let mut state = TokenizerState {
    nodes,
    groups: vec![],
    position: None,
    node: None,
    content: None,
    token_group: None,
    stack: vec![LexerContext::Outside],
  };

  loop {
    state.reset_current_node();

    if state.node.is_none() {
      break;
    }

    tokenize_node(&mut state)?;
  }

  Ok(state.groups)
}

fn tokenize_node(state: &mut TokenizerState) -> MdtResult<()> {
  loop {
    let (Some(_position), Some(content)) = (state.position.as_ref(), state.content.as_ref()) else {
      break;
    };

    match state.stack.last() {
      Some(LexerContext::Outside) => {
        if let Some("<!--") = content.get(0..4) {
          let token = Token::HtmlCommentOpen;
          // Entering an html comment
          state.stack.push(LexerContext::HtmlComment);
          state.update_token_group(token, true);
          state.advance(4);

          continue;
        }

        state.advance(1);
      }
      Some(LexerContext::HtmlComment) => {
        if let Some("-->") = content.get(0..3) {
          let token = Token::HtmlCommentClose;
          state.stack.pop();
          state.update_token_group(token, false);
          state.advance(3);
          state.push_token_group();
          continue;
        }

        match content.get(0..2) {
          Some("{=") => {
            let token = Token::ConsumerTag;
            state.stack.push(LexerContext::Tag);
            state.update_token_group(token, false);
            state.advance(2);
            continue;
          }
          Some("{@") => {
            let token = Token::ProviderTag;
            state.stack.push(LexerContext::Tag);
            state.update_token_group(token, false);
            state.advance(2);
            continue;
          }
          Some("{/") => {
            let token = Token::CloseTag;
            state.stack.push(LexerContext::Tag);
            state.update_token_group(token, false);
            state.advance(2);
            continue;
          }
          _ => {}
        }

        match content.bytes().next() {
          Some(b'\n') => {
            state.newline();
            continue;
          }
          Some(byte) if byte.is_ascii_whitespace() => {
            state.whitespace(byte);
            continue;
          }
          _ => {
            state.exit_comment_block();
            continue;
          }
        }
      }
      Some(LexerContext::Tag) => {
        match content.bytes().next() {
          Some(b'\n') => {
            state.newline();
            continue;
          }
          Some(byte) if byte.is_ascii_whitespace() => {
            state.whitespace(byte);
            continue;
          }
          Some(b'}') => {
            let token = Token::BraceClose;
            state.update_token_group(token, false);
            state.advance(1);
            state.stack.pop();
            continue;
          }
          Some(b'|') => {
            let token = Token::Pipe;
            state.update_token_group(token, false);
            state.stack.push(LexerContext::Filter);
            state.advance(1);
            continue;
          }
          Some(ch) if ch.is_ascii_alphabetic() => {
            let collected = state.collect_identifier();

            if !collected {
              state.exit_comment_block();
            }

            continue;
          }
          _ => {
            state.exit_comment_block();
          }
        }

        state.push_token_group();
        break;
      }
      Some(LexerContext::Filter) => {
        match content.bytes().next() {
          Some(b'\n') => {
            state.newline();
            continue;
          }
          Some(byte) if byte.is_ascii_whitespace() => {
            state.whitespace(byte);
            continue;
          }
          Some(b':') => {
            let token = Token::ArgumentDelimiter;
            state.update_token_group(token, false);
            state.advance(1);
            continue;
          }
          Some(b'|') => {
            let token = Token::Pipe;
            state.update_token_group(token, false);
            state.advance(1);
            continue;
          }
          Some(b'}') => {
            let token = Token::BraceClose;
            state.update_token_group(token, false);
            state.advance(1);
            state.stack.pop();
            state.stack.pop();
            continue;
          }
          Some(symbol @ (b'\'' | b'"')) => {
            let collected = state.collect_string(symbol);

            if !collected {
              state.exit_comment_block();
            }

            continue;
          }
          Some(ch) if ch.is_ascii_digit() => {
            let collected = state.collect_number();

            if !collected {
              state.exit_comment_block();
            }

            continue;
          }
          Some(ch) if ch.is_ascii_alphabetic() => {
            let collected = state.collect_identifier();

            if !collected {
              state.exit_comment_block();
            }

            continue;
          }
          _ => {
            state.exit_comment_block();
          }
        }
      }
      None => panic!("stack should never be empty"),
    }
  }

  Ok(())
}

fn remove_first<T>(list: &mut Vec<T>) -> Option<T> {
  if list.is_empty() {
    None
  } else {
    Some(list.remove(0))
  }
}

pub fn memchr(haystack: &[u8], needle: u8) -> Option<usize> {
  haystack.iter().position(|&x| x == needle)
}

pub fn memstr(haystack: &[u8], needle: &[u8]) -> Option<usize> {
  haystack
    .windows(needle.len())
    .position(|window| window == needle)
}

fn lex_identifier(content: impl AsRef<str>) -> usize {
  content
    .as_ref()
    .as_bytes()
    .iter()
    .enumerate()
    .take_while(|&(idx, &c)| {
      if c == b'_' {
        true
      } else if idx == 0 {
        c.is_ascii_alphabetic()
      } else {
        c.is_ascii_alphanumeric()
      }
    })
    .count()
}

enum LexerContext {
  /// The lexer is currently outside of any tags.
  Outside,
  /// The lexer is currently inside an html comment.
  HtmlComment,
  /// The lexer is currently inside a consumer, provider or closing tag.
  Tag,
  /// The lexer is currently inside a filters.
  Filter,
}

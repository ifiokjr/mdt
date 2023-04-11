use markdown::mdast::Html;

use crate::Position;
use crate::Result;
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
    let _steps = token.increment();

    self.update_token_group(token, false);

    true
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

pub fn tokenize(nodes: Vec<Html>) -> Result<Vec<TokenGroup>> {
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

fn tokenize_node(state: &mut TokenizerState) -> Result<()> {
  loop {
    let (Some(_position), Some(content)) = (state.position.as_ref(), state.content.as_ref()) else {
      break;
    };

    match state.stack.last() {
      Some(LexerContext::Outside) => {
        if let Some("<!--") = content.get(0..4) {
          let token = Token::HtmlCommentOpen;
          let steps = token.increment();
          // Entering an html comment
          state.stack.push(LexerContext::HtmlComment);
          state.update_token_group(token, true);
          state.advance(steps);

          continue;
        }

        state.advance(1);
      }
      Some(LexerContext::HtmlComment) => {
        if let Some("-->") = content.get(0..3) {
          let token = Token::HtmlCommentClose;
          let steps = token.increment();
          state.stack.pop();
          state.update_token_group(token, false);
          state.advance(steps);
          state.push_token_group();
          continue;
        }

        match content.get(0..2) {
          Some("{=") => {
            let token = Token::ConsumerTag;
            let steps = token.increment();
            state.stack.push(LexerContext::Tag);
            state.update_token_group(token, false);
            state.advance(steps);
            continue;
          }
          Some("{@") => {
            let token = Token::ProviderTag;
            let steps = token.increment();
            state.stack.push(LexerContext::Tag);
            state.update_token_group(token, false);
            state.advance(steps);
            continue;
          }
          Some("{/") => {
            let token = Token::CloseTag;
            let steps = token.increment();
            state.stack.push(LexerContext::Tag);
            state.update_token_group(token, false);
            state.advance(steps);
            continue;
          }
          _ => {}
        }

        match content.bytes().next() {
          Some(b'\n') => {
            let token = Token::Newline;
            let steps = token.increment();
            state.update_token_group(token, false);
            state.advance(steps);
            continue;
          }
          Some(byte) if byte.is_ascii_whitespace() => {
            let token = Token::Whitespace;
            let steps = token.increment();
            state.update_token_group(token, false);
            state.advance(steps);
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
            let token = Token::Newline;
            let steps = token.increment();
            state.update_token_group(token, false);
            state.advance(steps);
            continue;
          }
          Some(ch) if ch.is_ascii_whitespace() => {
            let token = Token::Whitespace;
            let steps = token.increment();
            state.update_token_group(token, false);
            state.advance(steps);
            continue;
          }
          Some(b'}') => {
            let token = Token::BraceClose;
            let steps = token.increment();
            state.update_token_group(token, false);
            state.advance(steps);
            state.stack.pop();
            continue;
          }
          Some(b'|') => {
            let token = Token::Pipe;
            let steps = token.increment();
            state.update_token_group(token, false);
            state.stack.push(LexerContext::Filter);
            state.advance(steps);
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
        if let Some(b':') = content.bytes().next() {
          let token = Token::ArgumentDelimiter;
          let steps = token.increment();
          state.stack.push(LexerContext::Arguments);
          state.update_token_group(token, false);
          state.advance(steps);
          continue;
        }
        state.push_token_group();
        break;
      }
      Some(LexerContext::Arguments) => {
        state.push_token_group();
        break;
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
  /// The lexer is currently inside a filter.
  Filter,
  /// The lexer is currently inside arguments.
  Arguments,
}

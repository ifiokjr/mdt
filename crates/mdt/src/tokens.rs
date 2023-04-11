use std::fmt::Display;

use crate::Position;

/// Only tokenize the blocks, not the content inside them or anything else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
  /// `\n`
  Newline,
  /// ` `
  Whitespace,
  /// `<!--`
  HtmlCommentOpen,
  /// `-->`
  HtmlCommentClose,
  /// `{=`
  ConsumerTag,
  /// `{@`
  ProviderTag,
  /// `{/`
  TagClose,
  /// `}`
  BraceClose,
  /// `|`
  Pipe,
  /// `:`
  ArgumentDelimiter,
  /// String content passed into a filter function e.g. `"my content"`
  String(String),
  /// An identifier, e.g. `exampleName`
  Ident(String),
}

impl Token {
  pub fn increment(&self) -> usize {
    match self {
      Token::Newline => 1,
      Token::Whitespace => 1,
      Token::HtmlCommentOpen => 4,
      Token::HtmlCommentClose => 3,
      Token::ProviderTag => 2,
      Token::ConsumerTag => 2,
      Token::TagClose => 2,
      Token::BraceClose => 1,
      Token::Pipe => 1,
      Token::ArgumentDelimiter => 1,
      Token::String(string) => string.len() + 2,
      Token::Ident(ident) => ident.len(),
    }
  }

  pub fn same_type(&self, other: &Token) -> bool {
    match (self, other) {
      (Token::String(_), Token::String(_)) => true,
      (Token::Ident(_), Token::Ident(_)) => true,
      _ => self == other,
    }
  }
}

impl Display for Token {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Token::Newline => writeln!(f),
      Token::Whitespace => write!(f, " "),
      Token::HtmlCommentOpen => write!(f, "<!--"),
      Token::HtmlCommentClose => write!(f, "-->"),
      Token::ConsumerTag => write!(f, "{{="),
      Token::ProviderTag => write!(f, "{{@"),
      Token::TagClose => write!(f, "{{/"),
      Token::BraceClose => write!(f, "}}"),
      Token::Pipe => write!(f, "|"),
      Token::ArgumentDelimiter => write!(f, ":"),
      Token::String(string) => write!(f, "\"{}\"", string),
      Token::Ident(ident) => write!(f, "{}", ident),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenGroup {
  pub tokens: Vec<Token>,
  pub position: Position,
}

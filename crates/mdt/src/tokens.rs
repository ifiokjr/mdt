use std::fmt::Display;

use float_cmp::approx_eq;

use crate::Position;

/// Only tokenize the blocks, not the content inside them or anything else.
#[derive(Debug, Clone)]
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
  CloseTag,
  /// `}`
  BraceClose,
  /// `|`
  Pipe,
  /// `:`
  ArgumentDelimiter,
  /// String content passed into a filter function e.g. `"my content"`
  String(String, char),
  /// An identifier, e.g. `exampleName`
  Ident(String),
  /// An integer number, e.g. `123`
  Int(i64),
  /// A floating point number, e.g. `123.456`
  Float(f64),
}

impl Eq for Token {}
impl PartialEq for Token {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Token::Newline, Token::Newline) => true,
      (Token::Whitespace, Token::Whitespace) => true,
      (Token::HtmlCommentOpen, Token::HtmlCommentOpen) => true,
      (Token::HtmlCommentClose, Token::HtmlCommentClose) => true,
      (Token::ConsumerTag, Token::ConsumerTag) => true,
      (Token::ProviderTag, Token::ProviderTag) => true,
      (Token::CloseTag, Token::CloseTag) => true,
      (Token::BraceClose, Token::BraceClose) => true,
      (Token::Pipe, Token::Pipe) => true,
      (Token::ArgumentDelimiter, Token::ArgumentDelimiter) => true,
      (Token::String(a, c), Token::String(b, d)) => a == b && c == d,
      (Token::Ident(a), Token::Ident(b)) => a == b,
      (Token::Int(a), Token::Int(b)) => a == b,
      (Token::Float(a), Token::Float(b)) => approx_eq!(f64, *a, *b, ulps = 2),
      _ => false,
    }
  }
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
      Token::CloseTag => 2,
      Token::BraceClose => 1,
      Token::Pipe => 1,
      Token::ArgumentDelimiter => 1,
      Token::String(string, _) => string.len() + 2,
      Token::Ident(ident) => ident.len(),
      Token::Int(number) => number.to_string().len(),
      Token::Float(number) => number.to_string().len(),
    }
  }

  pub fn same_type(&self, other: &Token) -> bool {
    match (self, other) {
      (Token::String(..), Token::String(..)) => true,
      (Token::Int(_), Token::Int(_)) => true,
      (Token::Float(_), Token::Float(_)) => true,
      // Ident can be a wildcard or specific name like `true` false`
      (Token::Ident(a), Token::Ident(b)) => a == b || a == "*" || b == "*",
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
      Token::CloseTag => write!(f, "{{/"),
      Token::BraceClose => write!(f, "}}"),
      Token::Pipe => write!(f, "|"),
      Token::ArgumentDelimiter => write!(f, ":"),
      Token::String(string, ch) => write!(f, "{ch}{string}{ch}"),
      Token::Ident(ident) => write!(f, "{ident}"),
      Token::Int(number) => write!(f, "{number}"),
      Token::Float(number) => write!(f, "{number}"),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenGroup {
  pub tokens: Vec<Token>,
  pub position: Position,
}

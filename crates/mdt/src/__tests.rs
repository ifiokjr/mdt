use rstest::rstest;

use super::*;
use crate::patterns::PatternMatcher;

#[rstest]
#[case::consumer(consumer_token_group(), patterns::consumer_pattern())]
#[case::provider(provider_token_group(), patterns::provider_pattern())]
#[case::closing(closing_token_group(), patterns::closing_pattern())]
fn matches_tokens(#[case] group: TokenGroup, #[case] pattern: Vec<PatternMatcher>) -> Result<()> {
  let matches = group.matches_pattern(pattern)?;
  assert!(matches);

  Ok(())
}

fn consumer_token_group() -> TokenGroup {
  TokenGroup {
    tokens: vec![
      Token::HtmlCommentOpen,
      Token::Whitespace,
      Token::ConsumerTag,
      Token::Ident("exampleName".to_string()),
      Token::BraceClose,
      Token::Whitespace,
      Token::HtmlCommentClose,
    ],
    position: Position {
      start: Point {
        line: 1,
        column: 1,
        offset: 0,
      },
      end: Point {
        line: 1,
        column: 24,
        offset: 23,
      },
    },
  }
}

fn provider_token_group() -> TokenGroup {
  TokenGroup {
    tokens: vec![
      Token::HtmlCommentOpen,
      Token::Whitespace,
      Token::ProviderTag,
      Token::Ident("exampleProvider".to_string()),
      Token::BraceClose,
      Token::Whitespace,
      Token::HtmlCommentClose,
    ],
    position: Position {
      start: Point {
        line: 1,
        column: 1,
        offset: 0,
      },
      end: Point {
        line: 1,
        column: 24,
        offset: 23,
      },
    },
  }
}

fn closing_token_group() -> TokenGroup {
  TokenGroup {
    tokens: vec![
      Token::HtmlCommentOpen,
      Token::Whitespace,
      Token::CloseTag,
      Token::Ident("example".to_string()),
      Token::BraceClose,
      Token::Whitespace,
      Token::HtmlCommentClose,
    ],
    position: Position {
      start: Point {
        line: 1,
        column: 1,
        offset: 0,
      },
      end: Point {
        line: 1,
        column: 24,
        offset: 23,
      },
    },
  }
}

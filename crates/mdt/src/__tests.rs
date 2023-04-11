use rstest::rstest;
use similar_asserts::assert_eq;

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

#[rstest]
#[case::without_comment("<div /><p>awesome</p>", vec![])]
#[case::empty_html_comment("<!--\n-->", vec![])]
#[case::invalid_html_comment(r#"<!-- abcd -->"#, vec![])]
#[case::multi_invalid_html_comment(r#"<!-- abcd --> <!-- abcd -->"#, vec![])]
#[case::consumer(r#"<!-- {=exampleName} -->"#, vec![consumer_token_group()])]
#[case::provider(r#"<!-- {@exampleProvider} -->"#, vec![provider_token_group()])]
#[case::closing(r#"<!-- {/example} -->"#, vec![closing_token_group()])]
fn generate_tokens(#[case] input: &str, #[case] expected: Vec<TokenGroup>) -> Result<()> {
  let nodes = get_html_nodes(input)?;
  let result = tokenize(nodes)?;
  assert_eq!(result, expected);

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
        column: 28,
        offset: 27,
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
        column: 20,
        offset: 19,
      },
    },
  }
}

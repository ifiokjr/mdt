use rstest::rstest;
use similar_asserts::assert_eq;

use super::__fixtures::*;
use super::*;

#[rstest]
#[case::consumer(consumer_token_group(), patterns::consumer_pattern())]
#[case::provider(provider_token_group(), patterns::provider_pattern())]
#[case::closing(closing_token_group(), patterns::closing_pattern())]
fn matches_tokens(
  #[case] group: TokenGroup,
  #[case] pattern: Vec<PatternMatcher>,
) -> MdtResult<()> {
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
#[case::closing_whitespace(" <!--\n{/example}--> ", vec![closing_token_group_no_whitespace()])]
#[case::consumer(r#"<!-- {=exampleName|trim|indent:"/// "} -->"#, vec![consumer_token_group_with_arguments()])]
fn generate_tokens(#[case] input: &str, #[case] expected: Vec<TokenGroup>) -> MdtResult<()> {
  let nodes = get_html_nodes(input)?;
  let result = tokenize(nodes)?;
  assert_eq!(result, expected);

  Ok(())
}

#[rstest]
#[case(0..1, closing_token_group(), Position::new(1, 1, 0, 1, 5, 4))]
#[case(1.., closing_token_group(), Position::new(1, 5, 4, 1, 20, 19))]
#[case(2..4, closing_token_group(), Position::new(1, 6, 5, 1, 15, 14))]
#[case(2..=4, closing_token_group(), Position::new(1, 6, 5, 1, 16, 15))]
#[case(..6, closing_token_group(), Position::new(1, 1, 0, 1, 17, 16))]
#[case(1..100, closing_token_group(), Position::new(1, 5, 4, 1, 20, 19))]
#[case(3, closing_token_group(), Position::new(1, 8, 7, 1, 15, 14))]
fn get_position_of_tokens(
  #[case] bounds: impl GetDynamicRange,
  #[case] group: TokenGroup,
  #[case] expected: Position,
) {
  let position = group.position_of_range(bounds);
  assert_eq!(position, expected);
}

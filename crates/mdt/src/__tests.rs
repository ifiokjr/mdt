use std::collections::HashMap;
use std::path::PathBuf;

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
	let matches = group.matches_pattern(&pattern)?;
	assert!(matches);

	Ok(())
}

#[rstest]
#[case::without_comment("<div /><p>awesome</p>", vec![])]
#[case::empty_html_comment("<!--\n-->", vec![])]
#[case::invalid_html_comment(r"<!-- abcd -->", vec![])]
#[case::multi_invalid_html_comment(r"<!-- abcd --> <!-- abcd -->", vec![])]
#[case::consumer(r"<!-- {=exampleName} -->", vec![consumer_token_group()])]
#[case::provider(r"<!-- {@exampleProvider} -->", vec![provider_token_group()])]
#[case::closing(r"<!-- {/example} -->", vec![closing_token_group()])]
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
	let position = group.position_of_range(&bounds);
	assert_eq!(position, expected);
}

#[test]
fn parse_provider_and_consumer_blocks() -> MdtResult<()> {
	let input = "# Title\n\n<!-- {@myBlock} -->\n\nSome provider content here.\n\n<!-- {/myBlock} \
	             -->\n\n<!-- {=myBlock} -->\n\nOld consumer content.\n\n<!-- {/myBlock} -->\n";
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 2);
	assert_eq!(blocks[0].name, "myBlock");
	assert_eq!(blocks[0].r#type, BlockType::Provider);
	assert_eq!(blocks[1].name, "myBlock");
	assert_eq!(blocks[1].r#type, BlockType::Consumer);

	Ok(())
}

#[test]
fn parse_consumer_with_transformers() -> MdtResult<()> {
	let input = r#"<!-- {=block|trim|indent:"  "} -->

content

<!-- {/block} -->
"#;
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "block");
	assert_eq!(blocks[0].r#type, BlockType::Consumer);
	assert_eq!(blocks[0].transformers.len(), 2);
	assert_eq!(blocks[0].transformers[0].r#type, TransformerType::Trim);
	assert_eq!(blocks[0].transformers[1].r#type, TransformerType::Indent);
	assert_eq!(blocks[0].transformers[1].args.len(), 1);

	Ok(())
}

#[test]
fn parse_missing_close_tag_errors() {
	let input = "<!-- {@openBlock} -->\n\nContent without close tag.\n";
	let result = parse(input);
	assert!(result.is_err());
}

#[test]
fn parse_multiple_blocks() -> MdtResult<()> {
	let input = "<!-- {@first} -->\ncontent a\n<!-- {/first} -->\n\n<!-- {@second} -->\ncontent \
	             b\n<!-- {/second} -->\n";
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 2);
	assert_eq!(blocks[0].name, "first");
	assert_eq!(blocks[1].name, "second");

	Ok(())
}

#[test]
fn parse_empty_content() -> MdtResult<()> {
	let blocks = parse("")?;
	assert!(blocks.is_empty());

	Ok(())
}

#[test]
fn parse_no_blocks() -> MdtResult<()> {
	let input = "# Just a heading\n\nSome regular markdown content.\n";
	let blocks = parse(input)?;
	assert!(blocks.is_empty());

	Ok(())
}

#[test]
fn parse_consumer_with_prefix_transformer() -> MdtResult<()> {
	let input = r#"<!-- {=docs|prefix:"\n"|indent:"//! "} -->
old
<!-- {/docs} -->
"#;
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].transformers.len(), 2);
	assert_eq!(blocks[0].transformers[0].r#type, TransformerType::Prefix);
	assert_eq!(blocks[0].transformers[1].r#type, TransformerType::Indent);

	Ok(())
}

// Config tests

#[test]
fn config_load_missing_file() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let config = MdtConfig::load(tmp.path())?;
	assert!(config.is_none());
	Ok(())
}

#[test]
fn config_load_valid() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npackage = \"package.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?;
	assert!(config.is_some());
	let config = config.unwrap_or_else(|| panic!("expected Some"));
	assert_eq!(
		config.data.get("package"),
		Some(&PathBuf::from("package.json"))
	);

	Ok(())
}

#[test]
fn config_load_malformed() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "not valid toml {{{{")
		.unwrap_or_else(|e| panic!("write: {e}"));

	let result = MdtConfig::load(tmp.path());
	assert!(result.is_err());
}

#[test]
fn config_load_data_json() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\npkg = \"data.json\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("data.json"),
		r#"{"name": "test", "version": "1.0.0"}"#,
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;

	let pkg = data.get("pkg").unwrap_or_else(|| panic!("expected pkg"));
	assert_eq!(pkg["name"], "test");
	assert_eq!(pkg["version"], "1.0.0");

	Ok(())
}

#[test]
fn config_load_data_toml() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\ncargo = \"data.toml\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("data.toml"),
		"[package]\nname = \"my-crate\"\nversion = \"0.1.0\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;

	let cargo = data
		.get("cargo")
		.unwrap_or_else(|| panic!("expected cargo"));
	assert_eq!(cargo["package"]["name"], "my-crate");
	assert_eq!(cargo["package"]["version"], "0.1.0");

	Ok(())
}

#[test]
fn config_load_data_yaml() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\ninfo = \"data.yaml\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("data.yaml"),
		"name: my-project\nversion: 2.0.0\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;

	let info = data.get("info").unwrap_or_else(|| panic!("expected info"));
	assert_eq!(info["name"], "my-project");
	assert_eq!(info["version"], "2.0.0");

	Ok(())
}

#[test]
fn config_load_data_kdl() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\nconf = \"data.kdl\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("data.kdl"),
		"name \"my-app\"\nversion \"3.0\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;

	let conf = data.get("conf").unwrap_or_else(|| panic!("expected conf"));
	assert_eq!(conf["name"], "my-app");
	assert_eq!(conf["version"], "3.0");

	Ok(())
}

#[test]
fn config_unsupported_format() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\ndata = \"data.xml\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("data.xml"), "<data/>").unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())
		.unwrap_or_else(|e| panic!("load: {e}"))
		.unwrap_or_else(|| panic!("expected Some"));
	let result = config.load_data(tmp.path());
	assert!(result.is_err());
}

// Template rendering tests

#[test]
fn render_template_with_variables() -> MdtResult<()> {
	let mut data = HashMap::new();
	data.insert(
		"package".to_string(),
		serde_json::json!({"name": "my-lib", "version": "1.2.3"}),
	);

	let content = "Install {{ package.name }} v{{ package.version }}";
	let result = render_template(content, &data)?;
	assert_eq!(result, "Install my-lib v1.2.3");

	Ok(())
}

#[test]
fn render_template_empty_data() -> MdtResult<()> {
	let data = HashMap::new();
	let content = "No variables here.";
	let result = render_template(content, &data)?;
	assert_eq!(result, "No variables here.");

	Ok(())
}

#[test]
fn render_template_no_syntax() -> MdtResult<()> {
	let mut data = HashMap::new();
	data.insert("pkg".to_string(), serde_json::json!({"name": "test"}));

	let content = "Plain text without template syntax.";
	let result = render_template(content, &data)?;
	assert_eq!(result, "Plain text without template syntax.");

	Ok(())
}

#[test]
fn render_template_nested_access() -> MdtResult<()> {
	let mut data = HashMap::new();
	data.insert(
		"cargo".to_string(),
		serde_json::json!({
			"package": {
				"name": "my-crate",
				"version": "0.1.0",
				"edition": "2024"
			}
		}),
	);

	let content = "{{ cargo.package.name }} edition {{ cargo.package.edition }}";
	let result = render_template(content, &data)?;
	assert_eq!(result, "my-crate edition 2024");

	Ok(())
}

// Source scanner tests

#[test]
fn source_scanner_extract_html_comments() {
	let content = "// some code\n// <!-- {=block} -->\n// content\n// <!-- {/block} -->\n";
	let nodes = extract_html_comments(content);
	assert_eq!(nodes.len(), 2);
	assert_eq!(nodes[0].value, "<!-- {=block} -->");
	assert_eq!(nodes[1].value, "<!-- {/block} -->");
}

#[test]
fn source_scanner_parse_source_ts() -> MdtResult<()> {
	let content = r#"/**
 * <!-- {=docs} -->
 * old content
 * <!-- {/docs} -->
 */
export function hello() {}
"#;
	let blocks = parse_source(content)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "docs");
	assert_eq!(blocks[0].r#type, BlockType::Consumer);

	Ok(())
}

#[test]
fn source_scanner_parse_source_rs() -> MdtResult<()> {
	let content = r#"//! <!-- {=myDocs} -->
//! Some documentation.
//! <!-- {/myDocs} -->

pub fn main() {}
"#;
	let blocks = parse_source(content)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "myDocs");
	assert_eq!(blocks[0].r#type, BlockType::Consumer);

	Ok(())
}

#[test]
fn source_scanner_lenient_unclosed() -> MdtResult<()> {
	let content = "// <!-- {=unclosed} -->\n// no close tag\n";
	let blocks = parse_source(content)?;
	assert!(blocks.is_empty());

	Ok(())
}

#[test]
fn source_scanner_with_transformers() -> MdtResult<()> {
	let content = r#"// <!-- {=block|trim|indent:"/// "} -->
// old
// <!-- {/block} -->
"#;
	let blocks = parse_source(content)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].transformers.len(), 2);
	assert_eq!(blocks[0].transformers[0].r#type, TransformerType::Trim);
	assert_eq!(blocks[0].transformers[1].r#type, TransformerType::Indent);

	Ok(())
}

#[test]
fn source_scanner_no_comments() -> MdtResult<()> {
	let content = "fn main() {\n\tprintln!(\"hello\");\n}\n";
	let blocks = parse_source(content)?;
	assert!(blocks.is_empty());

	Ok(())
}

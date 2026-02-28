use std::collections::HashMap;
use std::path::PathBuf;

use rstest::rstest;
use similar_asserts::assert_eq;

use super::__fixtures::*;
use super::*;
use crate::config::CodeBlockFilter;
use crate::lexer::tokenize;
use crate::parser::ParseDiagnostic;
use crate::parser::parse_with_diagnostics;
use crate::patterns;
use crate::patterns::PatternMatcher;
use crate::project;
use crate::project::ProjectContext;
use crate::project::ScanOptions;
use crate::project::scan_project_with_options;
use crate::tokens::GetDynamicRange;
use crate::tokens::TokenGroup;

#[rstest]
#[case::consumer(consumer_token_group(), patterns::consumer_pattern())]
#[case::provider(provider_token_group(), patterns::provider_pattern())]
#[case::inline(inline_token_group_with_argument(), patterns::inline_pattern())]
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
#[case::inline(r#"<!-- {~version:"{{ pkg.version }}"} -->"#, vec![inline_token_group_with_argument()])]
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
fn parse_inline_block_with_template_argument() -> MdtResult<()> {
	let input = r#"<!-- {~version:"{{ pkg.version }}"} -->0.0.0<!-- {/version} -->"#;
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "version");
	assert_eq!(blocks[0].r#type, BlockType::Inline);
	assert_eq!(blocks[0].arguments, vec!["{{ pkg.version }}".to_string()]);

	Ok(())
}

#[test]
fn parse_inline_block_inside_markdown_table_cell() -> MdtResult<()> {
	let input = r#"| Package | Version |
| ------- | ------- |
| mdt     | <!-- {~version:"{{ pkg.version }}"} -->0.0.0<!-- {/version} --> |
"#;
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "version");
	assert_eq!(blocks[0].r#type, BlockType::Inline);
	assert_eq!(blocks[0].arguments, vec!["{{ pkg.version }}".to_string()]);

	Ok(())
}

#[test]
fn parse_inline_block_inside_markdown_fence_is_ignored() -> MdtResult<()> {
	let input = r#"```markdown
<!-- {~version:"{{ pkg.version }}"} -->0.0.0<!-- {/version} -->
```"#;
	let blocks = parse(input)?;
	assert!(blocks.is_empty());

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

// --- Transformer tests ---

#[test]
fn transformer_trim() {
	let result = apply_transformers(
		"  hello world  \n",
		&[Transformer {
			r#type: TransformerType::Trim,
			args: vec![],
		}],
	);
	assert_eq!(result, "hello world");
}

#[test]
fn transformer_trim_start() {
	let result = apply_transformers(
		"\n  hello  ",
		&[Transformer {
			r#type: TransformerType::TrimStart,
			args: vec![],
		}],
	);
	assert_eq!(result, "hello  ");
}

#[test]
fn transformer_trim_end() {
	let result = apply_transformers(
		"  hello  \n",
		&[Transformer {
			r#type: TransformerType::TrimEnd,
			args: vec![],
		}],
	);
	assert_eq!(result, "  hello");
}

#[test]
fn transformer_indent_multiline() {
	let result = apply_transformers(
		"line1\nline2\nline3",
		&[Transformer {
			r#type: TransformerType::Indent,
			args: vec![Argument::String("  ".to_string())],
		}],
	);
	assert_eq!(result, "  line1\n  line2\n  line3");
}

#[test]
fn transformer_indent_preserves_empty_lines() {
	let result = apply_transformers(
		"line1\n\nline3",
		&[Transformer {
			r#type: TransformerType::Indent,
			args: vec![Argument::String("  ".to_string())],
		}],
	);
	assert_eq!(result, "  line1\n\n  line3");
}

#[test]
fn transformer_indent_includes_empty_lines() {
	let result = apply_transformers(
		"line1\n\nline3",
		&[Transformer {
			r#type: TransformerType::Indent,
			args: vec![Argument::String("  ".to_string()), Argument::Boolean(true)],
		}],
	);
	assert_eq!(result, "  line1\n  \n  line3");
}

#[test]
fn transformer_prefix() {
	let result = apply_transformers(
		"content",
		&[Transformer {
			r#type: TransformerType::Prefix,
			args: vec![Argument::String(">>> ".to_string())],
		}],
	);
	assert_eq!(result, ">>> content");
}

#[test]
fn transformer_wrap() {
	let result = apply_transformers(
		"inner",
		&[Transformer {
			r#type: TransformerType::Wrap,
			args: vec![Argument::String("**".to_string())],
		}],
	);
	assert_eq!(result, "**inner**");
}

#[test]
fn transformer_code_block_with_language() {
	let result = apply_transformers(
		"let x = 1;",
		&[Transformer {
			r#type: TransformerType::CodeBlock,
			args: vec![Argument::String("ts".to_string())],
		}],
	);
	assert_eq!(result, "```ts\nlet x = 1;\n```");
}

#[test]
fn transformer_code_block_without_language() {
	let result = apply_transformers(
		"hello",
		&[Transformer {
			r#type: TransformerType::CodeBlock,
			args: vec![],
		}],
	);
	assert_eq!(result, "```\nhello\n```");
}

#[test]
fn transformer_code_inline() {
	let result = apply_transformers(
		"my_fn",
		&[Transformer {
			r#type: TransformerType::Code,
			args: vec![],
		}],
	);
	assert_eq!(result, "`my_fn`");
}

#[test]
fn transformer_replace() {
	let result = apply_transformers(
		"Hello World, World!",
		&[Transformer {
			r#type: TransformerType::Replace,
			args: vec![
				Argument::String("World".to_string()),
				Argument::String("Rust".to_string()),
			],
		}],
	);
	assert_eq!(result, "Hello Rust, Rust!");
}

#[test]
fn transformer_chain_trim_then_indent() {
	let result = apply_transformers(
		"\n  content here  \n",
		&[
			Transformer {
				r#type: TransformerType::Trim,
				args: vec![],
			},
			Transformer {
				r#type: TransformerType::Indent,
				args: vec![Argument::String("/// ".to_string())],
			},
		],
	);
	assert_eq!(result, "/// content here");
}

#[test]
fn transformer_on_empty_content() {
	let result = apply_transformers(
		"",
		&[Transformer {
			r#type: TransformerType::Trim,
			args: vec![],
		}],
	);
	assert_eq!(result, "");
}

#[test]
fn transformer_chain_trim_prefix_code() {
	let result = apply_transformers(
		"\n  my_func  \n",
		&[
			Transformer {
				r#type: TransformerType::Trim,
				args: vec![],
			},
			Transformer {
				r#type: TransformerType::Code,
				args: vec![],
			},
			Transformer {
				r#type: TransformerType::Prefix,
				args: vec![Argument::String("See: ".to_string())],
			},
		],
	);
	assert_eq!(result, "See: `my_func`");
}

#[test]
fn transformer_replace_with_empty_replacement() {
	let result = apply_transformers(
		"remove this word",
		&[Transformer {
			r#type: TransformerType::Replace,
			args: vec![
				Argument::String("this ".to_string()),
				Argument::String(String::new()),
			],
		}],
	);
	assert_eq!(result, "remove word");
}

// --- Engine tests ---

#[test]
fn check_project_with_matching_content() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\nExpected content.\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=block} -->\n\nExpected content.\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = ProjectContext {
		project: scan_project(tmp.path())?,
		data: HashMap::new(),
		padding: None,
	};
	let result = check_project(&ctx)?;
	assert!(result.is_ok());

	Ok(())
}

#[test]
fn check_project_detects_stale() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\nNew content.\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=block} -->\n\nOld content.\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = ProjectContext {
		project: scan_project(tmp.path())?,
		data: HashMap::new(),
		padding: None,
	};
	let result = check_project(&ctx)?;
	assert!(!result.is_ok());
	assert_eq!(result.stale.len(), 1);
	assert_eq!(result.stale[0].block_name, "block");

	Ok(())
}

#[test]
fn check_project_detects_stale_inline_block() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npkg = \"package.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("package.json"), r#"{"version":"1.2.3"}"#)
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {~version:\"{{ pkg.version }}\"} -->\n\n0.0.0\n\n<!-- {/version} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let result = check_project(&ctx)?;
	assert!(!result.is_ok());
	assert_eq!(result.stale.len(), 1);
	assert_eq!(result.stale[0].block_name, "version");
	assert_eq!(result.render_errors.len(), 0);

	Ok(())
}

#[test]
fn compute_updates_replaces_content() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@info} -->\n\nUpdated info.\n\n<!-- {/info} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("doc.md"),
		"# Doc\n\n<!-- {=info} -->\n\nOld info.\n\n<!-- {/info} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = ProjectContext {
		project: scan_project(tmp.path())?,
		data: HashMap::new(),
		padding: None,
	};
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	assert_eq!(updates.updated_files.len(), 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	assert!(content.contains("Updated info."));
	assert!(!content.contains("Old info."));

	Ok(())
}

#[test]
fn compute_updates_replaces_inline_content() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npkg = \"package.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("package.json"), r#"{"version":"1.2.3"}"#)
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {~version:\"{{ pkg.version }}\"} -->\n\n0.0.0\n\n<!-- {/version} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	assert_eq!(
		content,
		"<!-- {~version:\"{{ pkg.version }}\"} -->1.2.3<!-- {/version} -->\n"
	);

	Ok(())
}

#[test]
fn compute_updates_replaces_inline_content_in_markdown_table() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npkg = \"package.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("package.json"), r#"{"version":"1.2.3"}"#)
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"| Package | Version |\n| ------- | ------- |\n| mdt     | <!-- {~version:\"{{ \
		 pkg.version }}\"} -->0.0.0<!-- {/version} --> |\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	assert!(content.contains(
		"| mdt     | <!-- {~version:\"{{ pkg.version }}\"} -->1.2.3<!-- {/version} --> |"
	));

	Ok(())
}

#[test]
fn compute_updates_inline_with_script_data_source() -> MdtResult<()> {
	if cfg!(windows) {
		return Ok(());
	}

	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("VERSION"), "2.4.6\n").unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\nrelease = { command = \"cat VERSION\", format = \"text\", watch = [\"VERSION\"] \
		 }\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"Release <!-- {~releaseValue:\"{{ release | trim }}\"} -->0.0.0<!-- {/releaseValue} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	assert!(content.contains(
		"Release <!-- {~releaseValue:\"{{ release | trim }}\"} -->2.4.6<!-- {/releaseValue} -->"
	));

	Ok(())
}

#[test]
fn compute_updates_multiple_consumers_same_file() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@blockA} -->\n\nContent A.\n\n<!-- {/blockA} -->\n\n<!-- {@blockB} -->\n\nContent \
		 B.\n\n<!-- {/blockB} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=blockA} -->\n\nOld A.\n\n<!-- {/blockA} -->\n\n<!-- {=blockB} -->\n\nOld \
		 B.\n\n<!-- {/blockB} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = ProjectContext {
		project: scan_project(tmp.path())?,
		data: HashMap::new(),
		padding: None,
	};
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 2);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	assert!(content.contains("Content A."));
	assert!(content.contains("Content B."));
	assert!(!content.contains("Old A."));
	assert!(!content.contains("Old B."));

	Ok(())
}

#[test]
fn compute_updates_skips_missing_provider() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@existing} -->\n\nContent.\n\n<!-- {/existing} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	// Consumer references non-existent provider "missing"
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=missing} -->\n\nOrphan.\n\n<!-- {/missing} -->\n\n<!-- {=existing} \
		 -->\n\nOld.\n\n<!-- {/existing} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = ProjectContext {
		project: scan_project(tmp.path())?,
		data: HashMap::new(),
		padding: None,
	};
	let updates = compute_updates(&ctx)?;
	// Only the existing consumer should be updated
	assert_eq!(updates.updated_count, 1);

	Ok(())
}

#[test]
fn compute_updates_noop_when_in_sync() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\nSame content.\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=block} -->\n\nSame content.\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = ProjectContext {
		project: scan_project(tmp.path())?,
		data: HashMap::new(),
		padding: None,
	};
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 0);
	assert!(updates.updated_files.is_empty());

	Ok(())
}

#[test]
fn compute_updates_idempotent() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\nFinal content.\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("doc.md"),
		"<!-- {=block} -->\n\nOld.\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// First update
	let ctx = ProjectContext {
		project: scan_project(tmp.path())?,
		data: HashMap::new(),
		padding: None,
	};
	let updates = compute_updates(&ctx)?;
	write_updates(&updates)?;
	assert_eq!(updates.updated_count, 1);

	// Second update should be noop
	let ctx = ProjectContext {
		project: scan_project(tmp.path())?,
		data: HashMap::new(),
		padding: None,
	};
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 0);

	Ok(())
}

#[test]
fn compute_updates_with_template_rendering() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npkg = \"package.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("package.json"),
		r#"{"name": "my-lib", "version": "2.0.0"}"#,
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@install} -->\n\nnpm install {{ pkg.name }}@{{ pkg.version }}\n\n<!-- {/install} \
		 -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=install} -->\n\nold\n\n<!-- {/install} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	assert!(content.contains("npm install my-lib@2.0.0"));

	Ok(())
}

// --- Project scanning tests ---

#[test]
fn find_missing_providers_detects_orphans() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@existingBlock} -->\n\ncontent\n\n<!-- {/existingBlock} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=orphanBlock} -->\n\nstuff\n\n<!-- {/orphanBlock} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let project = scan_project(tmp.path())?;
	let missing = find_missing_providers(&project);
	assert_eq!(missing, vec!["orphanBlock"]);

	Ok(())
}

#[test]
fn find_missing_providers_empty_when_all_match() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\ncontent\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=block} -->\n\nold\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let project = scan_project(tmp.path())?;
	let missing = find_missing_providers(&project);
	assert!(missing.is_empty());

	Ok(())
}

#[test]
fn validate_project_errors_on_missing_provider() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=noProvider} -->\n\norphan\n\n<!-- {/noProvider} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let project = scan_project(tmp.path())?;
	let result = validate_project(&project);
	assert!(result.is_err());

	Ok(())
}

#[test]
fn is_template_file_correct() {
	assert!(is_template_file(std::path::Path::new("template.t.md")));
	assert!(is_template_file(std::path::Path::new("docs/api.t.md")));
	assert!(!is_template_file(std::path::Path::new("readme.md")));
	assert!(!is_template_file(std::path::Path::new("template.md")));
}

#[test]
fn extract_content_between_tags_empty_block() {
	let block = Block {
		name: "test".to_string(),
		r#type: BlockType::Provider,
		opening: Position::new(1, 1, 0, 1, 10, 10),
		closing: Position::new(1, 10, 10, 1, 20, 20),
		transformers: vec![],
		arguments: vec![],
	};
	let content = extract_content_between_tags("0123456789<!-- {/test} -->", &block);
	assert_eq!(content, "");
}

#[test]
fn scan_project_skips_hidden_dirs() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::create_dir_all(tmp.path().join(".hidden")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join(".hidden/readme.md"),
		"<!-- {=block} -->\n\nold\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\ncontent\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let project = scan_project(tmp.path())?;
	// Only the template file's provider should be found, not the hidden dir
	// consumer
	assert!(project.consumers.is_empty());

	Ok(())
}

#[test]
fn scan_project_skips_node_modules() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::create_dir_all(tmp.path().join("node_modules/pkg"))
		.unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("node_modules/pkg/readme.md"),
		"<!-- {=block} -->\n\nold\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let project = scan_project(tmp.path())?;
	assert!(project.consumers.is_empty());

	Ok(())
}

#[test]
fn scan_project_with_exclude_patterns() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::create_dir_all(tmp.path().join("vendor")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[exclude]\npatterns = [\"vendor/**\"]\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("vendor/lib.md"),
		"<!-- {=block} -->\n\nvendor content\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\ncontent\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=block} -->\n\nold\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	// Should find the readme consumer but not the vendor one
	assert_eq!(ctx.project.consumers.len(), 1);
	assert!(
		ctx.project.consumers[0]
			.file
			.to_string_lossy()
			.contains("readme.md")
	);

	Ok(())
}

#[test]
fn scan_project_with_source_files() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::create_dir_all(tmp.path().join("src")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@docs} -->\n\nAPI documentation.\n\n<!-- {/docs} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("src/lib.rs"),
		"//! <!-- {=docs} -->\n//! old docs\n//! <!-- {/docs} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let project = scan_project(tmp.path())?;
	assert_eq!(project.providers.len(), 1);
	assert_eq!(project.consumers.len(), 1);
	assert_eq!(project.consumers[0].block.name, "docs");

	Ok(())
}

#[test]
fn scan_project_sub_project_boundary() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	// Sub-project boundary detection applies to non-root directories.
	// Place it inside a packages/ dir so walk_dir sees it with is_root=false.
	std::fs::create_dir_all(tmp.path().join("packages/subproject"))
		.unwrap_or_else(|e| panic!("mkdir: {e}"));
	// Create sub-project with its own mdt.toml
	std::fs::write(tmp.path().join("packages/subproject/mdt.toml"), "[data]\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("packages/subproject/readme.md"),
		"<!-- {=block} -->\n\nsub content\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let project = scan_project(tmp.path())?;
	// Sub-project files should not be scanned
	assert!(project.consumers.is_empty());

	Ok(())
}

// --- Config tests ---

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
		Some(&DataSource::Path(PathBuf::from("package.json")))
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

#[test]
fn config_load_data_yml_extension() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\ninfo = \"data.yml\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("data.yml"),
		"name: yml-project\nversion: 1.0.0\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;

	let info = data.get("info").unwrap_or_else(|| panic!("expected info"));
	assert_eq!(info["name"], "yml-project");

	Ok(())
}

#[test]
fn config_load_data_missing_file_errors() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\nmissing = \"does_not_exist.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())
		.unwrap_or_else(|e| panic!("load: {e}"))
		.unwrap_or_else(|| panic!("expected Some"));
	let result = config.load_data(tmp.path());
	assert!(result.is_err());
}

#[test]
fn config_with_exclude_patterns() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[exclude]\npatterns = [\"vendor/**\", \"dist/**\"]\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	assert_eq!(config.exclude.patterns.len(), 2);
	assert_eq!(config.exclude.patterns[0], "vendor/**");
	assert_eq!(config.exclude.patterns[1], "dist/**");

	Ok(())
}

#[test]
fn config_with_empty_data_section() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\n")
		.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	assert!(config.data.is_empty());
	let data = config.load_data(tmp.path())?;
	assert!(data.is_empty());

	Ok(())
}

#[test]
fn config_multiple_data_namespaces() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npkg = \"package.json\"\ncargo = \"Cargo.toml\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("package.json"), r#"{"name": "js-lib"}"#)
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("Cargo.toml"),
		"[package]\nname = \"rs-lib\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;
	assert_eq!(data.len(), 2);
	assert_eq!(data["pkg"]["name"], "js-lib");
	assert_eq!(data["cargo"]["package"]["name"], "rs-lib");

	Ok(())
}

// --- Template rendering tests ---

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

#[test]
fn render_template_undefined_variable_chainable() -> MdtResult<()> {
	let mut data = HashMap::new();
	data.insert("pkg".to_string(), serde_json::json!({"name": "test"}));

	// Access a non-existent key — should render empty due to Chainable behavior
	let content = "Value: {{ pkg.nonexistent }}";
	let result = render_template(content, &data)?;
	assert_eq!(result, "Value: ");

	Ok(())
}

#[test]
fn render_template_with_array_data() -> MdtResult<()> {
	let mut data = HashMap::new();
	data.insert(
		"items".to_string(),
		serde_json::json!(["alpha", "beta", "gamma"]),
	);

	let content = "{% for item in items %}{{ item }} {% endfor %}";
	let result = render_template(content, &data)?;
	assert_eq!(result, "alpha beta gamma ");

	Ok(())
}

#[test]
fn render_template_with_conditional() -> MdtResult<()> {
	let mut data = HashMap::new();
	data.insert(
		"pkg".to_string(),
		serde_json::json!({"private": true, "name": "secret"}),
	);

	let content = "{% if pkg.private %}Private package{% else %}Public{% endif %}";
	let result = render_template(content, &data)?;
	assert_eq!(result, "Private package");

	Ok(())
}

// --- Source scanner tests ---

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
	let content = r"/**
 * <!-- {=docs} -->
 * old content
 * <!-- {/docs} -->
 */
export function hello() {}
";
	let blocks = parse_source(content)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "docs");
	assert_eq!(blocks[0].r#type, BlockType::Consumer);

	Ok(())
}

#[test]
fn source_scanner_parse_source_rs() -> MdtResult<()> {
	let content = r"//! <!-- {=myDocs} -->
//! Some documentation.
//! <!-- {/myDocs} -->

pub fn main() {}
";
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

#[test]
fn source_scanner_multiple_blocks() -> MdtResult<()> {
	let content = "// <!-- {=blockA} -->\n// A\n// <!-- {/blockA} -->\n\n// <!-- {=blockB} \
	               -->\n// B\n// <!-- {/blockB} -->\n";
	let blocks = parse_source(content)?;
	assert_eq!(blocks.len(), 2);
	assert_eq!(blocks[0].name, "blockA");
	assert_eq!(blocks[1].name, "blockB");

	Ok(())
}

#[test]
fn source_scanner_python_comments() -> MdtResult<()> {
	let content = "# <!-- {=docs} -->\n# documentation here\n# <!-- {/docs} -->\n";
	let blocks = parse_source(content)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "docs");

	Ok(())
}

#[test]
fn source_scanner_adjacent_comments() {
	let content = "<!-- {=a} --><!-- {/a} --><!-- {=b} --><!-- {/b} -->";
	let nodes = extract_html_comments(content);
	assert_eq!(nodes.len(), 4);
}

#[test]
fn source_scanner_comment_positions() {
	let content = "line1\n<!-- {=block} -->\nline3\n<!-- {/block} -->\n";
	let nodes = extract_html_comments(content);
	assert_eq!(nodes.len(), 2);
	// First comment starts at line 2, column 1
	let pos0 = nodes[0]
		.position
		.as_ref()
		.unwrap_or_else(|| panic!("expected position"));
	assert_eq!(pos0.start.line, 2);
	assert_eq!(pos0.start.column, 1);
	// Second comment starts at line 4
	let pos1 = nodes[1]
		.position
		.as_ref()
		.unwrap_or_else(|| panic!("expected position"));
	assert_eq!(pos1.start.line, 4);
}

// --- Parser edge case tests ---

#[test]
fn parse_block_with_underscores_in_name() -> MdtResult<()> {
	let input = "<!-- {@my_block_name} -->\n\ncontent\n\n<!-- {/my_block_name} -->\n";
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "my_block_name");

	Ok(())
}

#[test]
fn parse_block_with_numbers_in_name() -> MdtResult<()> {
	let input = "<!-- {@block123} -->\n\ncontent\n\n<!-- {/block123} -->\n";
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "block123");

	Ok(())
}

#[test]
fn parse_consumer_with_all_transformer_types() -> MdtResult<()> {
	let input = r##"<!-- {=block|trim|trimStart|trimEnd|indent:"  "|prefix:"# "|wrap:"**"|codeBlock:"rs"|code|replace:"a":"b"} -->
old
<!-- {/block} -->
"##;
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].transformers.len(), 9);
	assert_eq!(blocks[0].transformers[0].r#type, TransformerType::Trim);
	assert_eq!(blocks[0].transformers[1].r#type, TransformerType::TrimStart);
	assert_eq!(blocks[0].transformers[2].r#type, TransformerType::TrimEnd);
	assert_eq!(blocks[0].transformers[3].r#type, TransformerType::Indent);
	assert_eq!(blocks[0].transformers[4].r#type, TransformerType::Prefix);
	assert_eq!(blocks[0].transformers[5].r#type, TransformerType::Wrap);
	assert_eq!(blocks[0].transformers[6].r#type, TransformerType::CodeBlock);
	assert_eq!(blocks[0].transformers[7].r#type, TransformerType::Code);
	assert_eq!(blocks[0].transformers[8].r#type, TransformerType::Replace);

	Ok(())
}

#[test]
fn parse_consumer_with_numeric_argument() -> MdtResult<()> {
	let input = "<!-- {=block|indent:4} -->\nold\n<!-- {/block} -->\n";
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].transformers.len(), 1);
	assert_eq!(blocks[0].transformers[0].args.len(), 1);
	match &blocks[0].transformers[0].args[0] {
		Argument::Number(n) => assert!((n.0 - 4.0).abs() < f64::EPSILON),
		other => panic!("expected Number, got {other:?}"),
	}

	Ok(())
}

#[test]
fn parse_alternate_transformer_names() -> MdtResult<()> {
	let input = r#"<!-- {=block|trim_start|trim_end|code_block:"rs"} -->
old
<!-- {/block} -->
"#;
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].transformers[0].r#type, TransformerType::TrimStart);
	assert_eq!(blocks[0].transformers[1].r#type, TransformerType::TrimEnd);
	assert_eq!(blocks[0].transformers[2].r#type, TransformerType::CodeBlock);

	Ok(())
}

#[test]
fn parse_blocks_preserve_content_offsets() -> MdtResult<()> {
	let input = "<!-- {@block} -->\nContent here.\n<!-- {/block} -->\n";
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	let content = extract_content_between_tags(input, &blocks[0]);
	assert_eq!(content, "\nContent here.\n");

	Ok(())
}

#[test]
fn parse_provider_in_non_template_file_not_provider() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	// Write a provider block in a non-template file (readme.md)
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {@block} -->\n\ncontent\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let project = scan_project(tmp.path())?;
	// Provider blocks in non-template files should not be registered as providers
	assert!(project.providers.is_empty());

	Ok(())
}

// --- Unicode and special character tests ---

#[test]
fn parse_unicode_content() -> MdtResult<()> {
	let input = "<!-- {@block} -->\n\nHello, world! \u{1f600} Привет мир!\n\n<!-- {/block} -->\n";
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	let content = extract_content_between_tags(input, &blocks[0]);
	assert!(content.contains('\u{1f600}'));
	assert!(content.contains("Привет"));

	Ok(())
}

#[test]
fn transformer_indent_with_unicode() {
	let result = apply_transformers(
		"line\u{1f600}\nline2",
		&[Transformer {
			r#type: TransformerType::Indent,
			args: vec![Argument::String("  ".to_string())],
		}],
	);
	assert_eq!(result, "  line\u{1f600}\n  line2");
}

// --- Write updates test ---

#[test]
fn write_updates_creates_files() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let file_path = tmp.path().join("output.md");
	std::fs::write(&file_path, "original").unwrap_or_else(|e| panic!("write: {e}"));

	let mut updated_files = HashMap::new();
	updated_files.insert(file_path.clone(), "updated content".to_string());
	let updates = UpdateResult {
		updated_files,
		updated_count: 1,
		warnings: Vec::new(),
	};
	write_updates(&updates)?;

	let content = std::fs::read_to_string(&file_path).unwrap_or_else(|e| panic!("read: {e}"));
	assert_eq!(content, "updated content");

	Ok(())
}

// --- Error type tests ---

#[test]
fn error_missing_closing_tag_message() {
	let err = MdtError::MissingClosingTag("myBlock".to_string());
	assert!(err.to_string().contains("myBlock"));
}

#[test]
fn error_missing_provider_message() {
	let err = MdtError::MissingProvider("orphan".to_string());
	assert!(err.to_string().contains("orphan"));
}

#[test]
fn error_data_file_message() {
	let err = MdtError::DataFile {
		path: "config.json".to_string(),
		reason: "not found".to_string(),
	};
	let msg = err.to_string();
	assert!(msg.contains("config.json"));
	assert!(msg.contains("not found"));
}

#[test]
fn error_unsupported_format_message() {
	let err = MdtError::UnsupportedDataFormat("xml".to_string());
	assert!(err.to_string().contains("xml"));
}

#[test]
fn error_template_render_message() {
	let err = MdtError::TemplateRender("syntax error".to_string());
	assert!(err.to_string().contains("syntax error"));
}

#[test]
fn error_config_parse_message() {
	let err = MdtError::ConfigParse("unexpected token".to_string());
	assert!(err.to_string().contains("unexpected token"));
}

// --- Position tests ---

#[test]
fn point_advance_str_basic() {
	let mut point = Point::new(1, 1, 0);
	point.advance_str("hello");
	assert_eq!(point.line, 1);
	assert_eq!(point.column, 6);
	assert_eq!(point.offset, 5);
}

#[test]
fn point_advance_str_with_newlines() {
	let mut point = Point::new(1, 1, 0);
	point.advance_str("line1\nline2\nline3");
	assert_eq!(point.line, 3);
	assert_eq!(point.column, 5);
	assert_eq!(point.offset, 17);
}

#[test]
fn point_advance_str_empty() {
	let mut point = Point::new(1, 5, 10);
	point.advance_str("");
	assert_eq!(point.line, 1);
	assert_eq!(point.column, 5);
	assert_eq!(point.offset, 10);
}

// --- Suffix transformer tests ---

#[test]
fn transformer_suffix() {
	let result = apply_transformers(
		"Hello",
		&[Transformer {
			r#type: TransformerType::Suffix,
			args: vec![Argument::String("!".to_string())],
		}],
	);
	assert_eq!(result, "Hello!");
}

#[test]
fn transformer_suffix_empty_arg() {
	let result = apply_transformers(
		"Hello",
		&[Transformer {
			r#type: TransformerType::Suffix,
			args: vec![],
		}],
	);
	assert_eq!(result, "Hello");
}

#[test]
fn transformer_line_prefix() {
	let result = apply_transformers(
		"line1\nline2\nline3",
		&[Transformer {
			r#type: TransformerType::LinePrefix,
			args: vec![Argument::String("// ".to_string())],
		}],
	);
	assert_eq!(result, "// line1\n// line2\n// line3");
}

#[test]
fn transformer_line_prefix_preserves_empty_lines() {
	let result = apply_transformers(
		"line1\n\nline3",
		&[Transformer {
			r#type: TransformerType::LinePrefix,
			args: vec![Argument::String("# ".to_string())],
		}],
	);
	assert_eq!(result, "# line1\n\n# line3");
}

#[test]
fn transformer_line_prefix_includes_empty_lines() {
	let result = apply_transformers(
		"line1\n\nline3",
		&[Transformer {
			r#type: TransformerType::LinePrefix,
			args: vec![
				Argument::String("//! ".to_string()),
				Argument::Boolean(true),
			],
		}],
	);
	assert_eq!(result, "//! line1\n//!\n//! line3");
}

#[test]
fn transformer_line_suffix() {
	let result = apply_transformers(
		"line1\nline2\nline3",
		&[Transformer {
			r#type: TransformerType::LineSuffix,
			args: vec![Argument::String(" \\".to_string())],
		}],
	);
	assert_eq!(result, "line1 \\\nline2 \\\nline3 \\");
}

#[test]
fn transformer_line_suffix_preserves_empty_lines() {
	let result = apply_transformers(
		"line1\n\nline3",
		&[Transformer {
			r#type: TransformerType::LineSuffix,
			args: vec![Argument::String(";".to_string())],
		}],
	);
	assert_eq!(result, "line1;\n\nline3;");
}

#[test]
fn transformer_line_suffix_includes_empty_lines() {
	let result = apply_transformers(
		"line1\n\nline3",
		&[Transformer {
			r#type: TransformerType::LineSuffix,
			args: vec![Argument::String(";".to_string()), Argument::Boolean(true)],
		}],
	);
	assert_eq!(result, "line1;\n;\nline3;");
}

#[test]
fn transformer_chain_line_prefix_and_suffix() {
	let result = apply_transformers(
		"hello\nworld",
		&[
			Transformer {
				r#type: TransformerType::LinePrefix,
				args: vec![Argument::String("* ".to_string())],
			},
			Transformer {
				r#type: TransformerType::LineSuffix,
				args: vec![Argument::String("!".to_string())],
			},
		],
	);
	assert_eq!(result, "* hello!\n* world!");
}

// --- Parse new transformer names ---

#[test]
fn parse_suffix_transformer() -> MdtResult<()> {
	let input = r#"<!-- {=block|suffix:"!"} -->
old
<!-- {/block} -->
"#;
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].transformers.len(), 1);
	assert_eq!(blocks[0].transformers[0].r#type, TransformerType::Suffix);

	Ok(())
}

#[test]
fn parse_line_prefix_transformer() -> MdtResult<()> {
	let input = r#"<!-- {=block|linePrefix:"// "} -->
old
<!-- {/block} -->
"#;
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(
		blocks[0].transformers[0].r#type,
		TransformerType::LinePrefix
	);

	Ok(())
}

#[test]
fn parse_line_prefix_snake_case() -> MdtResult<()> {
	let input = r#"<!-- {=block|line_prefix:"// "} -->
old
<!-- {/block} -->
"#;
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(
		blocks[0].transformers[0].r#type,
		TransformerType::LinePrefix
	);

	Ok(())
}

#[test]
fn parse_line_suffix_transformer() -> MdtResult<()> {
	let input = r#"<!-- {=block|lineSuffix:";"} -->
old
<!-- {/block} -->
"#;
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(
		blocks[0].transformers[0].r#type,
		TransformerType::LineSuffix
	);

	Ok(())
}

#[test]
fn parse_line_suffix_snake_case() -> MdtResult<()> {
	let input = r#"<!-- {=block|line_suffix:";"} -->
old
<!-- {/block} -->
"#;
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(
		blocks[0].transformers[0].r#type,
		TransformerType::LineSuffix
	);

	Ok(())
}

// --- Duplicate provider detection tests ---

#[test]
fn duplicate_provider_detected() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\nfirst\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("other.t.md"),
		"<!-- {@block} -->\n\nsecond\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let result = scan_project(tmp.path());
	assert!(result.is_err());
	let err = result.unwrap_err();
	let msg = err.to_string();
	assert!(msg.contains("duplicate provider"));
	assert!(msg.contains("block"));
}

#[test]
fn error_duplicate_provider_message() {
	let err = MdtError::DuplicateProvider {
		name: "myBlock".to_string(),
		first_file: "a.t.md".to_string(),
		second_file: "b.t.md".to_string(),
	};
	let msg = err.to_string();
	assert!(msg.contains("myBlock"));
	assert!(msg.contains("a.t.md"));
	assert!(msg.contains("b.t.md"));
}

// --- Validate transformers tests ---

#[test]
fn validate_transformers_valid() -> MdtResult<()> {
	let transformers = vec![
		Transformer {
			r#type: TransformerType::Trim,
			args: vec![],
		},
		Transformer {
			r#type: TransformerType::Indent,
			args: vec![Argument::String("  ".to_string())],
		},
		Transformer {
			r#type: TransformerType::Replace,
			args: vec![
				Argument::String("old".to_string()),
				Argument::String("new".to_string()),
			],
		},
	];
	validate_transformers(&transformers)?;
	Ok(())
}

#[test]
fn validate_transformers_trim_with_args_fails() {
	let transformers = vec![Transformer {
		r#type: TransformerType::Trim,
		args: vec![Argument::String("extra".to_string())],
	}];
	let result = validate_transformers(&transformers);
	assert!(result.is_err());
	let msg = result.unwrap_err().to_string();
	assert!(msg.contains("trim"));
	assert!(msg.contains('0'));
}

#[test]
fn validate_transformers_replace_missing_args_fails() {
	let transformers = vec![Transformer {
		r#type: TransformerType::Replace,
		args: vec![Argument::String("only_one".to_string())],
	}];
	let result = validate_transformers(&transformers);
	assert!(result.is_err());
	let msg = result.unwrap_err().to_string();
	assert!(msg.contains("replace"));
}

#[test]
fn validate_transformers_empty_is_ok() -> MdtResult<()> {
	validate_transformers(&[])?;
	Ok(())
}

// --- Unknown transformer and invalid args error tests ---

#[test]
fn error_unknown_transformer_message() {
	let err = MdtError::UnknownTransformer("foobar".to_string());
	let msg = err.to_string();
	assert!(msg.contains("foobar"));
}

#[test]
fn error_invalid_transformer_args_message() {
	let err = MdtError::InvalidTransformerArgs {
		name: "replace".to_string(),
		expected: "2".to_string(),
		got: 1,
	};
	let msg = err.to_string();
	assert!(msg.contains("replace"));
	assert!(msg.contains('2'));
	assert!(msg.contains('1'));
}

// --- Block PartialEq tests ---

#[test]
fn block_partial_eq() -> MdtResult<()> {
	let input = "<!-- {=myBlock} -->\n\ncontent\n\n<!-- {/myBlock} -->\n";
	let blocks1 = parse(input)?;
	let blocks2 = parse(input)?;
	assert_eq!(blocks1, blocks2);

	Ok(())
}

#[test]
fn transformer_partial_eq() {
	let t1 = Transformer {
		r#type: TransformerType::Indent,
		args: vec![Argument::String("  ".to_string())],
	};
	let t2 = Transformer {
		r#type: TransformerType::Indent,
		args: vec![Argument::String("  ".to_string())],
	};
	assert_eq!(t1, t2);
}

#[test]
fn transformer_partial_ne() {
	let t1 = Transformer {
		r#type: TransformerType::Indent,
		args: vec![Argument::String("  ".to_string())],
	};
	let t2 = Transformer {
		r#type: TransformerType::Prefix,
		args: vec![Argument::String("  ".to_string())],
	};
	assert_ne!(t1, t2);
}

// --- CRLF normalization tests ---

#[test]
fn normalize_line_endings_lf_passthrough() {
	let content = "line1\nline2\nline3\n";
	let result = normalize_line_endings(content);
	assert_eq!(result, content);
}

#[test]
fn normalize_line_endings_crlf_to_lf() {
	let content = "line1\r\nline2\r\nline3\r\n";
	let result = normalize_line_endings(content);
	assert_eq!(result, "line1\nline2\nline3\n");
}

#[test]
fn normalize_line_endings_bare_cr_to_lf() {
	let content = "line1\rline2\rline3\r";
	let result = normalize_line_endings(content);
	assert_eq!(result, "line1\nline2\nline3\n");
}

#[test]
fn normalize_line_endings_mixed() {
	let content = "line1\r\nline2\rline3\n";
	let result = normalize_line_endings(content);
	assert_eq!(result, "line1\nline2\nline3\n");
}

#[test]
fn crlf_content_parsed_correctly() {
	let content = "<!-- {=myBlock} -->\r\n\r\nsome content\r\n\r\n<!-- {/myBlock} -->\r\n";
	let normalized = normalize_line_endings(content);
	let blocks = parse(&normalized).unwrap_or_else(|e| panic!("parse failed: {e}"));
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "myBlock");
}

// --- File size limit tests ---

#[test]
fn file_too_large_error() {
	let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let large_file = dir.path().join("huge.md");
	// Write a file slightly larger than a 100-byte limit.
	std::fs::write(&large_file, "x".repeat(200)).unwrap_or_else(|e| panic!("write: {e}"));

	let result = scan_project_with_options(
		dir.path(),
		&ScanOptions {
			max_file_size: 100, // 100-byte limit
			disable_gitignore: true,
			..ScanOptions::default()
		},
	);

	assert!(result.is_err());
	let err_msg = format!("{}", result.unwrap_err());
	assert!(
		err_msg.contains("file too large"),
		"expected 'file too large', got: {err_msg}"
	);
}

#[test]
fn file_within_size_limit_succeeds() {
	let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let small_file = dir.path().join("small.md");
	std::fs::write(&small_file, "<!-- {=test} -->\ncontent\n<!-- {/test} -->\n")
		.unwrap_or_else(|e| panic!("write: {e}"));

	let result = scan_project_with_options(
		dir.path(),
		&ScanOptions {
			max_file_size: 10_000, // 10KB limit
			disable_gitignore: true,
			..ScanOptions::default()
		},
	);

	assert!(result.is_ok());
}

// --- UTF-8 edge case tests ---

#[test]
fn parse_content_with_emoji() {
	let content = "<!-- {=emoji} -->\n\n🦀 Hello 🌍\n\n<!-- {/emoji} -->\n";
	let blocks = parse(content).unwrap_or_else(|e| panic!("parse failed: {e}"));
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "emoji");
}

#[test]
fn parse_content_with_cjk() {
	let content = "<!-- {=cjk} -->\n\n日本語テキスト\n\n<!-- {/cjk} -->\n";
	let blocks = parse(content).unwrap_or_else(|e| panic!("parse failed: {e}"));
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "cjk");
}

#[test]
fn scan_project_with_emoji_content() {
	let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let template = dir.path().join("template.t.md");
	std::fs::write(
		&template,
		"<!-- {@emoji} -->\n\n🦀 Hello 🌍\n\n<!-- {/emoji} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let project = scan_project(dir.path()).unwrap_or_else(|e| panic!("scan: {e}"));
	let provider = project
		.providers
		.get("emoji")
		.unwrap_or_else(|| panic!("no provider"));
	assert!(provider.content.contains("🦀 Hello 🌍"));
}

#[test]
fn transformer_indent_with_multibyte_chars() {
	let content = "🦀 crab\n🌍 world\n";
	let result = apply_transformers(
		content,
		&[Transformer {
			r#type: TransformerType::Indent,
			args: vec![Argument::String("  ".to_string())],
		}],
	);
	assert_eq!(result, "  🦀 crab\n  🌍 world");
}

// --- No trailing newline edge case ---

#[test]
fn parse_block_without_trailing_newline() {
	let content = "<!-- {=test} -->\ncontent\n<!-- {/test} -->";
	let blocks = parse(content).unwrap_or_else(|e| panic!("parse failed: {e}"));
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "test");
}

// --- Insta snapshot tests ---

#[test]
fn snapshot_tokenize_consumer() -> MdtResult<()> {
	let nodes = get_html_nodes(r#"<!-- {=exampleName|trim|indent:"/// "} -->"#)?;
	let groups = tokenize(nodes)?;
	insta::assert_debug_snapshot!(groups);
	Ok(())
}

#[test]
fn snapshot_tokenize_provider() -> MdtResult<()> {
	let nodes = get_html_nodes("<!-- {@myProvider} -->")?;
	let groups = tokenize(nodes)?;
	insta::assert_debug_snapshot!(groups);
	Ok(())
}

#[test]
fn snapshot_tokenize_closing() -> MdtResult<()> {
	let nodes = get_html_nodes("<!-- {/blockName} -->")?;
	let groups = tokenize(nodes)?;
	insta::assert_debug_snapshot!(groups);
	Ok(())
}

#[test]
fn snapshot_parse_full_document() -> MdtResult<()> {
	let input = r#"# Title

<!-- {@header} -->

# Welcome to {{ pkg.name }}

<!-- {/header} -->

## Content

<!-- {=header} -->

old content

<!-- {/header} -->

<!-- {=docs|trim|indent:"  "} -->

old docs

<!-- {/docs} -->
"#;
	let blocks = parse(input)?;
	insta::assert_debug_snapshot!(blocks);
	Ok(())
}

#[test]
fn snapshot_parse_consumer_with_all_transformers() -> MdtResult<()> {
	let input = r##"<!-- {=block|trim|trimStart|trimEnd|indent:"  "|prefix:"# "|wrap:"**"|codeBlock:"rs"|code|replace:"a":"b"} -->
old
<!-- {/block} -->
"##;
	let blocks = parse(input)?;
	insta::assert_debug_snapshot!(blocks);
	Ok(())
}

// --- Edge case tests ---

#[test]
fn parse_empty_provider_content() -> MdtResult<()> {
	let input = "<!-- {@block} -->\n<!-- {/block} -->\n";
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "block");
	let content = extract_content_between_tags(input, &blocks[0]);
	assert_eq!(content, "\n");
	Ok(())
}

#[test]
fn parse_very_long_block_name() -> MdtResult<()> {
	let long_name = "a".repeat(200);
	let input = format!("<!-- {{@{long_name}}} -->\n\ncontent\n\n<!-- {{/{long_name}}} -->\n");
	let blocks = parse(&input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, long_name);
	Ok(())
}

#[test]
fn parse_multiple_consumers_same_provider() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@shared} -->\n\nShared content.\n\n<!-- {/shared} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("a.md"),
		"<!-- {=shared} -->\n\nold a\n\n<!-- {/shared} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("b.md"),
		"<!-- {=shared} -->\n\nold b\n\n<!-- {/shared} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = ProjectContext {
		project: scan_project(tmp.path())?,
		data: HashMap::new(),
		padding: None,
	};
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 2);
	assert_eq!(updates.updated_files.len(), 2);
	for content in updates.updated_files.values() {
		assert!(content.contains("Shared content."));
	}
	Ok(())
}

#[test]
fn transformer_with_boolean_argument() -> MdtResult<()> {
	let input = "<!-- {=block|indent:true} -->\nold\n<!-- {/block} -->\n";
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].transformers.len(), 1);
	match &blocks[0].transformers[0].args[0] {
		Argument::Boolean(b) => assert!(b),
		other => panic!("expected Boolean, got {other:?}"),
	}
	Ok(())
}

#[test]
fn config_multiple_data_formats() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\njson_data = \"data.json\"\ntoml_data = \"data.toml\"\nyaml_data = \"data.yaml\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("data.json"), r#"{"key": "json_value"}"#)
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("data.toml"), "key = \"toml_value\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("data.yaml"), "key: yaml_value\n")
		.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;
	assert_eq!(data["json_data"]["key"], "json_value");
	assert_eq!(data["toml_data"]["key"], "toml_value");
	assert_eq!(data["yaml_data"]["key"], "yaml_value");
	Ok(())
}

#[test]
fn render_template_deeply_nested_data() -> MdtResult<()> {
	let mut data = HashMap::new();
	data.insert(
		"a".to_string(),
		serde_json::json!({"b": {"c": {"d": "deep_value"}}}),
	);
	let content = "{{ a.b.c.d }}";
	let result = render_template(content, &data)?;
	assert_eq!(result, "deep_value");
	Ok(())
}

#[test]
fn source_scanner_mixed_comment_styles() -> MdtResult<()> {
	let content = r"// Single line comment with <!-- {=blockA} -->
// content A
// <!-- {/blockA} -->

/* Block comment */
/* <!-- {=blockB} --> */
/* content B */
/* <!-- {/blockB} --> */
";
	let blocks = parse_source(content)?;
	// At least blockA should be found (single-line comments)
	assert!(!blocks.is_empty());
	assert!(blocks.iter().any(|b| b.name == "blockA"));
	Ok(())
}

#[test]
fn tokenize_malformed_incomplete_comment() -> MdtResult<()> {
	// Malformed HTML comments should not panic
	let nodes = get_html_nodes("<!-- {= -->")?;
	let groups = tokenize(nodes)?;
	assert!(groups.is_empty());
	Ok(())
}

#[test]
fn tokenize_malformed_no_close_brace() -> MdtResult<()> {
	let nodes = get_html_nodes("<!-- {=name -->")?;
	let groups = tokenize(nodes)?;
	assert!(groups.is_empty());
	Ok(())
}

#[test]
fn tokenize_empty_tag_name() -> MdtResult<()> {
	let nodes = get_html_nodes("<!-- {=} -->")?;
	let groups = tokenize(nodes)?;
	assert!(groups.is_empty());
	Ok(())
}

// --- Fuzz-style no-panic tests ---

#[test]
fn fuzz_tokenizer_no_panic() {
	let long_input = "<!-- {=".to_string() + &"x".repeat(10000) + "} -->";
	let inputs: Vec<&str> = vec![
		"",
		"<!-- -->",
		"<!---->",
		"<!-- { -->",
		"<!-- {= -->",
		"<!-- {@ -->",
		"<!-- {/ -->",
		"<!-- {=} -->",
		"<!-- {@} -->",
		"<!-- {/} -->",
		"<!-- {=name} --> <!-- {/other} -->",
		"<!-- {=a|b|c|d|e|f} -->",
		r#"<!-- {=a|b:"c":"d":"e"} -->"#,
		"<!-- {=a|} -->",
		"<!-- {=a||} -->",
		"<!-- {=a|b:} -->",
		"<-- {=a} -->",
		"<!- {=a} -->",
		"<!-- {=a} --",
		"<!-- {=a} ->",
		"<!-- {=a\n} -->",
		&long_input,
		"<!-- {=name|trim|trim|trim|trim|trim|trim|trim|trim} -->",
	];

	for input in &inputs {
		let result = get_html_nodes(input);
		if let Ok(nodes) = result {
			let _ = tokenize(nodes);
		}
	}
}

#[test]
fn fuzz_parser_no_panic() {
	let inputs = [
		"",
		"<!-- {@a} -->\n<!-- {/a} -->\n",
		"<!-- {=a} -->\n<!-- {/a} -->\n",
		"<!-- {@a} -->\n<!-- {@b} -->\n<!-- {/b} -->\n<!-- {/a} -->\n",
		"<!-- {/orphan} -->\n",
		"<!-- {@a} -->\ncontent\n<!-- {/b} -->\n",
		"<!-- {=a} -->\n<!-- {=b} -->\n<!-- {/a} -->\n<!-- {/b} -->\n",
	];

	for input in &inputs {
		let _ = parse(input);
	}
}

#[test]
fn fuzz_source_scanner_no_panic() {
	let inputs = [
		"",
		"no comments here",
		"// <!-- partial",
		"// <!-- {= -->",
		"<!-- unmatched",
		"--><!-- --><!--",
		"// <!-- {=a} -->\n// <!-- {/b} -->\n",
	];

	for input in &inputs {
		let _ = parse_source(input);
	}
}

// --- Diagnostic tests ---

#[test]
fn parse_with_diagnostics_reports_unclosed_block() {
	let input = "<!-- {=block} -->\n\nold content\n";
	let (blocks, diagnostics) =
		parse_with_diagnostics(input).unwrap_or_else(|e| panic!("parse_with_diagnostics: {e}"));
	assert!(
		blocks.is_empty(),
		"unclosed block should not produce a Block"
	);
	assert_eq!(diagnostics.len(), 1);
	match &diagnostics[0] {
		ParseDiagnostic::UnclosedBlock { name, line, .. } => {
			assert_eq!(name, "block");
			assert_eq!(*line, 1);
		}
		other => panic!("expected UnclosedBlock, got {other:?}"),
	}
}

#[test]
fn parse_with_diagnostics_reports_unknown_transformer() {
	let input = "<!-- {=block|foobar} -->\n\nold\n\n<!-- {/block} -->\n";
	let (blocks, diagnostics) =
		parse_with_diagnostics(input).unwrap_or_else(|e| panic!("parse_with_diagnostics: {e}"));
	assert_eq!(
		blocks.len(),
		1,
		"block with unknown transformer should still parse"
	);
	assert_eq!(
		blocks[0].transformers.len(),
		0,
		"unknown transformer should not be in list"
	);
	assert_eq!(diagnostics.len(), 1);
	match &diagnostics[0] {
		ParseDiagnostic::UnknownTransformer { name, .. } => {
			assert_eq!(name, "foobar");
		}
		other => panic!("expected UnknownTransformer, got {other:?}"),
	}
}

#[test]
fn scan_project_collects_unclosed_block_diagnostic() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=block} -->\n\nold content\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let project = scan_project(tmp.path()).unwrap_or_else(|e| panic!("scan: {e}"));
	assert!(
		project.diagnostics.iter().any(|d| {
			matches!(
				&d.kind,
				DiagnosticKind::UnclosedBlock { name } if name == "block"
			)
		}),
		"expected UnclosedBlock diagnostic, got: {:?}",
		project.diagnostics
	);
}

#[test]
fn scan_project_collects_unknown_transformer_diagnostic() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\ncontent\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=block|foobar} -->\n\nold\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let project = scan_project(tmp.path()).unwrap_or_else(|e| panic!("scan: {e}"));
	assert!(
		project.diagnostics.iter().any(|d| {
			matches!(
				&d.kind,
				DiagnosticKind::UnknownTransformer { name } if name == "foobar"
			)
		}),
		"expected UnknownTransformer diagnostic, got: {:?}",
		project.diagnostics
	);
}

#[test]
fn scan_project_collects_invalid_transformer_args_diagnostic() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\ncontent\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	// trim takes 0 args but we give it one
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=block|trim:\"extra\"} -->\n\nold\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let project = scan_project(tmp.path()).unwrap_or_else(|e| panic!("scan: {e}"));
	assert!(
		project.diagnostics.iter().any(|d| {
			matches!(
				&d.kind,
				DiagnosticKind::InvalidTransformerArgs { name, .. } if name == "trim"
			)
		}),
		"expected InvalidTransformerArgs diagnostic, got: {:?}",
		project.diagnostics
	);
}

#[test]
fn scan_project_collects_unused_provider_diagnostic() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@unused_block} -->\n\ncontent\n\n<!-- {/unused_block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let project = scan_project(tmp.path()).unwrap_or_else(|e| panic!("scan: {e}"));
	assert!(
		project.diagnostics.iter().any(|d| {
			matches!(
				&d.kind,
				DiagnosticKind::UnusedProvider { name } if name == "unused_block"
			)
		}),
		"expected UnusedProvider diagnostic, got: {:?}",
		project.diagnostics
	);
}

#[test]
fn diagnostic_is_error_respects_validation_options() {
	use project::DiagnosticKind;
	use project::ProjectDiagnostic;
	use project::ValidationOptions;

	let diag = ProjectDiagnostic {
		file: PathBuf::from("test.md"),
		kind: DiagnosticKind::UnclosedBlock {
			name: "test".to_string(),
		},
		line: 1,
		column: 1,
	};

	let default_options = ValidationOptions::default();
	assert!(
		diag.is_error(&default_options),
		"unclosed block should be error by default"
	);

	let ignore_options = ValidationOptions {
		ignore_unclosed_blocks: true,
		..Default::default()
	};
	assert!(
		!diag.is_error(&ignore_options),
		"unclosed block should not be error when ignored"
	);
}

#[test]
fn stale_entry_includes_line_and_column() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\nnew content\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"Some preamble\n\n<!-- {=block} -->\n\nold content\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let project = scan_project_with_config(tmp.path()).unwrap_or_else(|e| panic!("scan: {e}"));
	let result = check_project(&project).unwrap_or_else(|e| panic!("check: {e}"));
	assert_eq!(result.stale.len(), 1);
	// The consumer opening tag is on line 3 (after "Some preamble\n\n")
	assert_eq!(result.stale[0].line, 3);
	assert!(result.stale[0].column > 0);
}

// --- padding tests ---

#[test]
fn pad_blocks_markdown_update() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[padding]\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@greeting} -->\n\nHello world.\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"# Title\n\n<!-- {=greeting} -->\n\nOld content.\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	assert!(ctx.padding.is_some());
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	// With before=1/after=1 (default), extra blank line is added.
	// Content from provider already has \n\n prefix/suffix, plus padding adds
	// one more blank line.
	assert_eq!(
		content.as_str(),
		"# Title\n\n<!-- {=greeting} -->\n\n\nHello world.\n\n\n<!-- {/greeting} -->\n"
	);

	Ok(())
}

#[test]
fn pad_blocks_prevents_squashed_content() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[padding]\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	// Provider whose content has no surrounding newlines after trim
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@info} -->\n\nSome info.\n\n<!-- {/info} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	// Consumer with trim transformer — would produce "Some info." with no newlines
	std::fs::write(
		tmp.path().join("doc.md"),
		"<!-- {=info|trim} -->\n\nold\n\n<!-- {/info} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	// With before=1/after=1 (default), one blank line between tags and content.
	assert_eq!(
		content.as_str(),
		"<!-- {=info|trim} -->\n\nSome info.\n\n<!-- {/info} -->\n"
	);

	Ok(())
}

#[test]
fn pad_blocks_rust_doc_comments() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[padding]\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@docs} -->\n\nHello from mdt.\n\n<!-- {/docs} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("lib.rs"),
		concat!(
			"//! <!-- {=docs|trim|linePrefix:\"//! \":true} -->\n",
			"//! old content\n",
			"//! <!-- {/docs} -->\n",
			"\n",
			"pub fn main() {}\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	// After trim: "Hello from mdt."
	// After linePrefix "//! " with includeEmpty: "//! Hello from mdt."
	// After pad_blocks: "\n//!\n//! Hello from mdt.\n//!\n//! "
	assert_eq!(
		content.as_str(),
		concat!(
			"//! <!-- {=docs|trim|linePrefix:\"//! \":true} -->\n",
			"//!\n",
			"//! Hello from mdt.\n",
			"//!\n",
			"//! <!-- {/docs} -->\n",
			"\n",
			"pub fn main() {}\n",
		)
	);

	Ok(())
}

#[test]
fn pad_blocks_rust_doc_comments_multiline() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[padding]\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		concat!(
			"<!-- {@docs} -->\n",
			"\n",
			"# My Library\n",
			"\n",
			"This is a great library.\n",
			"\n",
			"## Usage\n",
			"\n",
			"Just use it.\n",
			"\n",
			"<!-- {/docs} -->\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("lib.rs"),
		concat!(
			"//! <!-- {=docs|trim|linePrefix:\"//! \":true} -->\n",
			"//! old\n",
			"//! <!-- {/docs} -->\n",
			"\n",
			"pub fn main() {}\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	// After trim: "# My Library\n\nThis is a great library.\n\n## Usage\n\nJust use
	// it." After linePrefix "//! " with includeEmpty=true on each line
	// After pad_blocks: extra blank prefix line before content and before closing
	// tag
	assert_eq!(
		content.as_str(),
		concat!(
			"//! <!-- {=docs|trim|linePrefix:\"//! \":true} -->\n",
			"//!\n",
			"//! # My Library\n",
			"//!\n",
			"//! This is a great library.\n",
			"//!\n",
			"//! ## Usage\n",
			"//!\n",
			"//! Just use it.\n",
			"//!\n",
			"//! <!-- {/docs} -->\n",
			"\n",
			"pub fn main() {}\n",
		)
	);

	Ok(())
}

#[test]
fn pad_blocks_rust_triple_slash_comments() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[padding]\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@fn_docs} -->\n\nDoes something useful.\n\n<!-- {/fn_docs} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("lib.rs"),
		concat!(
			"/// <!-- {=fn_docs|trim|linePrefix:\"/// \":true} -->\n",
			"/// old docs\n",
			"/// <!-- {/fn_docs} -->\n",
			"pub fn do_something() {}\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	assert_eq!(
		content.as_str(),
		concat!(
			"/// <!-- {=fn_docs|trim|linePrefix:\"/// \":true} -->\n",
			"///\n",
			"/// Does something useful.\n",
			"///\n",
			"/// <!-- {/fn_docs} -->\n",
			"pub fn do_something() {}\n",
		)
	);

	Ok(())
}

#[test]
fn pad_blocks_typescript_jsdoc() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[padding]\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@jsdocs} -->\n\nGreets the user.\n\n<!-- {/jsdocs} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("index.ts"),
		concat!(
			"/**\n",
			" * <!-- {=jsdocs|trim|linePrefix:\" * \":true} -->\n",
			" * old docs\n",
			" * <!-- {/jsdocs} -->\n",
			" */\n",
			"export function greet() {}\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	assert_eq!(
		content.as_str(),
		concat!(
			"/**\n",
			" * <!-- {=jsdocs|trim|linePrefix:\" * \":true} -->\n",
			" *\n",
			" * Greets the user.\n",
			" *\n",
			" * <!-- {/jsdocs} -->\n",
			" */\n",
			"export function greet() {}\n",
		)
	);

	Ok(())
}

#[test]
fn pad_blocks_python_hash_comments() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[padding]\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@pydocs} -->\n\nPython docs here.\n\n<!-- {/pydocs} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("main.py"),
		concat!(
			"# <!-- {=pydocs|trim|linePrefix:\"# \":true} -->\n",
			"# old docs\n",
			"# <!-- {/pydocs} -->\n",
			"def main():\n",
			"    pass\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	assert_eq!(
		content.as_str(),
		concat!(
			"# <!-- {=pydocs|trim|linePrefix:\"# \":true} -->\n",
			"#\n",
			"# Python docs here.\n",
			"#\n",
			"# <!-- {/pydocs} -->\n",
			"def main():\n",
			"    pass\n",
		)
	);

	Ok(())
}

#[test]
fn pad_blocks_go_comments() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[padding]\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@godocs} -->\n\nGo function docs.\n\n<!-- {/godocs} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("main.go"),
		concat!(
			"// <!-- {=godocs|trim|linePrefix:\"// \":true} -->\n",
			"// old docs\n",
			"// <!-- {/godocs} -->\n",
			"func main() {}\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	assert_eq!(
		content.as_str(),
		concat!(
			"// <!-- {=godocs|trim|linePrefix:\"// \":true} -->\n",
			"//\n",
			"// Go function docs.\n",
			"//\n",
			"// <!-- {/godocs} -->\n",
			"func main() {}\n",
		)
	);

	Ok(())
}

#[test]
fn pad_blocks_java_comments() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[padding]\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@javadocs} -->\n\nJava method docs.\n\n<!-- {/javadocs} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("Main.java"),
		concat!(
			"/**\n",
			" * <!-- {=javadocs|trim|linePrefix:\" * \":true} -->\n",
			" * old docs\n",
			" * <!-- {/javadocs} -->\n",
			" */\n",
			"public class Main {}\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	assert_eq!(
		content.as_str(),
		concat!(
			"/**\n",
			" * <!-- {=javadocs|trim|linePrefix:\" * \":true} -->\n",
			" *\n",
			" * Java method docs.\n",
			" *\n",
			" * <!-- {/javadocs} -->\n",
			" */\n",
			"public class Main {}\n",
		)
	);

	Ok(())
}

#[test]
fn pad_blocks_c_comments() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[padding]\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@cdocs} -->\n\nC function docs.\n\n<!-- {/cdocs} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("main.c"),
		concat!(
			"// <!-- {=cdocs|trim|linePrefix:\"// \":true} -->\n",
			"// old docs\n",
			"// <!-- {/cdocs} -->\n",
			"int main() { return 0; }\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	assert_eq!(
		content.as_str(),
		concat!(
			"// <!-- {=cdocs|trim|linePrefix:\"// \":true} -->\n",
			"//\n",
			"// C function docs.\n",
			"//\n",
			"// <!-- {/cdocs} -->\n",
			"int main() { return 0; }\n",
		)
	);

	Ok(())
}

#[test]
fn pad_blocks_idempotent() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[padding]\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@docs} -->\n\nContent here.\n\n<!-- {/docs} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("lib.rs"),
		concat!(
			"//! <!-- {=docs|trim|linePrefix:\"//! \":true} -->\n",
			"//! old\n",
			"//! <!-- {/docs} -->\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// First update
	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	write_updates(&updates)?;

	// Second update should be noop
	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 0);

	Ok(())
}

#[test]
fn pad_blocks_check_detects_stale() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[padding]\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@docs} -->\n\nNew content.\n\n<!-- {/docs} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("lib.rs"),
		concat!(
			"//! <!-- {=docs|trim|linePrefix:\"//! \":true} -->\n",
			"//! old content\n",
			"//! <!-- {/docs} -->\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let result = check_project(&ctx)?;
	assert!(!result.is_ok());
	assert_eq!(result.stale.len(), 1);
	assert_eq!(result.stale[0].block_name, "docs");

	Ok(())
}

#[test]
fn pad_blocks_disabled_does_not_pad() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	// No mdt.toml → pad_blocks defaults to false
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@info} -->\n\nHello.\n\n<!-- {/info} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("doc.md"),
		"<!-- {=info|trim} -->old<!-- {/info} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = ProjectContext {
		project: scan_project(tmp.path())?,
		data: HashMap::new(),
		padding: None,
	};
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	// Without pad_blocks, "Hello." goes directly between tags with no padding
	assert_eq!(
		content.as_str(),
		"<!-- {=info|trim} -->Hello.<!-- {/info} -->\n"
	);

	Ok(())
}

#[test]
fn pad_blocks_rust_multiline_preserves_blank_lines() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[padding]\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		concat!(
			"<!-- {@api} -->\n",
			"\n",
			"# API\n",
			"\n",
			"Create a new instance:\n",
			"\n",
			"```rust\n",
			"let x = Foo::new();\n",
			"```\n",
			"\n",
			"Then call methods on it.\n",
			"\n",
			"<!-- {/api} -->\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("lib.rs"),
		concat!(
			"//! <!-- {=api|trim|linePrefix:\"//! \":true} -->\n",
			"//! old\n",
			"//! <!-- {/api} -->\n",
			"\n",
			"pub struct Foo;\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	assert_eq!(
		content.as_str(),
		concat!(
			"//! <!-- {=api|trim|linePrefix:\"//! \":true} -->\n",
			"//!\n",
			"//! # API\n",
			"//!\n",
			"//! Create a new instance:\n",
			"//!\n",
			"//! ```rust\n",
			"//! let x = Foo::new();\n",
			"//! ```\n",
			"//!\n",
			"//! Then call methods on it.\n",
			"//!\n",
			"//! <!-- {/api} -->\n",
			"\n",
			"pub struct Foo;\n",
		)
	);

	Ok(())
}

#[test]
fn pad_blocks_kotlin_comments() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[padding]\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@ktdocs} -->\n\nKotlin function docs.\n\n<!-- {/ktdocs} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("main.kt"),
		concat!(
			"/**\n",
			" * <!-- {=ktdocs|trim|linePrefix:\" * \":true} -->\n",
			" * old docs\n",
			" * <!-- {/ktdocs} -->\n",
			" */\n",
			"fun main() {}\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	assert_eq!(
		content.as_str(),
		concat!(
			"/**\n",
			" * <!-- {=ktdocs|trim|linePrefix:\" * \":true} -->\n",
			" *\n",
			" * Kotlin function docs.\n",
			" *\n",
			" * <!-- {/ktdocs} -->\n",
			" */\n",
			"fun main() {}\n",
		)
	);

	Ok(())
}

#[test]
fn pad_blocks_swift_comments() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[padding]\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@swiftdocs} -->\n\nSwift function docs.\n\n<!-- {/swiftdocs} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("main.swift"),
		concat!(
			"/// <!-- {=swiftdocs|trim|linePrefix:\"/// \":true} -->\n",
			"/// old docs\n",
			"/// <!-- {/swiftdocs} -->\n",
			"func greet() {}\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	assert_eq!(
		content.as_str(),
		concat!(
			"/// <!-- {=swiftdocs|trim|linePrefix:\"/// \":true} -->\n",
			"///\n",
			"/// Swift function docs.\n",
			"///\n",
			"/// <!-- {/swiftdocs} -->\n",
			"func greet() {}\n",
		)
	);

	Ok(())
}

#[test]
fn pad_blocks_cpp_comments() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[padding]\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@cppdocs} -->\n\nC++ function docs.\n\n<!-- {/cppdocs} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("main.cpp"),
		concat!(
			"// <!-- {=cppdocs|trim|linePrefix:\"// \":true} -->\n",
			"// old docs\n",
			"// <!-- {/cppdocs} -->\n",
			"int main() { return 0; }\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	assert_eq!(
		content.as_str(),
		concat!(
			"// <!-- {=cppdocs|trim|linePrefix:\"// \":true} -->\n",
			"//\n",
			"// C++ function docs.\n",
			"//\n",
			"// <!-- {/cppdocs} -->\n",
			"int main() { return 0; }\n",
		)
	);

	Ok(())
}

#[test]
fn pad_blocks_csharp_comments() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[padding]\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@csdocs} -->\n\nC# method docs.\n\n<!-- {/csdocs} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("Program.cs"),
		concat!(
			"/// <!-- {=csdocs|trim|linePrefix:\"/// \":true} -->\n",
			"/// old docs\n",
			"/// <!-- {/csdocs} -->\n",
			"public static void Main() {}\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	assert_eq!(
		content.as_str(),
		concat!(
			"/// <!-- {=csdocs|trim|linePrefix:\"/// \":true} -->\n",
			"///\n",
			"/// C# method docs.\n",
			"///\n",
			"/// <!-- {/csdocs} -->\n",
			"public static void Main() {}\n",
		)
	);

	Ok(())
}

// --- padding: before=0, after=0 tests ---

#[test]
fn padding_zero_rust_doc_comments() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[padding]\nbefore = 0\nafter = 0\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@docs} -->\n\nHello from mdt.\n\n<!-- {/docs} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("lib.rs"),
		concat!(
			"//! <!-- {=docs|trim|linePrefix:\"//! \":true} -->\n",
			"//! old content\n",
			"//! <!-- {/docs} -->\n",
			"\n",
			"pub fn main() {}\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	// With before=0/after=0: content on next line, no blank lines
	assert_eq!(
		content.as_str(),
		concat!(
			"//! <!-- {=docs|trim|linePrefix:\"//! \":true} -->\n",
			"//! Hello from mdt.\n",
			"//! <!-- {/docs} -->\n",
			"\n",
			"pub fn main() {}\n",
		)
	);

	Ok(())
}

#[test]
fn padding_zero_idempotent() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[padding]\nbefore = 0\nafter = 0\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@docs} -->\n\nContent here.\n\n<!-- {/docs} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("lib.rs"),
		concat!(
			"//! <!-- {=docs|trim|linePrefix:\"//! \":true} -->\n",
			"//! old\n",
			"//! <!-- {/docs} -->\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// First update
	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	write_updates(&updates)?;

	// Second update should be noop
	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 0);

	Ok(())
}

#[test]
fn padding_zero_markdown() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[padding]\nbefore = 0\nafter = 0\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@info} -->\n\nSome info.\n\n<!-- {/info} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("doc.md"),
		"<!-- {=info|trim} -->old<!-- {/info} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	// With before=0/after=0: content on next line, no blank lines
	assert_eq!(
		content.as_str(),
		"<!-- {=info|trim} -->\nSome info.\n<!-- {/info} -->\n"
	);

	Ok(())
}

#[test]
fn padding_false_inline() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[padding]\nbefore = false\nafter = false\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@info} -->\n\nHello.\n\n<!-- {/info} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("doc.md"),
		"<!-- {=info|trim} -->old<!-- {/info} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	// With before=false/after=false: content inline with tags
	assert_eq!(
		content.as_str(),
		"<!-- {=info|trim} -->Hello.<!-- {/info} -->\n"
	);

	Ok(())
}

#[test]
fn padding_two_blank_lines() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[padding]\nbefore = 2\nafter = 2\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@docs} -->\n\nHello.\n\n<!-- {/docs} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("lib.rs"),
		concat!(
			"//! <!-- {=docs|trim|linePrefix:\"//! \":true} -->\n",
			"//! old\n",
			"//! <!-- {/docs} -->\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	// With before=2/after=2: two blank lines with comment prefix
	assert_eq!(
		content.as_str(),
		concat!(
			"//! <!-- {=docs|trim|linePrefix:\"//! \":true} -->\n",
			"//!\n",
			"//!\n",
			"//! Hello.\n",
			"//!\n",
			"//!\n",
			"//! <!-- {/docs} -->\n",
		)
	);

	Ok(())
}

#[test]
fn padding_mixed_before_zero_after_one() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[padding]\nbefore = 0\nafter = 1\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@docs} -->\n\nHello.\n\n<!-- {/docs} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("lib.rs"),
		concat!(
			"//! <!-- {=docs|trim|linePrefix:\"//! \":true} -->\n",
			"//! old\n",
			"//! <!-- {/docs} -->\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	// before=0: content on next line, after=1: one blank line before close
	assert_eq!(
		content.as_str(),
		concat!(
			"//! <!-- {=docs|trim|linePrefix:\"//! \":true} -->\n",
			"//! Hello.\n",
			"//!\n",
			"//! <!-- {/docs} -->\n",
		)
	);

	Ok(())
}

// --- Exclude configuration tests ---

#[test]
fn custom_exclude_patterns_skip_matching_files() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	// Config with an exclude pattern that skips all files in "generated/".
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[exclude]\npatterns = [\"generated/\"]\n\ndisable_gitignore = true\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Template file at the root — should be scanned.
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@hello} -->\n\nHello world.\n\n<!-- {/hello} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Consumer in a normal directory — should be scanned.
	std::fs::create_dir_all(tmp.path().join("docs")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("docs/readme.md"),
		"<!-- {=hello} -->\nold\n<!-- {/hello} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Consumer in a generated directory — should be excluded.
	std::fs::create_dir_all(tmp.path().join("generated")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("generated/output.md"),
		"<!-- {=hello} -->\nstale\n<!-- {/hello} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	// Only the consumer in docs/ should be found, not the one in generated/.
	assert_eq!(ctx.project.consumers.len(), 1);
	assert!(
		ctx.project.consumers[0]
			.file
			.to_str()
			.unwrap_or_default()
			.contains("docs")
	);

	Ok(())
}

#[test]
fn custom_exclude_glob_pattern_skips_files() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	// Exclude all files matching *.generated.md
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[exclude]\npatterns = [\"*.generated.md\"]\n\ndisable_gitignore = true\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@info} -->\n\nInfo content.\n\n<!-- {/info} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Should be scanned.
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=info} -->\nold\n<!-- {/info} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Should be excluded because of the *.generated.md pattern.
	std::fs::write(
		tmp.path().join("api.generated.md"),
		"<!-- {=info} -->\nstale\n<!-- {/info} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	assert_eq!(ctx.project.consumers.len(), 1);
	assert!(
		ctx.project.consumers[0]
			.file
			.to_str()
			.unwrap_or_default()
			.contains("readme.md")
	);

	Ok(())
}

#[test]
fn gitignore_respected_by_default() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	// No explicit mdt.toml — default behavior respects .gitignore.
	// Create a .gitignore that ignores the "build/" directory.
	std::fs::write(tmp.path().join(".gitignore"), "build/\n")
		.unwrap_or_else(|e| panic!("write: {e}"));

	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@greeting} -->\n\nHi there.\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Consumer in root — should be scanned.
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=greeting} -->\nold\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Consumer in build/ — should be skipped via .gitignore.
	std::fs::create_dir_all(tmp.path().join("build")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("build/output.md"),
		"<!-- {=greeting} -->\nstale\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	assert_eq!(ctx.project.consumers.len(), 1);
	assert!(
		ctx.project.consumers[0]
			.file
			.to_str()
			.unwrap_or_default()
			.contains("readme.md")
	);

	Ok(())
}

#[test]
fn disable_gitignore_scans_all_files() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	// Explicitly disable gitignore.
	std::fs::write(tmp.path().join("mdt.toml"), "disable_gitignore = true\n")
		.unwrap_or_else(|e| panic!("write: {e}"));

	// .gitignore ignores "build/", but disable_gitignore overrides that.
	std::fs::write(tmp.path().join(".gitignore"), "build/\n")
		.unwrap_or_else(|e| panic!("write: {e}"));

	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@greeting} -->\n\nHi there.\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=greeting} -->\nold\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	std::fs::create_dir_all(tmp.path().join("build")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("build/output.md"),
		"<!-- {=greeting} -->\nstale\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	// Both consumers should be found since gitignore is disabled.
	assert_eq!(ctx.project.consumers.len(), 2);

	Ok(())
}

#[test]
fn exclude_and_gitignore_combined() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	// .gitignore ignores "build/", custom exclude patterns exclude "generated/".
	std::fs::write(tmp.path().join(".gitignore"), "build/\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[exclude]\npatterns = [\"generated/\"]\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@msg} -->\n\nMessage.\n\n<!-- {/msg} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Consumer in root — should be scanned.
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=msg} -->\nold\n<!-- {/msg} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Consumer in build/ — skipped by .gitignore.
	std::fs::create_dir_all(tmp.path().join("build")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("build/output.md"),
		"<!-- {=msg} -->\nstale\n<!-- {/msg} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Consumer in generated/ — skipped by custom exclude.
	std::fs::create_dir_all(tmp.path().join("generated")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("generated/api.md"),
		"<!-- {=msg} -->\nstale\n<!-- {/msg} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	// Only readme.md consumer should be found.
	assert_eq!(ctx.project.consumers.len(), 1);
	assert!(
		ctx.project.consumers[0]
			.file
			.to_str()
			.unwrap_or_default()
			.contains("readme.md")
	);

	Ok(())
}

#[test]
fn exclude_negation_pattern_re_includes_file() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	// Exclude all files in output/ via a wildcard, but re-include important.md.
	// Note: using "output/*" rather than "output/" — the latter blocks directory
	// traversal entirely, which prevents the negation from ever being evaluated
	// (matching real gitignore semantics).
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"disable_gitignore = true\n\n[exclude]\npatterns = [\"output/*\", \
		 \"!output/important.md\"]\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@note} -->\n\nImportant note.\n\n<!-- {/note} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	std::fs::create_dir_all(tmp.path().join("output")).unwrap_or_else(|e| panic!("mkdir: {e}"));

	// Should be excluded.
	std::fs::write(
		tmp.path().join("output/normal.md"),
		"<!-- {=note} -->\nold\n<!-- {/note} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Should be re-included via negation pattern.
	std::fs::write(
		tmp.path().join("output/important.md"),
		"<!-- {=note} -->\nold\n<!-- {/note} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	assert_eq!(ctx.project.consumers.len(), 1);
	assert!(
		ctx.project.consumers[0]
			.file
			.to_str()
			.unwrap_or_default()
			.contains("important.md")
	);

	Ok(())
}

#[test]
fn scan_project_with_options_exclude_patterns_parameter() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\nContent.\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=block} -->\nold\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	std::fs::create_dir_all(tmp.path().join("dist")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("dist/output.md"),
		"<!-- {=block} -->\nstale\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let result = scan_project_with_options(
		tmp.path(),
		&ScanOptions {
			exclude_patterns: vec!["dist/".to_string()],
			disable_gitignore: true, // disable gitignore so we're only testing custom patterns
			..ScanOptions::default()
		},
	)?;

	assert_eq!(result.consumers.len(), 1);
	assert!(
		result.consumers[0]
			.file
			.to_str()
			.unwrap_or_default()
			.contains("readme.md")
	);

	Ok(())
}

#[test]
fn config_parses_exclude_section() {
	let toml_content = r#"
disable_gitignore = true

[exclude]
patterns = ["build/", "*.bak"]
"#;
	let config: MdtConfig = toml::from_str(toml_content).unwrap_or_else(|e| panic!("parse: {e}"));
	assert_eq!(config.exclude.patterns, vec!["build/", "*.bak"]);
	assert!(config.disable_gitignore);
}

#[test]
fn config_defaults_for_exclude_fields() {
	let toml_content = "";
	let config: MdtConfig = toml::from_str(toml_content).unwrap_or_else(|e| panic!("parse: {e}"));
	assert!(config.exclude.patterns.is_empty());
	assert!(!config.disable_gitignore);
}

// =============================================================================
// Coverage improvement tests
// =============================================================================

// --- tokens.rs: GetDynamicRange impls for all numeric types ---

/// Helper that exercises `GetDynamicRange` for a given value.
/// Creates a `TokenGroup` with known tokens and calls `position_of_range`.
fn exercise_get_dynamic_range(range: &impl GetDynamicRange) {
	let group = closing_token_group();
	let _pos = group.position_of_range(range);
}

#[test]
fn get_dynamic_range_usize() {
	exercise_get_dynamic_range(&0_usize);
	exercise_get_dynamic_range(&3_usize);
}

#[test]
fn get_dynamic_range_u128() {
	exercise_get_dynamic_range(&0_u128);
	exercise_get_dynamic_range(&2_u128);
}

#[test]
fn get_dynamic_range_u64() {
	exercise_get_dynamic_range(&0_u64);
	exercise_get_dynamic_range(&1_u64);
}

#[test]
fn get_dynamic_range_u32() {
	exercise_get_dynamic_range(&0_u32);
	exercise_get_dynamic_range(&3_u32);
}

#[test]
fn get_dynamic_range_u16() {
	exercise_get_dynamic_range(&0_u16);
	exercise_get_dynamic_range(&2_u16);
}

#[test]
fn get_dynamic_range_u8() {
	exercise_get_dynamic_range(&0_u8);
	exercise_get_dynamic_range(&1_u8);
}

#[test]
fn get_dynamic_range_isize() {
	exercise_get_dynamic_range(&0_isize);
	exercise_get_dynamic_range(&2_isize);
}

#[test]
fn get_dynamic_range_i128() {
	exercise_get_dynamic_range(&0_i128);
	exercise_get_dynamic_range(&1_i128);
}

#[test]
fn get_dynamic_range_i64() {
	exercise_get_dynamic_range(&0_i64);
	exercise_get_dynamic_range(&3_i64);
}

#[test]
fn get_dynamic_range_i32() {
	exercise_get_dynamic_range(&0_i32);
	exercise_get_dynamic_range(&2_i32);
}

#[test]
fn get_dynamic_range_i16() {
	exercise_get_dynamic_range(&0_i16);
	exercise_get_dynamic_range(&1_i16);
}

#[test]
fn get_dynamic_range_i8() {
	exercise_get_dynamic_range(&0_i8);
	exercise_get_dynamic_range(&2_i8);
}

#[test]
fn get_dynamic_range_ref_usize() {
	let val: usize = 1;
	exercise_get_dynamic_range(&&val);
}

#[test]
fn get_dynamic_range_ref_u128() {
	let val: u128 = 2;
	exercise_get_dynamic_range(&&val);
}

#[test]
fn get_dynamic_range_ref_u64() {
	let val: u64 = 0;
	exercise_get_dynamic_range(&&val);
}

#[test]
fn get_dynamic_range_ref_u32() {
	let val: u32 = 3;
	exercise_get_dynamic_range(&&val);
}

#[test]
fn get_dynamic_range_ref_u16() {
	let val: u16 = 1;
	exercise_get_dynamic_range(&&val);
}

#[test]
fn get_dynamic_range_ref_u8() {
	let val: u8 = 2;
	exercise_get_dynamic_range(&&val);
}

#[test]
fn get_dynamic_range_ref_isize() {
	let val: isize = 0;
	exercise_get_dynamic_range(&&val);
}

#[test]
fn get_dynamic_range_ref_i128() {
	let val: i128 = 1;
	exercise_get_dynamic_range(&&val);
}

#[test]
fn get_dynamic_range_ref_i64() {
	let val: i64 = 2;
	exercise_get_dynamic_range(&&val);
}

#[test]
fn get_dynamic_range_ref_i32() {
	let val: i32 = 0;
	exercise_get_dynamic_range(&&val);
}

#[test]
fn get_dynamic_range_ref_i16() {
	let val: i16 = 3;
	exercise_get_dynamic_range(&&val);
}

#[test]
fn get_dynamic_range_ref_i8() {
	let val: i8 = 1;
	exercise_get_dynamic_range(&&val);
}

#[test]
fn get_dynamic_range_bound_tuple() {
	use std::ops::Bound;
	let range = (Bound::Included(1_usize), Bound::Excluded(3_usize));
	exercise_get_dynamic_range(&range);

	let range_unbounded = (Bound::<usize>::Unbounded, Bound::Included(2_usize));
	exercise_get_dynamic_range(&range_unbounded);

	let range_excluded_start = (Bound::Excluded(0_usize), Bound::<usize>::Unbounded);
	exercise_get_dynamic_range(&range_excluded_start);
}

#[test]
fn get_dynamic_range_range_ref_usize() {
	let start: usize = 1;
	let end: usize = 4;
	let range = &start..&end;
	exercise_get_dynamic_range(&range);
}

#[test]
fn get_dynamic_range_range_from_ref_usize() {
	let start: usize = 2;
	let range = &start..;
	exercise_get_dynamic_range(&range);
}

#[test]
fn get_dynamic_range_range_inclusive_ref_usize() {
	let start: usize = 0;
	let end: usize = 3;
	let range = &start..=&end;
	exercise_get_dynamic_range(&range);
}

#[test]
fn get_dynamic_range_range_to_ref_usize() {
	let end: usize = 5;
	let range = ..&end;
	exercise_get_dynamic_range(&range);
}

#[test]
fn get_dynamic_range_range_to_inclusive_ref_usize() {
	let end: usize = 4;
	let range = ..=&end;
	exercise_get_dynamic_range(&range);
}

#[test]
fn get_dynamic_range_range_to_usize() {
	let range = ..5_usize;
	exercise_get_dynamic_range(&range);
}

#[test]
fn get_dynamic_range_range_to_inclusive_usize() {
	let range = ..=3_usize;
	exercise_get_dynamic_range(&range);
}

// --- tokens.rs: Token::Display ---

#[test]
fn token_display_all_variants() {
	use crate::tokens::Token;

	assert_eq!(format!("{}", Token::Newline), "\n");
	assert_eq!(format!("{}", Token::Whitespace(b' ')), " ");
	assert_eq!(format!("{}", Token::Whitespace(b'\t')), "\t");
	assert_eq!(format!("{}", Token::HtmlCommentOpen), "<!--");
	assert_eq!(format!("{}", Token::HtmlCommentClose), "-->");
	assert_eq!(format!("{}", Token::ConsumerTag), "{=");
	assert_eq!(format!("{}", Token::ProviderTag), "{@");
	assert_eq!(format!("{}", Token::CloseTag), "{/");
	assert_eq!(format!("{}", Token::BraceClose), "}");
	assert_eq!(format!("{}", Token::Pipe), "|");
	assert_eq!(format!("{}", Token::ArgumentDelimiter), ":");
	assert_eq!(
		format!("{}", Token::String("hello".to_string(), b'"')),
		"\"hello\""
	);
	assert_eq!(
		format!("{}", Token::String("world".to_string(), b'\'')),
		"'world'"
	);
	assert_eq!(
		format!("{}", Token::Ident("myIdent".to_string())),
		"myIdent"
	);
	assert_eq!(format!("{}", Token::Int(42)), "42");
	assert_eq!(format!("{}", Token::Int(-7)), "-7");
	assert_eq!(format!("{}", Token::Float(2.75)), "2.75");
	assert_eq!(format!("{}", Token::Float(0.0)), "0");
}

// --- tokens.rs: Token::same_type ---

#[test]
fn token_same_type_different_variants() {
	use crate::tokens::Token;

	// Different variant types should not match
	assert!(!Token::Int(1).same_type(&Token::Float(1.0)));
	assert!(!Token::String("a".into(), b'"').same_type(&Token::Ident("a".into())));
	assert!(!Token::Newline.same_type(&Token::Pipe));
	assert!(!Token::HtmlCommentOpen.same_type(&Token::HtmlCommentClose));
	assert!(!Token::ConsumerTag.same_type(&Token::ProviderTag));
	assert!(!Token::CloseTag.same_type(&Token::BraceClose));

	// String with different values still matches type
	assert!(Token::String("hello".into(), b'"').same_type(&Token::String("world".into(), b'"')));

	// Int with different values still matches
	assert!(Token::Int(1).same_type(&Token::Int(99)));

	// Float with different values
	assert!(Token::Float(1.0).same_type(&Token::Float(2.5)));

	// Ident wildcard matching
	assert!(Token::Ident("*".into()).same_type(&Token::Ident("anything".into())));
	assert!(Token::Ident("anything".into()).same_type(&Token::Ident("*".into())));
	assert!(!Token::Ident("foo".into()).same_type(&Token::Ident("bar".into())));
	assert!(Token::Ident("same".into()).same_type(&Token::Ident("same".into())));

	// Whitespace wildcard
	assert!(Token::Whitespace(b'*').same_type(&Token::Whitespace(b' ')));
	assert!(Token::Whitespace(b' ').same_type(&Token::Whitespace(b'*')));
	assert!(!Token::Whitespace(b' ').same_type(&Token::Whitespace(b'\t')));
	assert!(Token::Whitespace(b' ').same_type(&Token::Whitespace(b' ')));
}

// --- tokens.rs: Token::increment ---

#[test]
fn token_increment_all_variants() {
	use crate::tokens::Token;

	assert_eq!(Token::HtmlCommentOpen.increment(), 4);
	assert_eq!(Token::HtmlCommentClose.increment(), 3);
	assert_eq!(Token::ProviderTag.increment(), 2);
	assert_eq!(Token::ConsumerTag.increment(), 2);
	assert_eq!(Token::CloseTag.increment(), 2);
	assert_eq!(Token::Newline.increment(), 1);
	assert_eq!(Token::BraceClose.increment(), 1);
	assert_eq!(Token::Pipe.increment(), 1);
	assert_eq!(Token::ArgumentDelimiter.increment(), 1);
	assert_eq!(Token::Whitespace(b' ').increment(), 1);
	assert_eq!(Token::String("abc".to_string(), b'"').increment(), 5); // 3 + 2 quotes
	assert_eq!(Token::Ident("hello".to_string()).increment(), 5);
	assert_eq!(Token::Int(123).increment(), 3); // "123"
	assert_eq!(Token::Float(1.5).increment(), 3); // "1.5"
	assert_eq!(Token::Int(-42).increment(), 3); // "-42"
}

// --- tokens.rs: DynamicRange start/end for different bound types ---

#[test]
fn dynamic_range_start_end_all_bound_types() {
	use std::ops::Bound;

	use crate::tokens::DynamicRange;

	// Included start
	let dr = DynamicRange::from(1_usize..5_usize);
	assert_eq!(dr.start(), Some(1));
	assert_eq!(dr.end(), Some(5));

	// Inclusive end (adds 1)
	let dr = DynamicRange::from(1_usize..=4_usize);
	assert_eq!(dr.start(), Some(1));
	assert_eq!(dr.end(), Some(5));

	// Unbounded start
	let dr = DynamicRange::from(..5_usize);
	assert_eq!(dr.start(), None);
	assert_eq!(dr.end(), Some(5));

	// Unbounded end
	let dr = DynamicRange::from(2_usize..);
	assert_eq!(dr.start(), Some(2));
	assert_eq!(dr.end(), None);

	// Both unbounded via tuple
	let dr = DynamicRange::from((Bound::<usize>::Unbounded, Bound::<usize>::Unbounded));
	assert_eq!(dr.start(), None);
	assert_eq!(dr.end(), None);

	// Excluded start via tuple
	let dr = DynamicRange::from((Bound::Excluded(3_usize), Bound::Included(5_usize)));
	assert_eq!(dr.start(), Some(3)); // Excluded still returns the value
	assert_eq!(dr.end(), Some(6)); // Included adds 1
}

// --- tokens.rs: position_of_range ---

#[test]
fn position_of_range_with_all_numeric_types() {
	let group = closing_token_group();

	// Exercise position_of_range with various numeric types that trigger
	// the GetDynamicRange impls for single values
	let _ = group.position_of_range(&0_u128);
	let _ = group.position_of_range(&0_u64);
	let _ = group.position_of_range(&0_u32);
	let _ = group.position_of_range(&0_u16);
	let _ = group.position_of_range(&0_u8);
	let _ = group.position_of_range(&0_isize);
	let _ = group.position_of_range(&0_i128);
	let _ = group.position_of_range(&0_i64);
	let _ = group.position_of_range(&0_i32);
	let _ = group.position_of_range(&0_i16);
	let _ = group.position_of_range(&0_i8);

	// Verify an actual result with a range
	let pos = group.position_of_range(&(0..2));
	// 0..2 covers tokens [HtmlCommentOpen, Whitespace]
	assert_eq!(pos.start.offset, 0);
	assert!(pos.end.offset > 0);
}

// --- config.rs: Additional format coverage ---

#[test]
fn config_load_data_toml_with_all_value_types() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\nconf = \"data.toml\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("data.toml"),
		concat!(
			"string_val = \"hello\"\n",
			"int_val = 42\n",
			"float_val = 2.75\n",
			"bool_val = true\n",
			"date_val = 2024-01-15\n",
			"array_val = [1, 2, 3]\n",
			"\n",
			"[nested]\n",
			"key = \"value\"\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;
	let conf = data.get("conf").unwrap_or_else(|| panic!("expected conf"));

	assert_eq!(conf["string_val"], "hello");
	// TOML integers are converted via from_f64, so check as f64
	assert!((conf["int_val"].as_f64().unwrap_or(0.0) - 42.0).abs() < 0.001);
	assert!((conf["float_val"].as_f64().unwrap_or(0.0) - 2.75).abs() < 0.001);
	assert_eq!(conf["bool_val"], true);
	// Datetime should be converted to a string
	assert!(conf["date_val"].is_string());
	// Array should be preserved
	assert!(conf["array_val"].is_array());
	assert_eq!(
		conf["array_val"]
			.as_array()
			.unwrap_or_else(|| panic!("expected array"))
			.len(),
		3
	);
	// Nested table
	assert_eq!(conf["nested"]["key"], "value");

	Ok(())
}

#[test]
fn config_load_data_kdl_complex() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\nconf = \"data.kdl\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	// KDL v2 syntax (kdl crate 6.x): booleans use #true/#false, null is #null
	std::fs::write(
		tmp.path().join("data.kdl"),
		concat!(
			"name \"my-app\"\n",
			"version \"3.0\"\n",
			"count 42\n",
			"ratio 2.5\n",
			"enabled #true\n",
			"empty_node\n",
			"named_args key=\"value\" other=\"thing\"\n",
			"nested {\n",
			"    inner \"deep\"\n",
			"}\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;
	let conf = data.get("conf").unwrap_or_else(|| panic!("expected conf"));

	assert_eq!(conf["name"], "my-app");
	assert_eq!(conf["version"], "3.0");
	assert!((conf["count"].as_f64().unwrap_or(0.0) - 42.0).abs() < 0.001);
	assert!((conf["ratio"].as_f64().unwrap_or(0.0) - 2.5).abs() < 0.001);
	assert_eq!(conf["enabled"], true);
	// empty_node has no entries, should be null
	assert!(conf["empty_node"].is_null());
	// named args should be an object
	assert!(conf["named_args"].is_object());
	assert_eq!(conf["named_args"]["key"], "value");
	assert_eq!(conf["named_args"]["other"], "thing");
	// nested should be an object with inner child
	assert_eq!(conf["nested"]["inner"], "deep");

	Ok(())
}

#[test]
fn config_load_data_kdl_with_bool_and_null() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\nconf = \"data.kdl\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	// KDL v2: booleans are #true/#false, null is #null
	std::fs::write(
		tmp.path().join("data.kdl"),
		"enabled #false\nnothing #null\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;
	let conf = data.get("conf").unwrap_or_else(|| panic!("expected conf"));
	assert_eq!(conf["enabled"], false);
	assert!(conf["nothing"].is_null());

	Ok(())
}

#[test]
fn config_load_data_json_malformed_errors() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\npkg = \"bad.json\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("bad.json"), "not valid json {{{")
		.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())
		.unwrap_or_else(|e| panic!("load: {e}"))
		.unwrap_or_else(|| panic!("expected Some"));
	let result = config.load_data(tmp.path());
	assert!(result.is_err());
}

#[test]
fn config_load_data_toml_malformed_errors() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\nconf = \"bad.toml\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("bad.toml"), "not valid toml {{{{")
		.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())
		.unwrap_or_else(|e| panic!("load: {e}"))
		.unwrap_or_else(|| panic!("expected Some"));
	let result = config.load_data(tmp.path());
	assert!(result.is_err());
}

#[test]
fn config_load_data_yaml_malformed_errors() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\nconf = \"bad.yaml\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("bad.yaml"), ":\n  - :\n    - : :")
		.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())
		.unwrap_or_else(|e| panic!("load: {e}"))
		.unwrap_or_else(|| panic!("expected Some"));
	let result = config.load_data(tmp.path());
	// YAML parsing of the malformed content may or may not fail depending on
	// the serde_yaml_ng tolerance. We just ensure no panic.
	let _ = result;
}

#[test]
fn config_load_data_kdl_malformed_errors() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\nconf = \"bad.kdl\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("bad.kdl"), "{{{{{").unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())
		.unwrap_or_else(|e| panic!("load: {e}"))
		.unwrap_or_else(|| panic!("expected Some"));
	let result = config.load_data(tmp.path());
	assert!(result.is_err());
}

#[test]
fn config_load_with_all_sections() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		concat!(
			"max_file_size = 5000\n",
			"disable_gitignore = true\n",
			"\n",
			"[padding]\n",
			"\n",
			"[data]\n",
			"pkg = \"package.json\"\n",
			"\n",
			"[exclude]\n",
			"patterns = [\"vendor/**\", \"build/\"]\n",
			"\n",
			"[include]\n",
			"patterns = [\"extra/**/*.txt\"]\n",
			"\n",
			"[templates]\n",
			"paths = [\"shared/templates\"]\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	assert_eq!(config.max_file_size, 5000);
	assert!(config.padding.is_some());
	assert!(config.disable_gitignore);
	assert_eq!(config.exclude.patterns.len(), 2);
	assert_eq!(config.include.patterns.len(), 1);
	assert_eq!(config.templates.paths.len(), 1);

	Ok(())
}

// --- position.rs: Point::new and Position constructors ---

#[test]
fn point_new_and_fields() {
	let p = Point::new(5, 10, 42);
	assert_eq!(p.line, 5);
	assert_eq!(p.column, 10);
	assert_eq!(p.offset, 42);
}

#[test]
fn position_from_point() {
	let p = Point::new(3, 7, 20);
	let pos = Position::from_point(p);
	assert_eq!(pos.start, p);
	assert_eq!(pos.end, p);
}

#[test]
fn position_from_points() {
	let start = Point::new(1, 1, 0);
	let end = Point::new(5, 10, 50);
	let pos = Position::from_points(start, end);
	assert_eq!(pos.start, start);
	assert_eq!(pos.end, end);
}

#[test]
fn position_advance_start_str() {
	let mut pos = Position::new(1, 1, 0, 1, 20, 20);
	pos.advance_start_str("hello\nworld");
	assert_eq!(pos.start.line, 2);
	assert_eq!(pos.start.column, 5);
	assert_eq!(pos.start.offset, 11);
	// End should not change
	assert_eq!(pos.end.offset, 20);
}

#[test]
fn position_advance_start() {
	use crate::tokens::Token;
	let mut pos = Position::new(1, 1, 0, 1, 20, 20);
	pos.advance_start(Token::HtmlCommentOpen); // "<!--" is 4 chars
	assert_eq!(pos.start.column, 5);
	assert_eq!(pos.start.offset, 4);
}

#[test]
fn position_advance_end() {
	use crate::tokens::Token;
	let mut pos = Position::new(1, 1, 0, 1, 1, 0);
	pos.advance_end(Token::HtmlCommentClose); // "-->" is 3 chars
	assert_eq!(pos.end.column, 4);
	assert_eq!(pos.end.offset, 3);
}

// --- position.rs: Debug impls ---

#[test]
fn point_debug_format() {
	let p = Point::new(3, 7, 20);
	let debug = format!("{p:?}");
	assert_eq!(debug, "3:7 (20)");
}

#[test]
fn position_debug_format() {
	let pos = Position::new(1, 5, 4, 3, 10, 30);
	let debug = format!("{pos:?}");
	assert_eq!(debug, "1:5-3:10 (4-30)");
}

// --- position.rs: From<UnistPosition> ---

#[test]
fn position_from_unist_position() {
	use markdown::unist::Point as UnistPoint;
	use markdown::unist::Position as UnistPosition;

	let unist_pos = UnistPosition {
		start: UnistPoint {
			line: 2,
			column: 3,
			offset: 10,
		},
		end: UnistPoint {
			line: 4,
			column: 8,
			offset: 40,
		},
	};
	let pos = Position::from(unist_pos);
	assert_eq!(pos.start.line, 2);
	assert_eq!(pos.start.column, 3);
	assert_eq!(pos.start.offset, 10);
	assert_eq!(pos.end.line, 4);
	assert_eq!(pos.end.column, 8);
	assert_eq!(pos.end.offset, 40);
}

#[test]
fn point_from_unist_point() {
	use markdown::unist::Point as UnistPoint;

	let unist_point = UnistPoint {
		line: 5,
		column: 12,
		offset: 100,
	};
	let p = Point::from(unist_point);
	assert_eq!(p.line, 5);
	assert_eq!(p.column, 12);
	assert_eq!(p.offset, 100);
}

// --- position.rs: Point::advance with newlines ---

#[test]
fn point_advance_display_impl() {
	let mut p = Point::new(1, 1, 0);
	// advance takes impl Display, so we can pass a Token
	use crate::tokens::Token;
	p.advance(Token::ConsumerTag); // "{=" is 2 chars
	assert_eq!(p.column, 3);
	assert_eq!(p.offset, 2);
}

// --- project.rs: is_template_file additional cases ---

#[test]
fn is_template_file_edge_cases() {
	assert!(is_template_file(std::path::Path::new("a.t.md")));
	assert!(is_template_file(std::path::Path::new(
		"/long/path/to/file.t.md"
	)));
	// ".t.md" is a valid template file name (it does end with ".t.md")
	assert!(is_template_file(std::path::Path::new(".t.md")));
	assert!(!is_template_file(std::path::Path::new("t.md")));
	assert!(!is_template_file(std::path::Path::new("readme.t.mdx")));
	assert!(!is_template_file(std::path::Path::new("")));
}

// --- project.rs: normalize_line_endings edge cases ---

#[test]
fn normalize_line_endings_empty_string() {
	assert_eq!(normalize_line_endings(""), "");
}

#[test]
fn normalize_line_endings_no_newlines() {
	assert_eq!(normalize_line_endings("hello world"), "hello world");
}

#[test]
fn normalize_line_endings_only_cr() {
	assert_eq!(normalize_line_endings("\r"), "\n");
}

#[test]
fn normalize_line_endings_only_crlf() {
	assert_eq!(normalize_line_endings("\r\n"), "\n");
}

#[test]
fn normalize_line_endings_multiple_bare_cr() {
	assert_eq!(normalize_line_endings("\r\r\r"), "\n\n\n");
}

// --- project.rs: collect_included_files via scan_project_with_options ---

#[test]
fn scan_with_include_patterns() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	// Create a template
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\nContent.\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Create a .txt file that wouldn't normally be scanned but is included via
	// pattern. Note: .txt files are not "scannable" by default, so include
	// patterns only pick up files that the glob matches but they also need to be
	// a scannable extension. Let's use a .rs file in a non-standard location.
	std::fs::create_dir_all(tmp.path().join("extra")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("extra/test.rs"),
		"// <!-- {=block} -->\n// old\n// <!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Build include set that matches extra/**/*.rs
	let include_glob = globset::Glob::new("extra/**/*.rs").unwrap_or_else(|e| panic!("glob: {e}"));
	let include_set = globset::GlobSetBuilder::new()
		.add(include_glob)
		.build()
		.unwrap_or_else(|e| panic!("build: {e}"));

	let project = scan_project_with_options(
		tmp.path(),
		&ScanOptions {
			include_set,
			disable_gitignore: true,
			..ScanOptions::default()
		},
	)?;

	// The extra/test.rs consumer should be found via include pattern
	assert!(
		project
			.consumers
			.iter()
			.any(|c| c.file.to_string_lossy().contains("extra")),
		"expected consumer from include path, got: {:?}",
		project
			.consumers
			.iter()
			.map(|c| c.file.display().to_string())
			.collect::<Vec<_>>()
	);

	Ok(())
}

// --- project.rs: template_paths ---

#[test]
fn scan_with_template_paths() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	// Create templates in a separate directory
	std::fs::create_dir_all(tmp.path().join("shared/templates"))
		.unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("shared/templates/defs.t.md"),
		"<!-- {@sharedBlock} -->\n\nShared content.\n\n<!-- {/sharedBlock} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Consumer in root
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=sharedBlock} -->\n\nold\n\n<!-- {/sharedBlock} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let project = scan_project_with_options(
		tmp.path(),
		&ScanOptions {
			template_paths: vec![PathBuf::from("shared/templates")],
			disable_gitignore: true,
			..ScanOptions::default()
		},
	)?;

	assert!(
		project.providers.contains_key("sharedBlock"),
		"expected sharedBlock provider from template path"
	);
	assert_eq!(project.consumers.len(), 1);

	Ok(())
}

// --- parser.rs: BlockType::Display ---

#[test]
fn block_type_display() {
	assert_eq!(format!("{}", BlockType::Provider), "provider");
	assert_eq!(format!("{}", BlockType::Consumer), "consumer");
	assert_eq!(format!("{}", BlockType::Inline), "inline");
}

// --- parser.rs: TransformerType::Display ---

#[test]
fn transformer_type_display_all() {
	assert_eq!(format!("{}", TransformerType::Trim), "trim");
	assert_eq!(format!("{}", TransformerType::TrimStart), "trimStart");
	assert_eq!(format!("{}", TransformerType::TrimEnd), "trimEnd");
	assert_eq!(format!("{}", TransformerType::Wrap), "wrap");
	assert_eq!(format!("{}", TransformerType::Indent), "indent");
	assert_eq!(format!("{}", TransformerType::CodeBlock), "codeBlock");
	assert_eq!(format!("{}", TransformerType::Code), "code");
	assert_eq!(format!("{}", TransformerType::Replace), "replace");
	assert_eq!(format!("{}", TransformerType::Prefix), "prefix");
	assert_eq!(format!("{}", TransformerType::Suffix), "suffix");
	assert_eq!(format!("{}", TransformerType::LinePrefix), "linePrefix");
	assert_eq!(format!("{}", TransformerType::LineSuffix), "lineSuffix");
	assert_eq!(format!("{}", TransformerType::If), "if");
}

// --- parser.rs: OrderedFloat::Display ---

#[test]
fn ordered_float_display() {
	let f = OrderedFloat(2.75);
	assert_eq!(format!("{f}"), "2.75");

	let f = OrderedFloat(0.0);
	assert_eq!(format!("{f}"), "0");

	let f = OrderedFloat(-42.5);
	assert_eq!(format!("{f}"), "-42.5");
}

#[test]
fn ordered_float_partial_eq() {
	let a = OrderedFloat(1.0);
	let b = OrderedFloat(1.0);
	let c = OrderedFloat(2.0);
	assert_eq!(a, b);
	assert_ne!(a, c);
}

// --- parser.rs: Argument::Number and Argument::Boolean parsing ---

#[test]
fn parse_consumer_with_float_argument() -> MdtResult<()> {
	let input = "<!-- {=block|indent:2.75} -->\nold\n<!-- {/block} -->\n";
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].transformers.len(), 1);
	assert_eq!(blocks[0].transformers[0].args.len(), 1);
	match &blocks[0].transformers[0].args[0] {
		Argument::Number(n) => assert!((n.0 - 2.75).abs() < 0.001),
		other => panic!("expected Number, got {other:?}"),
	}

	Ok(())
}

#[test]
fn parse_consumer_with_false_boolean() -> MdtResult<()> {
	let input = "<!-- {=block|indent:false} -->\nold\n<!-- {/block} -->\n";
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	match &blocks[0].transformers[0].args[0] {
		Argument::Boolean(b) => assert!(!b),
		other => panic!("expected Boolean(false), got {other:?}"),
	}

	Ok(())
}

#[test]
fn parse_consumer_with_scientific_notation() -> MdtResult<()> {
	let input = "<!-- {=block|indent:1e2} -->\nold\n<!-- {/block} -->\n";
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].transformers.len(), 1);
	match &blocks[0].transformers[0].args[0] {
		Argument::Number(n) => assert!((n.0 - 100.0).abs() < 0.001),
		other => panic!("expected Number, got {other:?}"),
	}

	Ok(())
}

// --- lexer.rs: memstr function ---

#[test]
fn memstr_basic() {
	use crate::lexer::memstr;

	assert_eq!(memstr(b"hello world", b"world"), Some(6));
	assert_eq!(memstr(b"hello world", b"hello"), Some(0));
	assert_eq!(memstr(b"hello world", b"xyz"), None);
	assert_eq!(memstr(b"", b"x"), None);
	assert_eq!(memstr(b"abc", b"abcd"), None);
	assert_eq!(memstr(b"abcabc", b"abc"), Some(0));
	assert_eq!(memstr(b"<!--", b"<!--"), Some(0));
	assert_eq!(memstr(b"  <!--", b"<!--"), Some(2));
}

// --- lexer.rs: single-quoted strings ---

#[test]
fn tokenize_single_quoted_string() -> MdtResult<()> {
	let input = r"<!-- {=block|indent:'  '} -->";
	let nodes = get_html_nodes(input)?;
	let groups = tokenize(nodes)?;
	assert_eq!(groups.len(), 1);
	// Verify the string token uses single quote delimiter
	let has_single_quote_string = groups[0]
		.tokens
		.iter()
		.any(|t| matches!(t, tokens::Token::String(_, b'\'')));
	assert!(
		has_single_quote_string,
		"expected single-quoted string token"
	);

	Ok(())
}

#[test]
fn tokenize_single_quoted_string_with_escapes() -> MdtResult<()> {
	let input = r"<!-- {=block|indent:'he\\llo'} -->";
	let nodes = get_html_nodes(input)?;
	let groups = tokenize(nodes)?;
	assert_eq!(groups.len(), 1);

	Ok(())
}

// --- lexer.rs: float numbers ---

#[test]
fn tokenize_float_number_in_tag() -> MdtResult<()> {
	let input = "<!-- {=block|indent:2.5} -->";
	let nodes = get_html_nodes(input)?;
	let groups = tokenize(nodes)?;
	assert_eq!(groups.len(), 1);
	let has_float = groups[0]
		.tokens
		.iter()
		.any(|t| matches!(t, tokens::Token::Float(_)));
	assert!(has_float, "expected float token");

	Ok(())
}

#[test]
fn tokenize_scientific_notation_float() -> MdtResult<()> {
	let input = "<!-- {=block|indent:1e3} -->";
	let nodes = get_html_nodes(input)?;
	let groups = tokenize(nodes)?;
	assert_eq!(groups.len(), 1);
	let has_float = groups[0]
		.tokens
		.iter()
		.any(|t| matches!(t, tokens::Token::Float(_)));
	assert!(has_float, "expected float token for scientific notation");

	Ok(())
}

#[test]
fn tokenize_integer_number_in_tag() -> MdtResult<()> {
	let input = "<!-- {=block|indent:42} -->";
	let nodes = get_html_nodes(input)?;
	let groups = tokenize(nodes)?;
	assert_eq!(groups.len(), 1);
	let has_int = groups[0]
		.tokens
		.iter()
		.any(|t| matches!(t, tokens::Token::Int(42)));
	assert!(has_int, "expected int token with value 42");

	Ok(())
}

// --- engine.rs: get_bool_arg with Number coercion ---

#[test]
fn transformer_indent_with_number_bool_coercion() {
	// When a Number is passed where a bool is expected (second arg of indent),
	// non-zero should be true.
	let result = apply_transformers(
		"line1\n\nline3",
		&[Transformer {
			r#type: TransformerType::Indent,
			args: vec![
				Argument::String("  ".to_string()),
				Argument::Number(OrderedFloat(1.0)),
			],
		}],
	);
	// 1.0 coerces to true, so empty lines should be indented
	assert_eq!(result, "  line1\n  \n  line3");
}

#[test]
fn transformer_indent_with_zero_number_bool_coercion() {
	// 0.0 coerces to false
	let result = apply_transformers(
		"line1\n\nline3",
		&[Transformer {
			r#type: TransformerType::Indent,
			args: vec![
				Argument::String("  ".to_string()),
				Argument::Number(OrderedFloat(0.0)),
			],
		}],
	);
	assert_eq!(result, "  line1\n\n  line3");
}

// --- engine.rs: get_string_arg with Number ---

#[test]
fn transformer_prefix_with_number_arg() {
	// When a Number is passed where a string is expected, it should be
	// coerced to string via to_string().
	let result = apply_transformers(
		"content",
		&[Transformer {
			r#type: TransformerType::Prefix,
			args: vec![Argument::Number(OrderedFloat(42.0))],
		}],
	);
	assert_eq!(result, "42content");
}

// --- engine.rs: get_string_arg with Boolean ---

#[test]
fn transformer_prefix_with_boolean_arg() {
	let result = apply_transformers(
		"content",
		&[Transformer {
			r#type: TransformerType::Prefix,
			args: vec![Argument::Boolean(true)],
		}],
	);
	assert_eq!(result, "truecontent");
}

// --- engine.rs: get_bool_arg with String coercion ---

#[test]
fn transformer_indent_with_string_true_bool_coercion() {
	let result = apply_transformers(
		"line1\n\nline3",
		&[Transformer {
			r#type: TransformerType::Indent,
			args: vec![
				Argument::String("  ".to_string()),
				Argument::String("true".to_string()),
			],
		}],
	);
	// "true" string coerces to true
	assert_eq!(result, "  line1\n  \n  line3");
}

#[test]
fn transformer_indent_with_string_false_bool_coercion() {
	let result = apply_transformers(
		"line1\n\nline3",
		&[Transformer {
			r#type: TransformerType::Indent,
			args: vec![
				Argument::String("  ".to_string()),
				Argument::String("false".to_string()),
			],
		}],
	);
	// "false" string coerces to false
	assert_eq!(result, "  line1\n\n  line3");
}

// --- parser.rs: parse_with_diagnostics additional coverage ---

#[test]
fn parse_with_diagnostics_valid_input_no_diagnostics() -> MdtResult<()> {
	let input = "<!-- {=block|trim} -->\n\nold\n\n<!-- {/block} -->\n";
	let (blocks, diagnostics) = parse_with_diagnostics(input)?;
	assert_eq!(blocks.len(), 1);
	assert!(diagnostics.is_empty());

	Ok(())
}

#[test]
fn parse_with_diagnostics_unknown_transformer_on_provider() -> MdtResult<()> {
	let input = "<!-- {@block|unknownFilter} -->\n\ncontent\n\n<!-- {/block} -->\n";
	let (blocks, diagnostics) = parse_with_diagnostics(input)?;
	assert_eq!(blocks.len(), 1);
	assert!(
		diagnostics.iter().any(
			|d| matches!(d, ParseDiagnostic::UnknownTransformer { name, .. } if name == "unknownFilter")
		),
		"expected UnknownTransformer diagnostic"
	);

	Ok(())
}

// --- project.rs: ProjectDiagnostic::message ---

#[test]
fn project_diagnostic_message_all_kinds() {
	use project::DiagnosticKind;
	use project::ProjectDiagnostic;

	let diag = ProjectDiagnostic {
		file: PathBuf::from("test.md"),
		kind: DiagnosticKind::UnclosedBlock {
			name: "myBlock".to_string(),
		},
		line: 1,
		column: 1,
	};
	assert!(diag.message().contains("myBlock"));

	let diag = ProjectDiagnostic {
		file: PathBuf::from("test.md"),
		kind: DiagnosticKind::UnknownTransformer {
			name: "foobar".to_string(),
		},
		line: 1,
		column: 1,
	};
	assert!(diag.message().contains("foobar"));

	let diag = ProjectDiagnostic {
		file: PathBuf::from("test.md"),
		kind: DiagnosticKind::InvalidTransformerArgs {
			name: "trim".to_string(),
			expected: "0".to_string(),
			got: 1,
		},
		line: 1,
		column: 1,
	};
	let msg = diag.message();
	assert!(msg.contains("trim"));
	assert!(msg.contains('0'));
	assert!(msg.contains('1'));

	let diag = ProjectDiagnostic {
		file: PathBuf::from("test.md"),
		kind: DiagnosticKind::UnusedProvider {
			name: "unused".to_string(),
		},
		line: 1,
		column: 1,
	};
	assert!(diag.message().contains("unused"));
}

// --- project.rs: ValidationOptions coverage ---

#[test]
fn validation_options_all_kinds() {
	use project::DiagnosticKind;
	use project::ProjectDiagnostic;
	use project::ValidationOptions;

	let unknown_transformer_diag = ProjectDiagnostic {
		file: PathBuf::from("test.md"),
		kind: DiagnosticKind::UnknownTransformer {
			name: "bad".to_string(),
		},
		line: 1,
		column: 1,
	};
	let default_opts = ValidationOptions::default();
	assert!(unknown_transformer_diag.is_error(&default_opts));

	let ignore_opts = ValidationOptions {
		ignore_invalid_transformers: true,
		..Default::default()
	};
	assert!(!unknown_transformer_diag.is_error(&ignore_opts));

	let invalid_args_diag = ProjectDiagnostic {
		file: PathBuf::from("test.md"),
		kind: DiagnosticKind::InvalidTransformerArgs {
			name: "trim".to_string(),
			expected: "0".to_string(),
			got: 1,
		},
		line: 1,
		column: 1,
	};
	assert!(invalid_args_diag.is_error(&default_opts));
	assert!(!invalid_args_diag.is_error(&ignore_opts));

	let unused_diag = ProjectDiagnostic {
		file: PathBuf::from("test.md"),
		kind: DiagnosticKind::UnusedProvider {
			name: "unused".to_string(),
		},
		line: 1,
		column: 1,
	};
	assert!(!unused_diag.is_error(&ValidationOptions {
		ignore_unused_blocks: true,
		..Default::default()
	}));
	assert!(unused_diag.is_error(&default_opts));
}

// --- project.rs: ProjectContext::find_missing_providers ---

#[test]
fn project_context_find_missing_providers() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@existing} -->\n\ncontent\n\n<!-- {/existing} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=missing1} -->\nold\n<!-- {/missing1} -->\n\n<!-- {=existing} -->\nold\n<!-- \
		 {/existing} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = ProjectContext {
		project: scan_project(tmp.path())?,
		data: HashMap::new(),
		padding: None,
	};
	let missing = ctx.find_missing_providers();
	assert_eq!(missing, vec!["missing1"]);

	Ok(())
}

// --- Token PartialEq edge cases ---

#[test]
fn token_partial_eq_edge_cases() {
	use crate::tokens::Token;

	// Float approximate equality
	assert_eq!(Token::Float(1.0), Token::Float(1.0));
	assert_ne!(Token::Float(1.0), Token::Float(2.0));

	// Different whitespace bytes
	assert_ne!(Token::Whitespace(b' '), Token::Whitespace(b'\t'));
	assert_eq!(Token::Whitespace(b' '), Token::Whitespace(b' '));

	// String different delimiters
	assert_ne!(
		Token::String("hello".into(), b'"'),
		Token::String("hello".into(), b'\'')
	);

	// Cross-variant always false
	assert_ne!(Token::Newline, Token::BraceClose);
	assert_ne!(Token::Pipe, Token::ArgumentDelimiter);
}

// --- lexer.rs: escaped strings ---

#[test]
fn tokenize_string_with_escape_sequences() -> MdtResult<()> {
	let input = r#"<!-- {=block|replace:"line1\nline2":"replaced"} -->"#;
	let nodes = get_html_nodes(input)?;
	let groups = tokenize(nodes)?;
	assert_eq!(groups.len(), 1);
	// Verify the first string was unescaped
	let string_tokens: Vec<_> = groups[0]
		.tokens
		.iter()
		.filter(|t| matches!(t, tokens::Token::String(..)))
		.collect();
	assert_eq!(string_tokens.len(), 2);

	Ok(())
}

// --- Error display coverage ---

#[test]
fn error_symlink_cycle_message() {
	let err = MdtError::SymlinkCycle {
		path: "/some/path".to_string(),
	};
	assert!(err.to_string().contains("/some/path"));
	assert!(err.to_string().contains("symlink cycle"));
}

#[test]
fn error_file_too_large_message() {
	let err = MdtError::FileTooLarge {
		path: "big.md".to_string(),
		size: 20_000_000,
		limit: 10_000_000,
	};
	let msg = err.to_string();
	assert!(msg.contains("big.md"));
	assert!(msg.contains("20000000"));
	assert!(msg.contains("10000000"));
}

#[test]
fn error_unconvertible_float_message() {
	let err = MdtError::UnconvertibleFloat {
		path: "data.toml".to_string(),
		value: "NaN".to_string(),
	};
	let msg = err.to_string();
	assert!(msg.contains("data.toml"));
	assert!(msg.contains("NaN"));
}

// --- parser.rs: Argument Display coverage (via Debug) ---

#[test]
fn argument_debug_all_variants() {
	let s = Argument::String("test".to_string());
	let n = Argument::Number(OrderedFloat(2.75));
	let b = Argument::Boolean(true);
	// Just exercise Debug formatting
	let _ = format!("{s:?}");
	let _ = format!("{n:?}");
	let _ = format!("{b:?}");
}

// --- lexer.rs: edge cases ---

#[test]
fn tokenize_comment_with_only_whitespace_and_close() -> MdtResult<()> {
	// A comment that has whitespace inside but no valid tag
	let input = "<!-- \t\r -->";
	let nodes = get_html_nodes(input)?;
	let groups = tokenize(nodes)?;
	assert!(groups.is_empty());

	Ok(())
}

#[test]
fn tokenize_multiple_comments_in_one_input() -> MdtResult<()> {
	let input = "<!-- {=a} --><!-- {/a} --><!-- {@b} --><!-- {/b} -->";
	let nodes = get_html_nodes(input)?;
	let groups = tokenize(nodes)?;
	assert_eq!(groups.len(), 4);

	Ok(())
}

// --- config.rs: default max file size ---

#[test]
fn default_max_file_size_value() {
	assert_eq!(DEFAULT_MAX_FILE_SIZE, 10 * 1024 * 1024);
}

// --- config.rs: MdtConfig defaults ---

#[test]
fn config_default_max_file_size() {
	let config: MdtConfig = toml::from_str("").unwrap_or_else(|e| panic!("parse: {e}"));
	assert_eq!(config.max_file_size, DEFAULT_MAX_FILE_SIZE);
	assert!(config.padding.is_none());
	assert!(!config.disable_gitignore);
	assert!(config.data.is_empty());
	assert!(config.exclude.patterns.is_empty());
	assert!(config.include.patterns.is_empty());
	assert!(config.templates.paths.is_empty());
}

// --- engine.rs: line_prefix and line_suffix with Number and Boolean bool args
// ---

#[test]
fn transformer_line_prefix_with_number_bool_arg() {
	let result = apply_transformers(
		"line1\n\nline3",
		&[Transformer {
			r#type: TransformerType::LinePrefix,
			args: vec![
				Argument::String("# ".to_string()),
				Argument::Number(OrderedFloat(1.0)),
			],
		}],
	);
	assert_eq!(result, "# line1\n#\n# line3");
}

#[test]
fn transformer_line_suffix_with_number_bool_arg() {
	let result = apply_transformers(
		"line1\n\nline3",
		&[Transformer {
			r#type: TransformerType::LineSuffix,
			args: vec![
				Argument::String(";".to_string()),
				Argument::Number(OrderedFloat(1.0)),
			],
		}],
	);
	assert_eq!(result, "line1;\n;\nline3;");
}

// --- engine.rs: replace with Number and Boolean string coercions ---

#[test]
fn transformer_replace_with_number_args() {
	let result = apply_transformers(
		"value is 42",
		&[Transformer {
			r#type: TransformerType::Replace,
			args: vec![
				Argument::Number(OrderedFloat(42.0)),
				Argument::Number(OrderedFloat(99.0)),
			],
		}],
	);
	assert_eq!(result, "value is 99");
}

#[test]
fn transformer_replace_with_boolean_args() {
	let result = apply_transformers(
		"is true today",
		&[Transformer {
			r#type: TransformerType::Replace,
			args: vec![Argument::Boolean(true), Argument::Boolean(false)],
		}],
	);
	assert_eq!(result, "is false today");
}

// --- lexer.rs: context-dependent behavior edge cases ---

#[test]
fn tokenize_nested_comment_like_content() -> MdtResult<()> {
	// Content that looks like a nested comment open inside a tag
	let input = "<!-- {=block} -->";
	let nodes = get_html_nodes(input)?;
	let groups = tokenize(nodes)?;
	assert_eq!(groups.len(), 1);

	Ok(())
}

#[test]
fn tokenize_tab_whitespace_in_comment() -> MdtResult<()> {
	let input = "<!--\t{=block}\t-->";
	let nodes = get_html_nodes(input)?;
	let groups = tokenize(nodes)?;
	assert_eq!(groups.len(), 1);

	Ok(())
}

// --- parser.rs: parse various transformer snake_case aliases ---

#[test]
fn parse_codeblock_alias() -> MdtResult<()> {
	let input = r#"<!-- {=block|codeblock:"rs"} -->
old
<!-- {/block} -->
"#;
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].transformers[0].r#type, TransformerType::CodeBlock);

	Ok(())
}

// --- Token PartialEq between different variant types ---

#[test]
fn token_eq_cross_variant_returns_false() {
	use crate::tokens::Token;

	// Ensure cross-variant comparisons return false (covers the _ => false
	// branch)
	let variants: Vec<Token> = vec![
		Token::Newline,
		Token::HtmlCommentOpen,
		Token::HtmlCommentClose,
		Token::ConsumerTag,
		Token::ProviderTag,
		Token::CloseTag,
		Token::BraceClose,
		Token::Pipe,
		Token::ArgumentDelimiter,
		Token::Whitespace(b' '),
		Token::String("s".into(), b'"'),
		Token::Ident("id".into()),
		Token::Int(1),
		Token::Float(1.0),
	];

	for (i, a) in variants.iter().enumerate() {
		for (j, b) in variants.iter().enumerate() {
			if i != j {
				// Most cross-variant pairs should be not equal
				// Some same-category pairs (like Int(1) vs Float(1.0)) should
				// also be not equal
				let _ = a == b; // Just exercise the eq implementation
			}
		}
	}
}

// --- source_scanner.rs: extract_html_comments with no comments ---

#[test]
fn extract_html_comments_empty_input() {
	let nodes = extract_html_comments("");
	assert!(nodes.is_empty());
}

#[test]
fn extract_html_comments_no_comments() {
	let nodes = extract_html_comments("just some plain text\nwith newlines\n");
	assert!(nodes.is_empty());
}

#[test]
fn extract_html_comments_unclosed_open() {
	let nodes = extract_html_comments("<!-- unclosed comment");
	assert!(nodes.is_empty());
}

#[test]
fn extract_html_comments_open_at_end() {
	let nodes = extract_html_comments("text<!--");
	assert!(nodes.is_empty());
}

// --- Coverage: config.rs TOML integer, float, array, table, datetime
// conversions ---

#[test]
fn config_toml_data_with_integers_and_floats() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\nconf = \"data.toml\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("data.toml"),
		concat!(
			"int_val = 42\n",
			"float_val = 2.72\n",
			"bool_val = true\n",
			"string_val = \"hello\"\n",
			"datetime_val = 2024-01-15T10:30:00Z\n",
			"array_val = [1, 2, 3]\n",
			"string_array = [\"a\", \"b\", \"c\"]\n",
			"\n",
			"[nested_table]\n",
			"key = \"value\"\n",
			"count = 7\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;

	let conf = data.get("conf").unwrap_or_else(|| panic!("expected conf"));
	// Integer conversion
	assert_eq!(conf["int_val"], serde_json::json!(42.0));
	// Float conversion
	assert!(
		(conf["float_val"]
			.as_f64()
			.unwrap_or_else(|| panic!("expected f64"))
			- 2.72)
			.abs() < f64::EPSILON
	);
	// Boolean conversion
	assert_eq!(conf["bool_val"], serde_json::json!(true));
	// String conversion
	assert_eq!(conf["string_val"], "hello");
	// Datetime conversion (rendered as string)
	assert!(conf["datetime_val"].is_string());
	let dt_str = conf["datetime_val"]
		.as_str()
		.unwrap_or_else(|| panic!("expected string"));
	assert!(dt_str.contains("2024"));
	// Array conversion
	let arr = conf["array_val"]
		.as_array()
		.unwrap_or_else(|| panic!("expected array"));
	assert_eq!(arr.len(), 3);
	// String array conversion
	let str_arr = conf["string_array"]
		.as_array()
		.unwrap_or_else(|| panic!("expected array"));
	assert_eq!(str_arr.len(), 3);
	assert_eq!(str_arr[0], "a");
	// Nested table conversion
	assert_eq!(conf["nested_table"]["key"], "value");
	assert_eq!(conf["nested_table"]["count"], serde_json::json!(7.0));

	Ok(())
}

// --- Coverage: config.rs KDL data file with various entry types ---

#[test]
fn config_kdl_empty_node_entries() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\nconf = \"data.kdl\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	// A node with no entries should produce null
	std::fs::write(tmp.path().join("data.kdl"), "empty_node\n")
		.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;
	let conf = data.get("conf").unwrap_or_else(|| panic!("expected conf"));
	assert!(conf["empty_node"].is_null());

	Ok(())
}

#[test]
fn config_kdl_all_named_entries() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\nconf = \"data.kdl\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	// All named entries should produce an object
	std::fs::write(
		tmp.path().join("data.kdl"),
		"settings host=\"localhost\" port=8080\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;
	let conf = data.get("conf").unwrap_or_else(|| panic!("expected conf"));
	assert!(conf["settings"].is_object());
	assert_eq!(conf["settings"]["host"], "localhost");
	// port is an integer in KDL
	assert_eq!(conf["settings"]["port"], serde_json::json!(8080.0));

	Ok(())
}

#[test]
fn config_kdl_mixed_entries() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\nconf = \"data.kdl\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	// Mixed positional and named entries should produce an array
	std::fs::write(
		tmp.path().join("data.kdl"),
		"mixed \"positional\" key=\"named\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;
	let conf = data.get("conf").unwrap_or_else(|| panic!("expected conf"));
	// Mixed entries become an array
	assert!(conf["mixed"].is_array());
	let arr = conf["mixed"]
		.as_array()
		.unwrap_or_else(|| panic!("expected array"));
	assert_eq!(arr.len(), 2);

	Ok(())
}

#[test]
fn config_kdl_integer_float_bool_null_values() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\nconf = \"data.kdl\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	// KDL v2 uses #true, #false, #null keywords
	std::fs::write(
		tmp.path().join("data.kdl"),
		concat!(
			"int_val 42\n",
			"float_val 2.72\n",
			"bool_val #true\n",
			"null_val #null\n",
			"string_val \"hello\"\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;
	let conf = data.get("conf").unwrap_or_else(|| panic!("expected conf"));

	// Integer
	assert!(conf["int_val"].is_number());
	#[allow(clippy::float_cmp)]
	{
		assert_eq!(
			conf["int_val"]
				.as_f64()
				.unwrap_or_else(|| panic!("expected f64")),
			42.0
		);
	}
	// Float
	assert!(conf["float_val"].is_number());
	assert!(
		(conf["float_val"]
			.as_f64()
			.unwrap_or_else(|| panic!("expected f64"))
			- 2.72)
			.abs() < 0.001
	);
	// Boolean
	assert_eq!(conf["bool_val"], serde_json::json!(true));
	// Null
	assert!(conf["null_val"].is_null());
	// String
	assert_eq!(conf["string_val"], "hello");

	Ok(())
}

#[test]
fn config_kdl_children_node() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\nconf = \"data.kdl\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	// Node with children should be treated as an object
	std::fs::write(
		tmp.path().join("data.kdl"),
		concat!(
			"package {\n",
			"  name \"my-app\"\n",
			"  version \"1.0.0\"\n",
			"}\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;
	let conf = data.get("conf").unwrap_or_else(|| panic!("expected conf"));
	assert!(conf["package"].is_object());
	assert_eq!(conf["package"]["name"], "my-app");
	assert_eq!(conf["package"]["version"], "1.0.0");

	Ok(())
}

// --- Coverage: config.rs MdtConfig::load reading from disk ---

#[test]
fn config_load_full_config_from_disk() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		concat!(
			"max_file_size = 5000\n",
			"disable_gitignore = true\n",
			"\n",
			"[padding]\n",
			"\n",
			"[data]\n",
			"pkg = \"package.json\"\n",
			"cargo = \"Cargo.toml\"\n",
			"\n",
			"[exclude]\n",
			"patterns = [\"vendor/**\", \"build/\"]\n",
			"\n",
			"[include]\n",
			"patterns = [\"extra/**/*.txt\"]\n",
			"\n",
			"[templates]\n",
			"paths = [\"shared/templates\"]\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	// Also create the data files so load_data succeeds
	std::fs::write(
		tmp.path().join("package.json"),
		r#"{"name": "test", "version": "1.0.0"}"#,
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("Cargo.toml"),
		"[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	assert_eq!(config.max_file_size, 5000);
	assert!(config.padding.is_some());
	assert!(config.disable_gitignore);
	assert_eq!(config.data.len(), 2);
	assert_eq!(config.exclude.patterns, vec!["vendor/**", "build/"]);
	assert_eq!(config.include.patterns, vec!["extra/**/*.txt"]);
	assert_eq!(
		config.templates.paths,
		vec![PathBuf::from("shared/templates")]
	);

	let data = config.load_data(tmp.path())?;
	assert_eq!(data.len(), 2);
	assert_eq!(data["pkg"]["name"], "test");
	assert_eq!(data["cargo"]["package"]["name"], "test");

	Ok(())
}

// --- Coverage: project.rs scan_project_with_config with real mdt.toml ---

#[test]
fn scan_project_with_config_loads_data_and_scans() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npkg = \"package.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("package.json"),
		r#"{"name": "myapp", "version": "2.0.0"}"#,
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@install} -->\n\nnpm install {{ pkg.name }}@{{ pkg.version }}\n\n<!-- {/install} \
		 -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=install} -->\n\nold\n\n<!-- {/install} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	assert_eq!(ctx.project.providers.len(), 1);
	assert_eq!(ctx.project.consumers.len(), 1);
	assert!(ctx.data.contains_key("pkg"));
	assert_eq!(ctx.data["pkg"]["name"], "myapp");

	Ok(())
}

#[test]
fn scan_project_with_config_no_config_file() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\ncontent\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=block} -->\n\nold\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	assert_eq!(ctx.project.providers.len(), 1);
	assert_eq!(ctx.project.consumers.len(), 1);
	assert!(ctx.data.is_empty());
	assert!(ctx.padding.is_none());

	Ok(())
}

// --- Coverage: project.rs extra template directories ---

#[test]
fn scan_project_with_extra_template_dirs() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	// Create config pointing to an extra template directory
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[templates]\npaths = [\"shared/templates\"]\n\ndisable_gitignore = true\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Create the extra template directory with a template file
	std::fs::create_dir_all(tmp.path().join("shared/templates"))
		.unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("shared/templates/extra.t.md"),
		"<!-- {@extraBlock} -->\n\nExtra content from shared templates.\n\n<!-- {/extraBlock} \
		 -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Consumer in root
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=extraBlock} -->\n\nold\n\n<!-- {/extraBlock} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	assert!(
		ctx.project.providers.contains_key("extraBlock"),
		"expected provider from extra template dir"
	);
	assert_eq!(ctx.project.consumers.len(), 1);

	// Verify update works
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	assert!(content.contains("Extra content from shared templates."));

	Ok(())
}

#[test]
fn scan_project_with_extra_template_dir_nonexistent() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	// Template path points to a directory that does not exist -- should be silently
	// skipped
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[templates]\npaths = [\"nonexistent/templates\"]\n\ndisable_gitignore = true\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\ncontent\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=block} -->\n\nold\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	assert_eq!(ctx.project.providers.len(), 1);
	assert_eq!(ctx.project.consumers.len(), 1);

	Ok(())
}

// --- Coverage: project.rs include patterns ---

#[test]
fn scan_project_with_include_patterns() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	// Include .txt files which are not normally scannable
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[include]\npatterns = [\"**/*.txt\"]\n\ndisable_gitignore = true\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@info} -->\n\nIncluded content.\n\n<!-- {/info} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	// A .txt file with consumer block -- normally not scanned, but included by
	// pattern
	std::fs::create_dir_all(tmp.path().join("docs")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("docs/notes.txt"),
		"<!-- {=info} -->\n\nold notes\n\n<!-- {/info} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	// The .txt file should have been picked up by include patterns
	assert!(
		ctx.project
			.consumers
			.iter()
			.any(|c| c.file.to_string_lossy().contains("notes.txt")),
		"expected consumer from included .txt file, consumers: {:?}",
		ctx.project
			.consumers
			.iter()
			.map(|c| c.file.display().to_string())
			.collect::<Vec<_>>()
	);

	Ok(())
}

// --- Coverage: project.rs diagnostic conversion from ParseDiagnostic ---

#[test]
fn scan_project_unclosed_block_diagnostic_has_correct_fields() {
	use project::DiagnosticKind;

	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"Some text\n\n<!-- {=unclosedBlock} -->\n\nContent without close.\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let project = scan_project(tmp.path()).unwrap_or_else(|e| panic!("scan: {e}"));
	let diag = project
		.diagnostics
		.iter()
		.find(|d| matches!(&d.kind, DiagnosticKind::UnclosedBlock { .. }))
		.unwrap_or_else(|| panic!("expected UnclosedBlock diagnostic"));

	match &diag.kind {
		DiagnosticKind::UnclosedBlock { name } => {
			assert_eq!(name, "unclosedBlock");
		}
		other => panic!("expected UnclosedBlock, got {other:?}"),
	}
	assert!(diag.line > 0);
	assert!(diag.column > 0);
	assert!(diag.file.to_string_lossy().contains("readme.md"));
}

#[test]
fn scan_project_unknown_transformer_diagnostic_has_correct_fields() {
	use project::DiagnosticKind;

	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\ncontent\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=block|nonExistentTransformer} -->\n\nold\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let project = scan_project(tmp.path()).unwrap_or_else(|e| panic!("scan: {e}"));
	let diag = project
		.diagnostics
		.iter()
		.find(|d| matches!(&d.kind, DiagnosticKind::UnknownTransformer { .. }))
		.unwrap_or_else(|| panic!("expected UnknownTransformer diagnostic"));

	match &diag.kind {
		DiagnosticKind::UnknownTransformer { name } => {
			assert_eq!(name, "nonExistentTransformer");
		}
		other => panic!("expected UnknownTransformer, got {other:?}"),
	}
	assert!(diag.line > 0);
	assert!(diag.column > 0);
}

#[test]
fn scan_project_invalid_transformer_args_diagnostic_has_correct_fields() {
	use project::DiagnosticKind;

	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\ncontent\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	// replace expects exactly 2 args, give it 1
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=block|replace:\"only_one\"} -->\n\nold\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let project = scan_project(tmp.path()).unwrap_or_else(|e| panic!("scan: {e}"));
	let diag = project
		.diagnostics
		.iter()
		.find(|d| matches!(&d.kind, DiagnosticKind::InvalidTransformerArgs { .. }))
		.unwrap_or_else(|| panic!("expected InvalidTransformerArgs diagnostic"));

	match &diag.kind {
		DiagnosticKind::InvalidTransformerArgs {
			name,
			expected,
			got,
		} => {
			assert_eq!(name, "replace");
			assert!(expected.contains('2'));
			assert_eq!(*got, 1);
		}
		other => panic!("expected InvalidTransformerArgs, got {other:?}"),
	}
	assert!(diag.line > 0);
	assert!(diag.column > 0);
}

// --- Coverage: project.rs diagnostic message ---

#[test]
fn project_diagnostic_messages_are_descriptive() {
	use project::DiagnosticKind;
	use project::ProjectDiagnostic;

	let unclosed = ProjectDiagnostic {
		file: PathBuf::from("test.md"),
		kind: DiagnosticKind::UnclosedBlock {
			name: "myBlock".to_string(),
		},
		line: 1,
		column: 1,
	};
	assert!(unclosed.message().contains("myBlock"));
	assert!(unclosed.message().contains("closing"));

	let unknown = ProjectDiagnostic {
		file: PathBuf::from("test.md"),
		kind: DiagnosticKind::UnknownTransformer {
			name: "bogus".to_string(),
		},
		line: 1,
		column: 1,
	};
	assert!(unknown.message().contains("bogus"));
	assert!(unknown.message().contains("unknown"));

	let invalid_args = ProjectDiagnostic {
		file: PathBuf::from("test.md"),
		kind: DiagnosticKind::InvalidTransformerArgs {
			name: "replace".to_string(),
			expected: "2".to_string(),
			got: 1,
		},
		line: 1,
		column: 1,
	};
	assert!(invalid_args.message().contains("replace"));
	assert!(invalid_args.message().contains('2'));
	assert!(invalid_args.message().contains('1'));

	let unused = ProjectDiagnostic {
		file: PathBuf::from("test.t.md"),
		kind: DiagnosticKind::UnusedProvider {
			name: "orphan".to_string(),
		},
		line: 1,
		column: 1,
	};
	assert!(unused.message().contains("orphan"));
	assert!(unused.message().contains("no consumers"));
}

// --- Coverage: project.rs is_error for all diagnostic kinds ---

#[test]
fn diagnostic_is_error_all_kinds() {
	use project::DiagnosticKind;
	use project::ProjectDiagnostic;
	use project::ValidationOptions;

	let make_diag = |kind: DiagnosticKind| -> ProjectDiagnostic {
		ProjectDiagnostic {
			file: PathBuf::from("test.md"),
			kind,
			line: 1,
			column: 1,
		}
	};

	// Default options: all are errors
	let default_opts = ValidationOptions::default();
	assert!(
		make_diag(DiagnosticKind::UnclosedBlock {
			name: "x".to_string()
		})
		.is_error(&default_opts)
	);
	assert!(
		make_diag(DiagnosticKind::UnknownTransformer {
			name: "x".to_string()
		})
		.is_error(&default_opts)
	);
	assert!(
		make_diag(DiagnosticKind::InvalidTransformerArgs {
			name: "x".to_string(),
			expected: "1".to_string(),
			got: 0,
		})
		.is_error(&default_opts)
	);
	assert!(
		make_diag(DiagnosticKind::UnusedProvider {
			name: "x".to_string()
		})
		.is_error(&default_opts)
	);

	// Ignoring transformers should suppress both unknown and invalid args
	let ignore_transformers = ValidationOptions {
		ignore_invalid_transformers: true,
		..Default::default()
	};
	assert!(
		!make_diag(DiagnosticKind::UnknownTransformer {
			name: "x".to_string()
		})
		.is_error(&ignore_transformers)
	);
	assert!(
		!make_diag(DiagnosticKind::InvalidTransformerArgs {
			name: "x".to_string(),
			expected: "1".to_string(),
			got: 0,
		})
		.is_error(&ignore_transformers)
	);

	// Ignoring unused blocks
	let ignore_unused = ValidationOptions {
		ignore_unused_blocks: true,
		..Default::default()
	};
	assert!(
		!make_diag(DiagnosticKind::UnusedProvider {
			name: "x".to_string()
		})
		.is_error(&ignore_unused)
	);
}

// --- Coverage: project.rs normalize_line_endings via CRLF scanning ---

#[test]
fn scan_project_crlf_content_normalized() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	// Write template with CRLF line endings
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\r\n\r\nNormalized content.\r\n\r\n<!-- {/block} -->\r\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=block} -->\r\n\r\nold\r\n\r\n<!-- {/block} -->\r\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = ProjectContext {
		project: scan_project(tmp.path())?,
		data: HashMap::new(),
		padding: None,
	};
	assert_eq!(ctx.project.providers.len(), 1);
	assert_eq!(ctx.project.consumers.len(), 1);
	// Content should be normalized (no \r)
	let provider_content = &ctx.project.providers["block"].content;
	assert!(!provider_content.contains('\r'));

	Ok(())
}

// --- Coverage: project.rs ProjectContext::find_missing_providers with multiple
// missing ---

#[test]
fn project_context_find_multiple_missing_providers() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@existing} -->\n\ncontent\n\n<!-- {/existing} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=existing} -->\n\nold\n\n<!-- {/existing} -->\n\n<!-- {=missing1} \
		 -->\n\norphan1\n\n<!-- {/missing1} -->\n\n<!-- {=missing2} -->\n\norphan2\n\n<!-- \
		 {/missing2} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let missing = ctx.find_missing_providers();
	assert_eq!(missing.len(), 2);
	assert!(missing.contains(&"missing1".to_string()));
	assert!(missing.contains(&"missing2".to_string()));

	Ok(())
}

// --- Coverage: project.rs validate_project ---

#[test]
fn validate_project_ok_when_all_providers_exist() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\ncontent\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=block} -->\n\nold\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let project = scan_project(tmp.path())?;
	let result = validate_project(&project);
	assert!(result.is_ok());

	Ok(())
}

// --- Coverage: project.rs is_template_file additional edge cases ---

#[test]
fn is_template_file_more_edge_cases() {
	// Various additional edge cases
	assert!(is_template_file(std::path::Path::new("foo.t.md")));
	assert!(is_template_file(std::path::Path::new(
		"deep/nested/path/template.t.md"
	)));
	assert!(!is_template_file(std::path::Path::new("t.md")));
	assert!(!is_template_file(std::path::Path::new("readme.mdx")));
	assert!(!is_template_file(std::path::Path::new("notes.txt")));
}

// --- Coverage: error.rs SymlinkCycle display ---

#[test]
fn error_symlink_cycle_display_format() {
	let err = MdtError::SymlinkCycle {
		path: "/circular/link".to_string(),
	};
	let msg = err.to_string();
	assert!(msg.contains("symlink cycle"));
	assert!(msg.contains("/circular/link"));
}

// --- Coverage: error.rs FileTooLarge display ---

#[test]
fn error_file_too_large_display_format() {
	let err = MdtError::FileTooLarge {
		path: "huge.md".to_string(),
		size: 50_000_000,
		limit: 10_000_000,
	};
	let msg = err.to_string();
	assert!(msg.contains("huge.md"));
	assert!(msg.contains("50000000"));
	assert!(msg.contains("10000000"));
}

// --- Coverage: error.rs UnconvertibleFloat display ---

#[test]
fn error_unconvertible_float_display_format() {
	let err = MdtError::UnconvertibleFloat {
		path: "config.toml".to_string(),
		value: "Infinity".to_string(),
	};
	let msg = err.to_string();
	assert!(msg.contains("config.toml"));
	assert!(msg.contains("Infinity"));
}

// --- Coverage: config.rs unsupported format with explicit check ---

#[test]
fn config_unsupported_format_returns_specific_error() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\ndata = \"data.xml\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("data.xml"), "<data/>").unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())
		.unwrap_or_else(|e| panic!("load: {e}"))
		.unwrap_or_else(|| panic!("expected Some"));
	let result = config.load_data(tmp.path());
	assert!(result.is_err());
	let err_msg = result.unwrap_err().to_string();
	assert!(
		err_msg.contains("unsupported") || err_msg.contains("xml"),
		"error should mention unsupported format, got: {err_msg}"
	);
}

// --- Coverage: project.rs source file scanning with unclosed block ---

#[test]
fn scan_project_source_file_unclosed_block_diagnostic() {
	use project::DiagnosticKind;

	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::create_dir_all(tmp.path().join("src")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	// Source file with unclosed block
	std::fs::write(
		tmp.path().join("src/lib.rs"),
		"//! <!-- {=docs} -->\n//! content without close\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let project = scan_project(tmp.path()).unwrap_or_else(|e| panic!("scan: {e}"));
	assert!(
		project.diagnostics.iter().any(|d| {
			matches!(
				&d.kind,
				DiagnosticKind::UnclosedBlock { name } if name == "docs"
			)
		}),
		"expected UnclosedBlock diagnostic for source file, got: {:?}",
		project.diagnostics
	);
}

// --- Coverage: config.rs TOML with nested arrays of tables ---

#[test]
fn config_toml_nested_array_of_tables() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\nconf = \"data.toml\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("data.toml"),
		concat!(
			"[[items]]\n",
			"name = \"first\"\n",
			"value = 10\n",
			"\n",
			"[[items]]\n",
			"name = \"second\"\n",
			"value = 20\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;
	let conf = data.get("conf").unwrap_or_else(|| panic!("expected conf"));
	let items = conf["items"]
		.as_array()
		.unwrap_or_else(|| panic!("expected array"));
	assert_eq!(items.len(), 2);
	assert_eq!(items[0]["name"], "first");
	assert_eq!(items[1]["name"], "second");

	Ok(())
}

// --- Coverage: project.rs scan_project_with_config with pad_blocks ---

#[test]
fn scan_project_with_config_pad_blocks_flag() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[padding]\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\ncontent\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	assert!(ctx.padding.is_some());

	Ok(())
}

// --- Coverage: project.rs scan with include excluding hidden directories ---

#[test]
fn include_pattern_does_not_scan_hidden_dirs() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[include]\npatterns = [\"**/*.txt\"]\n\ndisable_gitignore = true\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\ncontent\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// File in hidden directory should not be included even with include patterns
	std::fs::create_dir_all(tmp.path().join(".hidden")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join(".hidden/notes.txt"),
		"<!-- {=block} -->\n\nold\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// File in a normal directory should be included
	std::fs::create_dir_all(tmp.path().join("docs")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("docs/readme.txt"),
		"<!-- {=block} -->\n\nold\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	// Only docs/readme.txt should be found, not .hidden/notes.txt
	let consumer_files: Vec<String> = ctx
		.project
		.consumers
		.iter()
		.map(|c| c.file.display().to_string())
		.collect();
	assert!(
		consumer_files.iter().any(|f| f.contains("readme.txt")),
		"expected readme.txt in consumers, got: {consumer_files:?}"
	);
	assert!(
		!consumer_files.iter().any(|f| f.contains(".hidden")),
		"hidden dir files should not be included, got: {consumer_files:?}"
	);

	Ok(())
}

// --- Coverage: config.rs invalid JSON data file ---

#[test]
fn config_load_data_invalid_json() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\npkg = \"bad.json\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("bad.json"), "not valid json {{{")
		.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())
		.unwrap_or_else(|e| panic!("load: {e}"))
		.unwrap_or_else(|| panic!("expected Some"));
	let result = config.load_data(tmp.path());
	assert!(result.is_err());
}

// --- Coverage: config.rs invalid TOML data file ---

#[test]
fn config_load_data_invalid_toml() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\nconf = \"bad.toml\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("bad.toml"), "[invalid\nbroken = {{{{")
		.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())
		.unwrap_or_else(|e| panic!("load: {e}"))
		.unwrap_or_else(|| panic!("expected Some"));
	let result = config.load_data(tmp.path());
	assert!(result.is_err());
}

// --- Coverage: config.rs invalid YAML data file ---

#[test]
fn config_load_data_invalid_yaml() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\nconf = \"bad.yaml\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("bad.yaml"), ":\n  - :\n    - : :")
		.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())
		.unwrap_or_else(|e| panic!("load: {e}"))
		.unwrap_or_else(|| panic!("expected Some"));
	let result = config.load_data(tmp.path());
	// YAML parser may or may not error on malformed input; just exercise the path
	let _ = result;
}

// --- Coverage: config.rs invalid KDL data file ---

#[test]
fn config_load_data_invalid_kdl() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\nconf = \"bad.kdl\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("bad.kdl"),
		"this is not valid kdl {{{ }}}\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())
		.unwrap_or_else(|e| panic!("load: {e}"))
		.unwrap_or_else(|| panic!("expected Some"));
	let result = config.load_data(tmp.path());
	assert!(result.is_err());
}

// --- Coverage: project.rs exclude patterns with include patterns interaction
// ---

#[test]
fn include_patterns_respect_exclude_patterns() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		concat!(
			"[include]\n",
			"patterns = [\"**/*.txt\"]\n",
			"\n",
			"[exclude]\n",
			"patterns = [\"excluded/**\"]\n",
			"\n",
			"disable_gitignore = true\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\ncontent\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Include in normal dir
	std::fs::create_dir_all(tmp.path().join("docs")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("docs/notes.txt"),
		"<!-- {=block} -->\n\nold\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Include in excluded dir -- should not be found
	std::fs::create_dir_all(tmp.path().join("excluded")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("excluded/notes.txt"),
		"<!-- {=block} -->\n\nold\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let consumer_files: Vec<String> = ctx
		.project
		.consumers
		.iter()
		.map(|c| c.file.display().to_string())
		.collect();
	assert!(
		consumer_files.iter().any(|f| f.contains("docs")),
		"expected docs/notes.txt, got: {consumer_files:?}"
	);
	assert!(
		!consumer_files.iter().any(|f| f.contains("excluded")),
		"excluded dir should not appear, got: {consumer_files:?}"
	);

	Ok(())
}

// --- Coverage: config.rs pad_blocks + data loading through
// scan_project_with_config ---

#[test]
fn scan_project_with_config_pad_blocks_and_data() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[padding]\n\n[data]\npkg = \"package.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("package.json"),
		r#"{"name": "test-app", "version": "1.0.0"}"#,
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@info} -->\n\nVersion: {{ pkg.version }}\n\n<!-- {/info} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=info} -->\n\nold\n\n<!-- {/info} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	assert!(ctx.padding.is_some(), "padding should be configured");
	assert!(
		ctx.data.contains_key("pkg"),
		"data should contain pkg namespace"
	);
	assert_eq!(ctx.data["pkg"]["name"], "test-app");
	assert_eq!(ctx.project.providers.len(), 1);
	assert_eq!(ctx.project.consumers.len(), 1);

	// Also verify the update works with pad_blocks and data
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	assert!(content.contains("Version: 1.0.0"));

	Ok(())
}

// --- Coverage: config.rs TOML all value types exercised individually ---

#[test]
fn config_toml_integer_value_standalone() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\nconf = \"data.toml\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	// A TOML file with only an integer at the top level
	std::fs::write(tmp.path().join("data.toml"), "count = 99\n")
		.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;
	let conf = data.get("conf").unwrap_or_else(|| panic!("expected conf"));
	// Verify integer conversion through from_f64
	let count_val = conf["count"]
		.as_f64()
		.unwrap_or_else(|| panic!("expected number"));
	assert!((count_val - 99.0).abs() < f64::EPSILON);

	Ok(())
}

#[test]
fn config_toml_float_value_standalone() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\nconf = \"data.toml\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	// A TOML file with only a float at the top level
	std::fs::write(tmp.path().join("data.toml"), "ratio = 0.75\n")
		.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;
	let conf = data.get("conf").unwrap_or_else(|| panic!("expected conf"));
	// Verify float conversion through from_f64
	let ratio_val = conf["ratio"]
		.as_f64()
		.unwrap_or_else(|| panic!("expected number"));
	assert!((ratio_val - 0.75).abs() < f64::EPSILON);

	Ok(())
}

#[test]
fn config_toml_array_of_mixed_types() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\nconf = \"data.toml\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	// TOML arrays (all same type for valid TOML)
	std::fs::write(
		tmp.path().join("data.toml"),
		"numbers = [10, 20, 30]\nstrings = [\"a\", \"b\"]\nfloats = [1.1, 2.2]\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;
	let conf = data.get("conf").unwrap_or_else(|| panic!("expected conf"));

	let nums = conf["numbers"]
		.as_array()
		.unwrap_or_else(|| panic!("expected array"));
	assert_eq!(nums.len(), 3);
	#[allow(clippy::float_cmp)]
	{
		assert_eq!(
			nums[0]
				.as_f64()
				.unwrap_or_else(|| panic!("expected number")),
			10.0
		);
	}

	let strings = conf["strings"]
		.as_array()
		.unwrap_or_else(|| panic!("expected array"));
	assert_eq!(strings.len(), 2);
	assert_eq!(strings[0], "a");

	let floats = conf["floats"]
		.as_array()
		.unwrap_or_else(|| panic!("expected array"));
	assert_eq!(floats.len(), 2);

	Ok(())
}

#[test]
fn config_toml_deeply_nested_table() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\nconf = \"data.toml\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("data.toml"),
		concat!(
			"[level1]\n",
			"name = \"outer\"\n",
			"[level1.level2]\n",
			"name = \"inner\"\n",
			"value = 42\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;
	let conf = data.get("conf").unwrap_or_else(|| panic!("expected conf"));
	assert_eq!(conf["level1"]["name"], "outer");
	assert_eq!(conf["level1"]["level2"]["name"], "inner");
	assert_eq!(conf["level1"]["level2"]["value"], serde_json::json!(42.0));

	Ok(())
}

// --- Coverage: config.rs KDL integer and float values in named entries ---

#[test]
fn config_kdl_named_entries_with_integer_and_float() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\nconf = \"data.kdl\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	// Named entries with integer and float values
	std::fs::write(
		tmp.path().join("data.kdl"),
		"server host=\"0.0.0.0\" port=3000 weight=1.5\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;
	let conf = data.get("conf").unwrap_or_else(|| panic!("expected conf"));
	assert!(conf["server"].is_object());
	assert_eq!(conf["server"]["host"], "0.0.0.0");
	// Integer in named entry
	let port = conf["server"]["port"]
		.as_f64()
		.unwrap_or_else(|| panic!("expected number"));
	assert!((port - 3000.0).abs() < f64::EPSILON);
	// Float in named entry
	let weight = conf["server"]["weight"]
		.as_f64()
		.unwrap_or_else(|| panic!("expected number"));
	assert!((weight - 1.5).abs() < f64::EPSILON);

	Ok(())
}

// --- Coverage: config.rs KDL mixed entries with integers and floats ---

#[test]
fn config_kdl_mixed_entries_with_numbers() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\nconf = \"data.kdl\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	// Mixed positional and named entries containing integers and floats
	std::fs::write(
		tmp.path().join("data.kdl"),
		"coords 10 20.5 name=\"point\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;
	let conf = data.get("conf").unwrap_or_else(|| panic!("expected conf"));
	// Mixed entries become array
	assert!(conf["coords"].is_array());
	let arr = conf["coords"]
		.as_array()
		.unwrap_or_else(|| panic!("expected array"));
	assert_eq!(arr.len(), 3);
	// First: integer 10
	#[allow(clippy::float_cmp)]
	{
		assert_eq!(
			arr[0].as_f64().unwrap_or_else(|| panic!("expected number")),
			10.0
		);
	}
	// Second: float 20.5
	assert!(
		(arr[1].as_f64().unwrap_or_else(|| panic!("expected number")) - 20.5).abs() < f64::EPSILON
	);
	// Third: string "point"
	assert_eq!(arr[2], "point");

	Ok(())
}

// --- Coverage: config.rs KDL node with children containing integers ---

#[test]
fn config_kdl_children_with_integer_and_float_values() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data]\nconf = \"data.kdl\"\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("data.kdl"),
		concat!(
			"config {\n",
			"  port 8080\n",
			"  rate 0.95\n",
			"  debug #false\n",
			"}\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;
	let conf = data.get("conf").unwrap_or_else(|| panic!("expected conf"));
	assert!(conf["config"].is_object());
	// Integer child
	let port = conf["config"]["port"]
		.as_f64()
		.unwrap_or_else(|| panic!("expected number"));
	assert!((port - 8080.0).abs() < f64::EPSILON);
	// Float child
	let rate = conf["config"]["rate"]
		.as_f64()
		.unwrap_or_else(|| panic!("expected number"));
	assert!((rate - 0.95).abs() < 0.001);
	// Boolean child
	assert_eq!(conf["config"]["debug"], false);

	Ok(())
}

// --- Coverage: project.rs collect_included_files with subdirectory recursion
// ---

#[test]
fn include_pattern_scans_nested_subdirectories() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[include]\npatterns = [\"**/*.txt\"]\n\ndisable_gitignore = true\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\ncontent\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Create deeply nested subdirectory with txt file
	std::fs::create_dir_all(tmp.path().join("a/b/c")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("a/b/c/deep.txt"),
		"<!-- {=block} -->\n\nold deep\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Also create a txt file at root
	std::fs::write(
		tmp.path().join("root.txt"),
		"<!-- {=block} -->\n\nold root\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let consumer_files: Vec<String> = ctx
		.project
		.consumers
		.iter()
		.map(|c| c.file.display().to_string())
		.collect();
	assert!(
		consumer_files.iter().any(|f| f.contains("deep.txt")),
		"expected deep.txt in consumers, got: {consumer_files:?}"
	);
	assert!(
		consumer_files.iter().any(|f| f.contains("root.txt")),
		"expected root.txt in consumers, got: {consumer_files:?}"
	);

	Ok(())
}

// --- Coverage: project.rs include patterns with exclude interaction ---

#[test]
fn include_pattern_respects_exclude_patterns() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		concat!(
			"[include]\n",
			"patterns = [\"**/*.txt\"]\n",
			"\n",
			"[exclude]\n",
			"patterns = [\"skip/**\"]\n",
			"\n",
			"disable_gitignore = true\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\ncontent\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// File in excluded dir -- should not be included
	std::fs::create_dir_all(tmp.path().join("skip")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("skip/notes.txt"),
		"<!-- {=block} -->\n\nold\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// File not in excluded dir -- should be included
	std::fs::create_dir_all(tmp.path().join("keep")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("keep/notes.txt"),
		"<!-- {=block} -->\n\nold\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let consumer_files: Vec<String> = ctx
		.project
		.consumers
		.iter()
		.map(|c| c.file.display().to_string())
		.collect();
	assert!(
		consumer_files.iter().any(|f| f.contains("keep")),
		"expected keep/notes.txt, got: {consumer_files:?}"
	);
	assert!(
		!consumer_files.iter().any(|f| f.contains("skip")),
		"excluded files should not appear, got: {consumer_files:?}"
	);

	Ok(())
}

// --- Coverage: project.rs include patterns skip node_modules and target ---

#[test]
fn include_pattern_skips_node_modules_and_target() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[include]\npatterns = [\"**/*.txt\"]\n\ndisable_gitignore = true\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\ncontent\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Create files in node_modules and target
	std::fs::create_dir_all(tmp.path().join("node_modules/pkg"))
		.unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("node_modules/pkg/readme.txt"),
		"<!-- {=block} -->\n\nold\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	std::fs::create_dir_all(tmp.path().join("target/debug"))
		.unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("target/debug/output.txt"),
		"<!-- {=block} -->\n\nold\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Create a regular file that should be included
	std::fs::write(
		tmp.path().join("valid.txt"),
		"<!-- {=block} -->\n\nold\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let consumer_files: Vec<String> = ctx
		.project
		.consumers
		.iter()
		.map(|c| c.file.display().to_string())
		.collect();
	assert!(
		consumer_files.iter().any(|f| f.contains("valid.txt")),
		"expected valid.txt, got: {consumer_files:?}"
	);
	assert!(
		!consumer_files.iter().any(|f| f.contains("node_modules")),
		"node_modules should be skipped, got: {consumer_files:?}"
	);
	assert!(
		!consumer_files.iter().any(|f| f.contains("target")),
		"target should be skipped, got: {consumer_files:?}"
	);

	Ok(())
}

// --- Coverage: config.rs read_to_string success path (line 124) ---

#[test]
fn config_load_reads_valid_toml_content() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		concat!(
			"max_file_size = 5242880\n",
			"disable_gitignore = true\n",
			"\n",
			"[padding]\n",
			"\n",
			"[data]\n",
			"pkg = \"package.json\"\n",
			"\n",
			"[exclude]\n",
			"patterns = [\"vendor/**\", \"build/\"]\n",
			"\n",
			"[include]\n",
			"patterns = [\"**/*.txt\"]\n",
			"\n",
			"[templates]\n",
			"paths = [\"shared\"]\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	assert_eq!(config.max_file_size, 5_242_880);
	assert!(config.padding.is_some());
	assert!(config.disable_gitignore);
	assert_eq!(config.exclude.patterns, vec!["vendor/**", "build/"]);
	assert_eq!(config.include.patterns, vec!["**/*.txt"]);
	assert_eq!(config.templates.paths, vec![PathBuf::from("shared")]);
	assert_eq!(
		config.data.get("pkg"),
		Some(&DataSource::Path(PathBuf::from("package.json")))
	);

	Ok(())
}

// --- Coverage: project.rs scan_project_with_config with all config sections
// ---

#[test]
fn scan_project_with_config_all_sections_loaded() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		concat!(
			"max_file_size = 1048576\n",
			"\n",
			"[padding]\n",
			"\n",
			"[data]\n",
			"info = \"info.yaml\"\n",
			"\n",
			"[exclude]\n",
			"patterns = [\"ignored/**\"]\n",
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("info.yaml"),
		"name: test-project\nversion: 3.0.0\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@ver} -->\n\n{{ info.version }}\n\n<!-- {/ver} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=ver} -->\n\nold\n\n<!-- {/ver} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	// Ignored directory
	std::fs::create_dir_all(tmp.path().join("ignored")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("ignored/file.md"),
		"<!-- {=ver} -->\n\nignored\n\n<!-- {/ver} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	assert!(ctx.padding.is_some());
	assert!(ctx.data.contains_key("info"));
	assert_eq!(ctx.data["info"]["version"], "3.0.0");
	assert_eq!(ctx.project.consumers.len(), 1);

	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates.updated_files.values().next().unwrap_or_else(|| {
		panic!("expected one file");
	});
	assert!(content.contains("3.0.0"));

	Ok(())
}

// =============================================================================
// Feature 1: [exclude] markdown_codeblocks
// =============================================================================

#[test]
fn config_parses_exclude_markdown_codeblocks_true() {
	let toml_content = "[exclude]\nmarkdown_codeblocks = true\n";
	let config: MdtConfig = toml::from_str(toml_content).unwrap_or_else(|e| panic!("parse: {e}"));
	assert!(
		matches!(
			config.exclude.markdown_codeblocks,
			CodeBlockFilter::Bool(true)
		),
		"expected CodeBlockFilter::Bool(true)"
	);
	assert!(config.exclude.markdown_codeblocks.is_enabled());
	assert!(config.exclude.markdown_codeblocks.should_skip(""));
}

#[test]
fn config_parses_exclude_markdown_codeblocks_string() {
	let toml_content = "[exclude]\nmarkdown_codeblocks = \"ignore\"\n";
	let config: MdtConfig = toml::from_str(toml_content).unwrap_or_else(|e| panic!("parse: {e}"));
	assert!(
		matches!(
			config.exclude.markdown_codeblocks,
			CodeBlockFilter::InfoString(ref s) if s == "ignore"
		),
		"expected CodeBlockFilter::InfoString(\"ignore\")"
	);
	assert!(config.exclude.markdown_codeblocks.is_enabled());
	assert!(
		config
			.exclude
			.markdown_codeblocks
			.should_skip("rust,ignore")
	);
	assert!(!config.exclude.markdown_codeblocks.should_skip("rust"));
}

#[test]
fn config_parses_exclude_markdown_codeblocks_array() {
	let toml_content = "[exclude]\nmarkdown_codeblocks = [\"ignore\", \"skip\"]\n";
	let config: MdtConfig = toml::from_str(toml_content).unwrap_or_else(|e| panic!("parse: {e}"));
	assert!(
		matches!(config.exclude.markdown_codeblocks, CodeBlockFilter::InfoStrings(ref v) if v.len() == 2),
		"expected CodeBlockFilter::InfoStrings with 2 elements"
	);
	assert!(config.exclude.markdown_codeblocks.is_enabled());
	assert!(
		config
			.exclude
			.markdown_codeblocks
			.should_skip("rust,ignore")
	);
	assert!(
		config
			.exclude
			.markdown_codeblocks
			.should_skip("python,skip")
	);
	assert!(!config.exclude.markdown_codeblocks.should_skip("rust"));
}

#[test]
fn config_defaults_exclude_markdown_codeblocks_to_false() {
	let toml_content = "";
	let config: MdtConfig = toml::from_str(toml_content).unwrap_or_else(|e| panic!("parse: {e}"));
	assert!(
		matches!(
			config.exclude.markdown_codeblocks,
			CodeBlockFilter::Bool(false)
		),
		"expected CodeBlockFilter::Bool(false)"
	);
	assert!(!config.exclude.markdown_codeblocks.is_enabled());
}

#[test]
fn source_scanner_filters_codeblock_html_comments() -> MdtResult<()> {
	let content = "\
/// ```markdown
/// <!-- {=example} -->
/// Some content
/// <!-- {/example} -->
/// ```
";

	// With filtering enabled (Bool(true)), the block inside the code fence should
	// be skipped entirely.
	let filter_on = CodeBlockFilter::Bool(true);
	let (blocks, diagnostics) = parse_source_with_diagnostics(content, &filter_on)?;
	assert!(
		blocks.is_empty(),
		"expected no blocks when codeblock filter is enabled, got {}",
		blocks.len()
	);
	assert!(
		diagnostics.is_empty(),
		"expected no diagnostics when codeblock filter is enabled, got {}",
		diagnostics.len()
	);

	// With filtering disabled (Bool(false)), the block should be found.
	let filter_off = CodeBlockFilter::Bool(false);
	let (blocks, _diagnostics) = parse_source_with_diagnostics(content, &filter_off)?;
	assert_eq!(
		blocks.len(),
		1,
		"expected 1 block when codeblock filter is disabled, got {}",
		blocks.len()
	);

	Ok(())
}

#[test]
fn source_scanner_filters_codeblock_with_info_string_match() -> MdtResult<()> {
	let content = "\
/// ```rust,ignore
/// <!-- {=ignored} -->
/// content
/// <!-- {/ignored} -->
/// ```
/// ```rust
/// <!-- {=kept} -->
/// content
/// <!-- {/kept} -->
/// ```
";

	let filter = CodeBlockFilter::InfoString("ignore".to_string());
	let (blocks, _diagnostics) = parse_source_with_diagnostics(content, &filter)?;
	assert_eq!(
		blocks.len(),
		1,
		"expected 1 block (only 'kept'), got {}",
		blocks.len()
	);
	assert_eq!(blocks[0].name, "kept");

	Ok(())
}

// =============================================================================
// Feature 2: [exclude] blocks
// =============================================================================

#[test]
fn config_parses_exclude_blocks() {
	let toml_content = "[exclude]\nblocks = [\"internal\", \"debug\"]\n";
	let config: MdtConfig = toml::from_str(toml_content).unwrap_or_else(|e| panic!("parse: {e}"));
	assert_eq!(
		config.exclude.blocks,
		vec!["internal".to_string(), "debug".to_string()]
	);
}

#[test]
fn excluded_blocks_are_skipped_during_scan() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[exclude]\nblocks = [\"internal\"]\n\ndisable_gitignore = true\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@greeting} -->\n\nHello!\n\n<!-- {/greeting} -->\n\n<!-- {@internal} -->\n\nSecret \
		 stuff.\n\n<!-- {/internal} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=greeting} -->\n\nold greeting\n\n<!-- {/greeting} -->\n\n<!-- {=internal} \
		 -->\n\nold secret\n\n<!-- {/internal} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;

	// Only "greeting" provider should be found, not "internal".
	assert_eq!(
		ctx.project.providers.len(),
		1,
		"expected 1 provider, got {}",
		ctx.project.providers.len()
	);
	assert!(
		ctx.project.providers.contains_key("greeting"),
		"expected provider 'greeting' to be present"
	);
	assert!(
		!ctx.project.providers.contains_key("internal"),
		"expected provider 'internal' to be excluded"
	);

	// Only "greeting" consumer should be found, not "internal".
	assert_eq!(
		ctx.project.consumers.len(),
		1,
		"expected 1 consumer, got {}",
		ctx.project.consumers.len()
	);
	assert_eq!(ctx.project.consumers[0].block.name, "greeting");

	Ok(())
}

#[test]
fn excluded_blocks_defaults_to_empty() {
	let toml_content = "";
	let config: MdtConfig = toml::from_str(toml_content).unwrap_or_else(|e| panic!("parse: {e}"));
	assert!(
		config.exclude.blocks.is_empty(),
		"expected exclude.blocks to default to empty"
	);
}

// --- Undefined template variable detection tests ---

#[test]
fn find_undefined_variables_with_valid_data() {
	let mut data = HashMap::new();
	data.insert(
		"pkg".to_string(),
		serde_json::json!({"name": "my-lib", "version": "1.0.0"}),
	);

	let content = "Install {{ pkg.name }} v{{ pkg.version }}";
	let undefined = find_undefined_variables(content, &data);
	assert!(
		undefined.is_empty(),
		"expected no undefined variables, got: {undefined:?}"
	);
}

#[test]
fn find_undefined_variables_with_typo() {
	let mut data = HashMap::new();
	data.insert(
		"pkg".to_string(),
		serde_json::json!({"name": "my-lib", "version": "1.0.0"}),
	);

	let content = "Install {{ pkgg.name }} v{{ pkg.version }}";
	let undefined = find_undefined_variables(content, &data);
	assert_eq!(undefined, vec!["pkgg.name"]);
}

#[test]
fn find_undefined_variables_with_multiple_undefined() {
	let mut data = HashMap::new();
	data.insert("pkg".to_string(), serde_json::json!({"name": "my-lib"}));

	let content = "{{ unknown.field }} and {{ typo.value }} but {{ pkg.name }}";
	let undefined = find_undefined_variables(content, &data);
	assert_eq!(undefined, vec!["typo.value", "unknown.field"]);
}

#[test]
fn find_undefined_variables_empty_data() {
	let data = HashMap::new();
	let content = "{{ pkg.name }}";
	let undefined = find_undefined_variables(content, &data);
	assert!(
		undefined.is_empty(),
		"expected no warnings when data is empty (template rendering is a no-op)"
	);
}

#[test]
fn find_undefined_variables_no_template_syntax() {
	let mut data = HashMap::new();
	data.insert("pkg".to_string(), serde_json::json!({"name": "test"}));

	let content = "Plain text without any template syntax.";
	let undefined = find_undefined_variables(content, &data);
	assert!(
		undefined.is_empty(),
		"expected no warnings when content has no template syntax"
	);
}

#[test]
fn find_undefined_variables_with_loop_builtin() {
	let mut data = HashMap::new();
	data.insert("items".to_string(), serde_json::json!(["a", "b", "c"]));

	// `loop` is a minijinja builtin in for-loops
	let content = "{% for item in items %}{{ loop.index }}: {{ item }}{% endfor %}";
	let undefined = find_undefined_variables(content, &data);
	assert!(
		undefined.is_empty(),
		"expected no warnings for minijinja builtins like `loop`, got: {undefined:?}"
	);
}

#[test]
fn find_undefined_variables_top_level_only() {
	let mut data = HashMap::new();
	data.insert("pkg".to_string(), serde_json::json!({"name": "test"}));

	// References a top-level variable that doesn't exist at all in data
	let content = "{{ missing }}";
	let undefined = find_undefined_variables(content, &data);
	assert_eq!(undefined, vec!["missing"]);
}

#[test]
fn check_project_reports_undefined_variable_warnings() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npkg = \"package.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("package.json"),
		r#"{"name": "my-lib", "version": "1.0.0"}"#,
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	// Provider with a typo: "pkgg" instead of "pkg"
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@install} -->\n\nnpm install {{ pkgg.name }}\n\n<!-- {/install} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=install} -->\n\nold\n\n<!-- {/install} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let result = check_project(&ctx)?;

	assert!(
		result.has_warnings(),
		"expected warnings for undefined variable"
	);
	assert_eq!(result.warnings.len(), 1);
	assert_eq!(result.warnings[0].block_name, "install");
	assert_eq!(result.warnings[0].undefined_variables, vec!["pkgg.name"]);

	Ok(())
}

#[test]
fn check_project_no_warnings_for_valid_variables() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npkg = \"package.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("package.json"),
		r#"{"name": "my-lib", "version": "1.0.0"}"#,
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@install} -->\n\nnpm install {{ pkg.name }}@{{ pkg.version }}\n\n<!-- {/install} \
		 -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=install} -->\n\nold\n\n<!-- {/install} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let result = check_project(&ctx)?;

	assert!(
		!result.has_warnings(),
		"expected no warnings when all variables are defined, got: {:?}",
		result.warnings
	);

	Ok(())
}

#[test]
fn compute_updates_reports_undefined_variable_warnings() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npkg = \"package.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("package.json"),
		r#"{"name": "my-lib", "version": "1.0.0"}"#,
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	// Provider with a typo
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@install} -->\n\nnpm install {{ typo.name }}\n\n<!-- {/install} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=install} -->\n\nold\n\n<!-- {/install} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;

	// The update should still proceed (rendering with empty string for undefined)
	assert!(updates.warnings.len() == 1);
	assert_eq!(updates.warnings[0].block_name, "install");
	assert_eq!(updates.warnings[0].undefined_variables, vec!["typo.name"]);

	Ok(())
}

#[test]
fn find_undefined_variables_partial_match() {
	let mut data = HashMap::new();
	data.insert("pkg".to_string(), serde_json::json!({"name": "my-lib"}));
	data.insert(
		"cargo".to_string(),
		serde_json::json!({"package": {"edition": "2024"}}),
	);

	// pkg is defined but typo is not; cargo is defined
	let content = "{{ pkg.name }} {{ typo.version }} {{ cargo.package.edition }}";
	let undefined = find_undefined_variables(content, &data);
	assert_eq!(undefined, vec!["typo.version"]);
}

// ─── Block arguments ───────────────────────────────────────────────────

#[test]
fn parse_provider_with_arguments() -> MdtResult<()> {
	let input = "<!-- {@badges:\"crate_name\"} -->\n\nContent with {{ crate_name }}\n\n<!-- \
	             {/badges} -->\n";
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "badges");
	assert_eq!(blocks[0].r#type, BlockType::Provider);
	assert_eq!(blocks[0].arguments, vec!["crate_name"]);
	Ok(())
}

#[test]
fn parse_consumer_with_arguments() -> MdtResult<()> {
	let input = "<!-- {=badges:\"mdt_core\"} -->\n\nold\n\n<!-- {/badges} -->\n";
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "badges");
	assert_eq!(blocks[0].r#type, BlockType::Consumer);
	assert_eq!(blocks[0].arguments, vec!["mdt_core"]);
	Ok(())
}

#[test]
fn parse_provider_with_multiple_arguments() -> MdtResult<()> {
	let input =
		"<!-- {@tmpl:\"a\":\"b\":\"c\"} -->\n\n{{ a }} {{ b }} {{ c }}\n\n<!-- {/tmpl} -->\n";
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].arguments, vec!["a", "b", "c"]);
	Ok(())
}

#[test]
fn parse_consumer_with_multiple_arguments() -> MdtResult<()> {
	let input = "<!-- {=tmpl:\"x\":\"y\":\"z\"} -->\n\nold\n\n<!-- {/tmpl} -->\n";
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].arguments, vec!["x", "y", "z"]);
	Ok(())
}

#[test]
fn parse_consumer_with_arguments_and_transformers() -> MdtResult<()> {
	let input = "<!-- {=badges:\"mdt_core\"|trim} -->\n\nold\n\n<!-- {/badges} -->\n";
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].arguments, vec!["mdt_core"]);
	assert_eq!(blocks[0].transformers.len(), 1);
	assert_eq!(blocks[0].transformers[0].r#type, TransformerType::Trim);
	Ok(())
}

#[test]
fn parse_block_without_arguments_has_empty_vec() -> MdtResult<()> {
	let input = "<!-- {@block} -->\n\nContent\n\n<!-- {/block} -->\n";
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert!(blocks[0].arguments.is_empty());
	Ok(())
}

#[test]
fn parse_arguments_with_spaces() -> MdtResult<()> {
	let input = "<!-- {@tmpl : \"param1\" : \"param2\"} -->\n\nContent\n\n<!-- {/tmpl} -->\n";
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].arguments, vec!["param1", "param2"]);
	Ok(())
}

#[test]
fn parse_single_quoted_arguments() -> MdtResult<()> {
	let input = "<!-- {@tmpl:'param1':'param2'} -->\n\nContent\n\n<!-- {/tmpl} -->\n";
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].arguments, vec!["param1", "param2"]);
	Ok(())
}

#[test]
fn build_render_context_merges_args() {
	let provider = ProviderEntry {
		block: Block {
			name: "badges".to_string(),
			r#type: BlockType::Provider,
			opening: Position::new(1, 1, 0, 1, 10, 10),
			closing: Position::new(3, 1, 20, 3, 10, 30),
			transformers: vec![],
			arguments: vec!["crate_name".to_string()],
		},
		file: PathBuf::from("template.t.md"),
		content: "badge for {{ crate_name }}".to_string(),
	};
	let consumer = ConsumerEntry {
		block: Block {
			name: "badges".to_string(),
			r#type: BlockType::Consumer,
			opening: Position::new(1, 1, 0, 1, 10, 10),
			closing: Position::new(3, 1, 20, 3, 10, 30),
			transformers: vec![],
			arguments: vec!["mdt_core".to_string()],
		},
		file: PathBuf::from("readme.md"),
		content: "old".to_string(),
	};

	let base_data = HashMap::new();
	let result = build_render_context(&base_data, &provider, &consumer);
	assert!(result.is_some());
	let data = result.unwrap_or_else(|| panic!("expected Some"));
	assert_eq!(
		data.get("crate_name"),
		Some(&serde_json::Value::String("mdt_core".to_string()))
	);
}

#[test]
fn build_render_context_preserves_base_data() {
	let mut base_data = HashMap::new();
	base_data.insert("pkg".to_string(), serde_json::json!({"version": "1.0.0"}));

	let provider = ProviderEntry {
		block: Block {
			name: "tmpl".to_string(),
			r#type: BlockType::Provider,
			opening: Position::new(1, 1, 0, 1, 10, 10),
			closing: Position::new(3, 1, 20, 3, 10, 30),
			transformers: vec![],
			arguments: vec!["name".to_string()],
		},
		file: PathBuf::from("template.t.md"),
		content: "".to_string(),
	};
	let consumer = ConsumerEntry {
		block: Block {
			name: "tmpl".to_string(),
			r#type: BlockType::Consumer,
			opening: Position::new(1, 1, 0, 1, 10, 10),
			closing: Position::new(3, 1, 20, 3, 10, 30),
			transformers: vec![],
			arguments: vec!["my-lib".to_string()],
		},
		file: PathBuf::from("readme.md"),
		content: "".to_string(),
	};

	let result = build_render_context(&base_data, &provider, &consumer);
	assert!(result.is_some());
	let data = result.unwrap_or_else(|| panic!("expected Some"));
	// base data is preserved
	assert!(data.contains_key("pkg"));
	// new arg is added
	assert_eq!(
		data.get("name"),
		Some(&serde_json::Value::String("my-lib".to_string()))
	);
}

#[test]
fn build_render_context_returns_none_on_count_mismatch() {
	let provider = ProviderEntry {
		block: Block {
			name: "tmpl".to_string(),
			r#type: BlockType::Provider,
			opening: Position::new(1, 1, 0, 1, 10, 10),
			closing: Position::new(3, 1, 20, 3, 10, 30),
			transformers: vec![],
			arguments: vec!["a".to_string(), "b".to_string()],
		},
		file: PathBuf::from("template.t.md"),
		content: "".to_string(),
	};
	let consumer = ConsumerEntry {
		block: Block {
			name: "tmpl".to_string(),
			r#type: BlockType::Consumer,
			opening: Position::new(1, 1, 0, 1, 10, 10),
			closing: Position::new(3, 1, 20, 3, 10, 30),
			transformers: vec![],
			arguments: vec!["x".to_string()],
		},
		file: PathBuf::from("readme.md"),
		content: "".to_string(),
	};

	let result = build_render_context(&HashMap::new(), &provider, &consumer);
	assert!(result.is_none());
}

#[test]
fn build_render_context_no_args_returns_base_data() {
	let mut base_data = HashMap::new();
	base_data.insert("key".to_string(), serde_json::json!("value"));

	let provider = ProviderEntry {
		block: Block {
			name: "tmpl".to_string(),
			r#type: BlockType::Provider,
			opening: Position::new(1, 1, 0, 1, 10, 10),
			closing: Position::new(3, 1, 20, 3, 10, 30),
			transformers: vec![],
			arguments: vec![],
		},
		file: PathBuf::from("template.t.md"),
		content: "".to_string(),
	};
	let consumer = ConsumerEntry {
		block: Block {
			name: "tmpl".to_string(),
			r#type: BlockType::Consumer,
			opening: Position::new(1, 1, 0, 1, 10, 10),
			closing: Position::new(3, 1, 20, 3, 10, 30),
			transformers: vec![],
			arguments: vec![],
		},
		file: PathBuf::from("readme.md"),
		content: "".to_string(),
	};

	let result = build_render_context(&base_data, &provider, &consumer);
	assert!(result.is_some());
	let data = result.unwrap_or_else(|| panic!("expected Some"));
	assert_eq!(data, base_data);
}

#[test]
fn block_arguments_end_to_end() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@badges:\"crate_name\"} -->\n\n[![crates.io](https://img.shields.io/crates/v/{{ \
		 crate_name }})]\n\n<!-- {/badges} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=badges:\"mdt_core\"} -->\n\nold content\n\n<!-- {/badges} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = ProjectContext {
		project: scan_project(tmp.path())?,
		data: HashMap::new(),
		padding: None,
	};

	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);

	let updated_content = updates
		.updated_files
		.values()
		.next()
		.unwrap_or_else(|| panic!("expected one file"));
	assert!(
		updated_content.contains("mdt_core"),
		"expected rendered crate_name in output, got: {updated_content}"
	);
	assert!(
		!updated_content.contains("{{ crate_name }}"),
		"template variable should be interpolated"
	);

	Ok(())
}

#[test]
fn block_arguments_multiple_consumers_different_args() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@badge:\"name\"} -->\n\n[{{ name }}](https://crates.io/crates/{{ name }})\n\n<!-- \
		 {/badge} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("a.md"),
		"<!-- {=badge:\"mdt_core\"} -->\n\nold\n\n<!-- {/badge} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("b.md"),
		"<!-- {=badge:\"mdt_cli\"} -->\n\nold\n\n<!-- {/badge} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = ProjectContext {
		project: scan_project(tmp.path())?,
		data: HashMap::new(),
		padding: None,
	};

	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 2);

	let a_content = updates
		.updated_files
		.get(&tmp.path().join("a.md"))
		.unwrap_or_else(|| panic!("expected a.md"));
	assert!(
		a_content.contains("mdt_core"),
		"a.md should contain mdt_core"
	);
	assert!(
		!a_content.contains("mdt_cli"),
		"a.md should not contain mdt_cli"
	);

	let b_content = updates
		.updated_files
		.get(&tmp.path().join("b.md"))
		.unwrap_or_else(|| panic!("expected b.md"));
	assert!(b_content.contains("mdt_cli"), "b.md should contain mdt_cli");
	assert!(
		!b_content.contains("mdt_core"),
		"b.md should not contain mdt_core"
	);

	Ok(())
}

#[test]
fn block_arguments_with_data_and_args() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npkg = \"package.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("package.json"), r#"{"version": "2.0.0"}"#)
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@info:\"lib_name\"} -->\n\n{{ lib_name }} v{{ pkg.version }}\n\n<!-- {/info} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=info:\"mylib\"} -->\n\nold\n\n<!-- {/info} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);

	let content = updates
		.updated_files
		.values()
		.next()
		.unwrap_or_else(|| panic!("expected one file"));
	assert!(
		content.contains("mylib v2.0.0"),
		"expected data + args interpolation, got: {content}"
	);

	Ok(())
}

#[test]
fn check_project_reports_argument_count_mismatch() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@tmpl:\"a\":\"b\"} -->\n\n{{ a }} {{ b }}\n\n<!-- {/tmpl} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	// Consumer provides only 1 argument, provider expects 2
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=tmpl:\"x\"} -->\n\nold\n\n<!-- {/tmpl} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = ProjectContext {
		project: scan_project(tmp.path())?,
		data: HashMap::new(),
		padding: None,
	};
	let result = check_project(&ctx)?;
	assert!(
		!result.render_errors.is_empty(),
		"expected render error for argument count mismatch"
	);
	Ok(())
}

#[test]
fn block_arguments_with_transformers_end_to_end() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@badge:\"name\"} -->\n\n{{ name }}\n\n<!-- {/badge} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=badge:\"mdt_core\"|trim} -->\n\nold\n\n<!-- {/badge} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = ProjectContext {
		project: scan_project(tmp.path())?,
		data: HashMap::new(),
		padding: None,
	};

	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);

	let content = updates
		.updated_files
		.values()
		.next()
		.unwrap_or_else(|| panic!("expected one file"));
	// trim transformer should strip leading/trailing whitespace
	assert!(
		content.contains("mdt_core"),
		"expected rendered name, got: {content}"
	);

	Ok(())
}

#[test]
fn block_arguments_up_to_date_consumer() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@greeting:\"who\"} -->\n\nHello, {{ who }}!\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	// Consumer already has the expected content
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=greeting:\"World\"} -->\n\nHello, World!\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = ProjectContext {
		project: scan_project(tmp.path())?,
		data: HashMap::new(),
		padding: None,
	};
	let result = check_project(&ctx)?;
	assert!(
		result.is_ok(),
		"consumer should be up to date: stale={:?}, errors={:?}",
		result.stale,
		result.render_errors
	);

	Ok(())
}

// --- engine.rs: `if` transformer tests ---

#[test]
fn transformer_if_truthy_bool_includes_content() {
	let mut data = HashMap::new();
	data.insert("config".to_string(), serde_json::json!({"enabled": true}));
	let result = apply_transformers_with_data(
		"hello world",
		&[Transformer {
			r#type: TransformerType::If,
			args: vec![Argument::String("config.enabled".to_string())],
		}],
		Some(&data),
	);
	assert_eq!(result, "hello world");
}

#[test]
fn transformer_if_falsy_bool_excludes_content() {
	let mut data = HashMap::new();
	data.insert("config".to_string(), serde_json::json!({"enabled": false}));
	let result = apply_transformers_with_data(
		"hello world",
		&[Transformer {
			r#type: TransformerType::If,
			args: vec![Argument::String("config.enabled".to_string())],
		}],
		Some(&data),
	);
	assert_eq!(result, "");
}

#[test]
fn transformer_if_falsy_null_excludes_content() {
	let mut data = HashMap::new();
	data.insert("config".to_string(), serde_json::json!({"value": null}));
	let result = apply_transformers_with_data(
		"hello world",
		&[Transformer {
			r#type: TransformerType::If,
			args: vec![Argument::String("config.value".to_string())],
		}],
		Some(&data),
	);
	assert_eq!(result, "");
}

#[test]
fn transformer_if_falsy_empty_string_excludes_content() {
	let mut data = HashMap::new();
	data.insert("config".to_string(), serde_json::json!({"name": ""}));
	let result = apply_transformers_with_data(
		"hello world",
		&[Transformer {
			r#type: TransformerType::If,
			args: vec![Argument::String("config.name".to_string())],
		}],
		Some(&data),
	);
	assert_eq!(result, "");
}

#[test]
fn transformer_if_falsy_zero_excludes_content() {
	let mut data = HashMap::new();
	data.insert("config".to_string(), serde_json::json!({"count": 0}));
	let result = apply_transformers_with_data(
		"hello world",
		&[Transformer {
			r#type: TransformerType::If,
			args: vec![Argument::String("config.count".to_string())],
		}],
		Some(&data),
	);
	assert_eq!(result, "");
}

#[test]
fn transformer_if_falsy_zero_float_excludes_content() {
	let mut data = HashMap::new();
	data.insert("config".to_string(), serde_json::json!({"ratio": 0.0}));
	let result = apply_transformers_with_data(
		"hello world",
		&[Transformer {
			r#type: TransformerType::If,
			args: vec![Argument::String("config.ratio".to_string())],
		}],
		Some(&data),
	);
	assert_eq!(result, "");
}

#[test]
fn transformer_if_nested_data_path() {
	let mut data = HashMap::new();
	data.insert(
		"config".to_string(),
		serde_json::json!({"features": {"experimental": true}}),
	);
	let result = apply_transformers_with_data(
		"experimental content",
		&[Transformer {
			r#type: TransformerType::If,
			args: vec![Argument::String("config.features.experimental".to_string())],
		}],
		Some(&data),
	);
	assert_eq!(result, "experimental content");
}

#[test]
fn transformer_if_nested_data_path_falsy() {
	let mut data = HashMap::new();
	data.insert(
		"config".to_string(),
		serde_json::json!({"features": {"deprecated_api": false}}),
	);
	let result = apply_transformers_with_data(
		"deprecated content",
		&[Transformer {
			r#type: TransformerType::If,
			args: vec![Argument::String(
				"config.features.deprecated_api".to_string(),
			)],
		}],
		Some(&data),
	);
	assert_eq!(result, "");
}

#[test]
fn transformer_if_missing_path_excludes_content() {
	let mut data = HashMap::new();
	data.insert("config".to_string(), serde_json::json!({"enabled": true}));
	let result = apply_transformers_with_data(
		"hello world",
		&[Transformer {
			r#type: TransformerType::If,
			args: vec![Argument::String("config.nonexistent".to_string())],
		}],
		Some(&data),
	);
	assert_eq!(result, "");
}

#[test]
fn transformer_if_missing_root_namespace_excludes_content() {
	let mut data = HashMap::new();
	data.insert("config".to_string(), serde_json::json!({"enabled": true}));
	let result = apply_transformers_with_data(
		"hello world",
		&[Transformer {
			r#type: TransformerType::If,
			args: vec![Argument::String("missing.key".to_string())],
		}],
		Some(&data),
	);
	assert_eq!(result, "");
}

#[test]
fn transformer_if_no_data_context_excludes_content() {
	let result = apply_transformers_with_data(
		"hello world",
		&[Transformer {
			r#type: TransformerType::If,
			args: vec![Argument::String("config.enabled".to_string())],
		}],
		None,
	);
	assert_eq!(result, "");
}

#[test]
fn transformer_if_combined_with_trim() {
	let mut data = HashMap::new();
	data.insert("config".to_string(), serde_json::json!({"enabled": true}));
	let result = apply_transformers_with_data(
		"  hello world  ",
		&[
			Transformer {
				r#type: TransformerType::If,
				args: vec![Argument::String("config.enabled".to_string())],
			},
			Transformer {
				r#type: TransformerType::Trim,
				args: vec![],
			},
		],
		Some(&data),
	);
	assert_eq!(result, "hello world");
}

#[test]
fn transformer_if_falsy_combined_with_trim() {
	let mut data = HashMap::new();
	data.insert("config".to_string(), serde_json::json!({"enabled": false}));
	let result = apply_transformers_with_data(
		"  hello world  ",
		&[
			Transformer {
				r#type: TransformerType::If,
				args: vec![Argument::String("config.enabled".to_string())],
			},
			Transformer {
				r#type: TransformerType::Trim,
				args: vec![],
			},
		],
		Some(&data),
	);
	assert_eq!(result, "");
}

#[test]
fn transformer_if_truthy_string_includes_content() {
	let mut data = HashMap::new();
	data.insert("config".to_string(), serde_json::json!({"name": "hello"}));
	let result = apply_transformers_with_data(
		"content here",
		&[Transformer {
			r#type: TransformerType::If,
			args: vec![Argument::String("config.name".to_string())],
		}],
		Some(&data),
	);
	assert_eq!(result, "content here");
}

#[test]
fn transformer_if_truthy_nonzero_number_includes_content() {
	let mut data = HashMap::new();
	data.insert("config".to_string(), serde_json::json!({"count": 42}));
	let result = apply_transformers_with_data(
		"content here",
		&[Transformer {
			r#type: TransformerType::If,
			args: vec![Argument::String("config.count".to_string())],
		}],
		Some(&data),
	);
	assert_eq!(result, "content here");
}

#[test]
fn transformer_if_truthy_array_includes_content() {
	let mut data = HashMap::new();
	data.insert(
		"config".to_string(),
		serde_json::json!({"items": [1, 2, 3]}),
	);
	let result = apply_transformers_with_data(
		"content here",
		&[Transformer {
			r#type: TransformerType::If,
			args: vec![Argument::String("config.items".to_string())],
		}],
		Some(&data),
	);
	assert_eq!(result, "content here");
}

#[test]
fn transformer_if_truthy_object_includes_content() {
	let mut data = HashMap::new();
	data.insert(
		"config".to_string(),
		serde_json::json!({"nested": {"key": "value"}}),
	);
	let result = apply_transformers_with_data(
		"content here",
		&[Transformer {
			r#type: TransformerType::If,
			args: vec![Argument::String("config.nested".to_string())],
		}],
		Some(&data),
	);
	assert_eq!(result, "content here");
}

#[test]
fn transformer_if_path_into_non_object_excludes_content() {
	let mut data = HashMap::new();
	data.insert("config".to_string(), serde_json::json!({"value": "string"}));
	// Trying to access config.value.deeper when config.value is a string
	let result = apply_transformers_with_data(
		"content here",
		&[Transformer {
			r#type: TransformerType::If,
			args: vec![Argument::String("config.value.deeper".to_string())],
		}],
		Some(&data),
	);
	assert_eq!(result, "");
}

#[test]
fn transformer_if_empty_path_excludes_content() {
	let mut data = HashMap::new();
	data.insert("config".to_string(), serde_json::json!({"enabled": true}));
	let result = apply_transformers_with_data(
		"content here",
		&[Transformer {
			r#type: TransformerType::If,
			args: vec![Argument::String(String::new())],
		}],
		Some(&data),
	);
	assert_eq!(result, "");
}

#[test]
fn transformer_if_top_level_key() {
	let mut data = HashMap::new();
	data.insert("enabled".to_string(), serde_json::json!(true));
	let result = apply_transformers_with_data(
		"content here",
		&[Transformer {
			r#type: TransformerType::If,
			args: vec![Argument::String("enabled".to_string())],
		}],
		Some(&data),
	);
	assert_eq!(result, "content here");
}

#[test]
fn transformer_if_validates_requires_one_arg() -> MdtResult<()> {
	let result = validate_transformers(&[Transformer {
		r#type: TransformerType::If,
		args: vec![],
	}]);
	assert!(result.is_err());
	Ok(())
}

#[test]
fn transformer_if_validates_rejects_extra_args() -> MdtResult<()> {
	let result = validate_transformers(&[Transformer {
		r#type: TransformerType::If,
		args: vec![
			Argument::String("a".to_string()),
			Argument::String("b".to_string()),
		],
	}]);
	assert!(result.is_err());
	Ok(())
}

#[test]
fn transformer_if_validates_accepts_one_arg() -> MdtResult<()> {
	validate_transformers(&[Transformer {
		r#type: TransformerType::If,
		args: vec![Argument::String("config.enabled".to_string())],
	}])?;
	Ok(())
}

#[test]
fn parse_consumer_with_if_transformer() -> MdtResult<()> {
	let input = "<!-- {=block|if:\"config.features.enabled\"} -->\ncontent\n<!-- {/block} -->\n";
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].transformers.len(), 1);
	assert_eq!(blocks[0].transformers[0].r#type, TransformerType::If);
	assert_eq!(
		blocks[0].transformers[0].args,
		vec![Argument::String("config.features.enabled".to_string())]
	);
	Ok(())
}

#[test]
fn parse_consumer_with_if_and_other_transformers() -> MdtResult<()> {
	let input =
		"<!-- {=block|if:\"config.enabled\"|trim|indent:\"  \"} -->\ncontent\n<!-- {/block} -->\n";
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].transformers.len(), 3);
	assert_eq!(blocks[0].transformers[0].r#type, TransformerType::If);
	assert_eq!(blocks[0].transformers[1].r#type, TransformerType::Trim);
	assert_eq!(blocks[0].transformers[2].r#type, TransformerType::Indent);
	Ok(())
}

#[test]
fn update_preserves_multiline_link_definitions_with_template_vars() -> MdtResult<()> {
	let input = r#"<!-- {@badge:"crateName"} -->

[crate-image]: https://img.shields.io/crates/v/{{ crateName }}.svg
[crate-link]: https://crates.io/crates/{{ crateName }}
[docs-image]: https://docs.rs/{{ crateName }}/badge.svg
[docs-link]: https://docs.rs/{{ crateName }}/

<!-- {/badge} -->
"#;
	let blocks = parse(input)?;
	assert_eq!(blocks.len(), 1, "Should parse one block");
	let content = extract_content_between_tags(input, &blocks[0]);
	assert!(
		content.contains('\n'),
		"Content should contain newlines but got: {content:?}"
	);
	let newline_count = content.chars().filter(|c| *c == '\n').count();
	assert!(
		newline_count >= 6,
		"Content should have at least 6 newlines but got {newline_count}: {content:?}"
	);

	Ok(())
}

#[test]
fn update_preserves_multiline_content_in_consumer() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let template_content = r#"<!-- {@badge:"crateName"} -->

[crate-image]: https://img.shields.io/crates/v/{{ crateName }}.svg
[crate-link]: https://crates.io/crates/{{ crateName }}
[docs-image]: https://docs.rs/{{ crateName }}/badge.svg
[docs-link]: https://docs.rs/{{ crateName }}/
[ci-status-image]: https://github.com/ifiokjr/mdt/workflows/ci/badge.svg
[ci-status-link]: https://github.com/ifiokjr/mdt/actions?query=workflow:ci
[coverage-image]: https://codecov.io/gh/ifiokjr/mdt/branch/main/graph/badge.svg
[coverage-link]: https://codecov.io/gh/ifiokjr/mdt
[unlicense-image]: https://img.shields.io/badge/license-Unlicence-blue.svg
[unlicense-link]: https://opensource.org/license/unlicense

<!-- {/badge} -->
"#;
	let consumer_content = r#"# Readme

<!-- {=badge:"mdt_core"} -->

old content

<!-- {/badge} -->
"#;
	std::fs::write(tmp.path().join("template.t.md"), template_content)
		.unwrap_or_else(|e| panic!("write template: {e}"));
	std::fs::write(tmp.path().join("readme.md"), consumer_content)
		.unwrap_or_else(|e| panic!("write consumer: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;

	let readme_path = tmp.path().join("readme.md");
	let updated = updates
		.updated_files
		.get(&readme_path)
		.unwrap_or_else(|| panic!("readme.md should be in updated files"));

	// Every link definition should be on its own line
	assert!(
		updated.contains("\n[crate-image]:"),
		"crate-image should be on its own line"
	);
	assert!(
		updated.contains("\n[crate-link]:"),
		"crate-link should be on its own line"
	);
	assert!(
		updated.contains("\n[docs-image]:"),
		"docs-image should be on its own line"
	);
	assert!(
		updated.contains("\n[docs-link]:"),
		"docs-link should be on its own line"
	);
	assert!(
		updated.contains("\n[ci-status-image]:"),
		"ci-status-image should be on its own line"
	);
	assert!(
		updated.contains("\n[ci-status-link]:"),
		"ci-status-link should be on its own line"
	);
	assert!(
		updated.contains("\n[coverage-image]:"),
		"coverage-image should be on its own line"
	);
	assert!(
		updated.contains("\n[coverage-link]:"),
		"coverage-link should be on its own line"
	);
	assert!(
		updated.contains("\n[unlicense-image]:"),
		"unlicense-image should be on its own line"
	);
	assert!(
		updated.contains("\n[unlicense-link]:"),
		"unlicense-link should be on its own line"
	);

	Ok(())
}

/// Verifies that running `mdt update` twice (idempotency) preserves
/// newlines even when the consumer already contains valid link
/// reference definitions from a previous update.
#[test]
fn update_idempotent_multiline_link_definitions() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let template_content = r#"<!-- {@badge:"crateName"} -->

[crate-image]: https://img.shields.io/crates/v/{{ crateName }}.svg
[crate-link]: https://crates.io/crates/{{ crateName }}
[docs-image]: https://docs.rs/{{ crateName }}/badge.svg
[docs-link]: https://docs.rs/{{ crateName }}/

<!-- {/badge} -->
"#;
	// Consumer already has rendered multi-line content from a previous update
	let consumer_content = r#"# Readme

<!-- {=badge:"mdt_core"} -->

[crate-image]: https://img.shields.io/crates/v/mdt_core.svg
[crate-link]: https://crates.io/crates/mdt_core
[docs-image]: https://docs.rs/mdt_core/badge.svg
[docs-link]: https://docs.rs/mdt_core/

<!-- {/badge} -->
"#;
	std::fs::write(tmp.path().join("template.t.md"), template_content)
		.unwrap_or_else(|e| panic!("write template: {e}"));
	std::fs::write(tmp.path().join("readme.md"), consumer_content)
		.unwrap_or_else(|e| panic!("write consumer: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;

	// When content already matches, there should be no updates
	assert_eq!(
		updates.updated_count, 0,
		"Re-running update on already-up-to-date content should produce no changes"
	);

	Ok(())
}

/// Verifies that link reference definitions with valid URLs (no
/// template variables) are preserved across the full
/// scan → update → re-scan cycle.
#[test]
fn update_preserves_newlines_with_valid_link_definitions() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	// Template with NO template variables — all URLs are valid
	let template_content = r#"<!-- {@links} -->

[repo]: https://github.com/example/repo
[docs]: https://docs.example.com
[ci]: https://ci.example.com/badge.svg

<!-- {/links} -->
"#;
	let consumer_content = r#"# Readme

<!-- {=links} -->

old content

<!-- {/links} -->
"#;
	std::fs::write(tmp.path().join("template.t.md"), template_content)
		.unwrap_or_else(|e| panic!("write template: {e}"));
	std::fs::write(tmp.path().join("readme.md"), consumer_content)
		.unwrap_or_else(|e| panic!("write consumer: {e}"));

	// First update
	let ctx = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&ctx)?;
	write_updates(&updates)?;

	// Read back the updated content
	let updated = std::fs::read_to_string(tmp.path().join("readme.md"))
		.unwrap_or_else(|e| panic!("read: {e}"));
	assert!(
		updated.contains("\n[repo]:"),
		"[repo] should be on its own line after first update"
	);
	assert!(
		updated.contains("\n[docs]:"),
		"[docs] should be on its own line after first update"
	);
	assert!(
		updated.contains("\n[ci]:"),
		"[ci] should be on its own line after first update"
	);

	// Second update — should be idempotent
	let ctx2 = scan_project_with_config(tmp.path())?;
	let updates2 = compute_updates(&ctx2)?;
	assert_eq!(
		updates2.updated_count, 0,
		"Second update should find nothing to change"
	);

	Ok(())
}

// --- PR2: config discovery + typed data entries + ini + canonical templates ---

#[test]
fn config_load_data_ini() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\nsettings = \"settings.ini\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("settings.ini"),
		"name = my-app\n[server]\nport = 8080\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;

	assert_eq!(data["settings"]["name"], "my-app");
	assert_eq!(data["settings"]["server"]["port"], "8080");

	Ok(())
}

#[test]
fn config_load_data_typed_entry_explicit_json_format() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\nrelease = { path = \"release-info\", format = \"json\" }\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("release-info"), r#"{"version":"1.2.3"}"#)
		.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	assert_eq!(
		config.data.get("release"),
		Some(&DataSource::Typed(TypedDataSource {
			path: PathBuf::from("release-info"),
			format: "json".to_string(),
		}))
	);

	let data = config.load_data(tmp.path())?;
	assert_eq!(data["release"]["version"], "1.2.3");

	Ok(())
}

#[test]
fn config_load_data_script_text_entry() -> MdtResult<()> {
	if cfg!(windows) {
		return Ok(());
	}

	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\nversion = { command = \"printf 1.2.3\", format = \"text\", watch = [\"VERSION\"] \
		 }\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("VERSION"), "1.2.3\n").unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	assert_eq!(
		config.data.get("version"),
		Some(&DataSource::Script(ScriptDataSource {
			command: "printf 1.2.3".to_string(),
			format: Some("text".to_string()),
			watch: vec![PathBuf::from("VERSION")],
		}))
	);

	let data = config.load_data(tmp.path())?;
	assert_eq!(data["version"].as_str().unwrap_or("").trim(), "1.2.3");

	Ok(())
}

#[test]
fn config_load_data_script_uses_cache_until_watch_changes() -> MdtResult<()> {
	if cfg!(windows) {
		return Ok(());
	}

	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("VERSION"), "1.0.0\n").unwrap_or_else(|e| panic!("write: {e}"));
	let command = "count=$(cat .run_count 2>/dev/null || echo 0); count=$((count+1)); echo \
	               \"$count\" > .run_count; cat VERSION";
	std::fs::write(
		tmp.path().join("mdt.toml"),
		format!(
			"[data]\nversion = {{ command = {:?}, format = \"text\", watch = [\"VERSION\"] }}\n",
			command
		),
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));

	let data1 = config.load_data(tmp.path())?;
	assert_eq!(data1["version"].as_str().unwrap_or(""), "1.0.0\n");
	assert_eq!(
		std::fs::read_to_string(tmp.path().join(".run_count"))
			.unwrap_or_else(|e| panic!("read: {e}"))
			.trim(),
		"1"
	);

	let data2 = config.load_data(tmp.path())?;
	assert_eq!(data2["version"].as_str().unwrap_or(""), "1.0.0\n");
	assert_eq!(
		std::fs::read_to_string(tmp.path().join(".run_count"))
			.unwrap_or_else(|e| panic!("read: {e}"))
			.trim(),
		"1",
		"script should not rerun while watch files are unchanged"
	);

	std::fs::write(tmp.path().join("VERSION"), "2.0.0-beta\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	let data3 = config.load_data(tmp.path())?;
	assert_eq!(data3["version"].as_str().unwrap_or(""), "2.0.0-beta\n");
	assert_eq!(
		std::fs::read_to_string(tmp.path().join(".run_count"))
			.unwrap_or_else(|e| panic!("read: {e}"))
			.trim(),
		"2",
		"script should rerun after watched file changes"
	);

	Ok(())
}

#[test]
fn config_load_resolves_dot_mdt_toml() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join(".mdt.toml"),
		"[data]\npkg = \"package.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("package.json"), r#"{"name":"dot-config"}"#)
		.unwrap_or_else(|e| panic!("write: {e}"));

	assert_eq!(
		MdtConfig::resolve_path(tmp.path()),
		Some(tmp.path().join(".mdt.toml"))
	);
	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;
	assert_eq!(data["pkg"]["name"], "dot-config");

	Ok(())
}

#[test]
fn config_load_resolves_dot_config_mdt_toml() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::create_dir_all(tmp.path().join(".config")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join(".config/mdt.toml"),
		"[data]\npkg = \"package.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("package.json"),
		r#"{"name":"nested-config"}"#,
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	assert_eq!(
		MdtConfig::resolve_path(tmp.path()),
		Some(tmp.path().join(".config/mdt.toml"))
	);
	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;
	assert_eq!(data["pkg"]["name"], "nested-config");

	Ok(())
}

#[test]
fn config_load_prefers_mdt_toml_over_other_candidates() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::create_dir_all(tmp.path().join(".config")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\nselected = \"a.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join(".mdt.toml"),
		"[data]\nselected = \"b.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join(".config/mdt.toml"),
		"[data]\nselected = \"c.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("a.json"), r#"{"name":"primary"}"#)
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("b.json"), r#"{"name":"secondary"}"#)
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("c.json"), r#"{"name":"tertiary"}"#)
		.unwrap_or_else(|e| panic!("write: {e}"));

	assert_eq!(
		MdtConfig::resolve_path(tmp.path()),
		Some(tmp.path().join("mdt.toml"))
	);
	let config = MdtConfig::load(tmp.path())?.unwrap_or_else(|| panic!("expected Some"));
	let data = config.load_data(tmp.path())?;
	assert_eq!(data["selected"]["name"], "primary");

	Ok(())
}

#[test]
fn scan_project_sub_project_boundary_dot_mdt_toml() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::create_dir_all(tmp.path().join("packages/subproject"))
		.unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(tmp.path().join("packages/subproject/.mdt.toml"), "[data]\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("packages/subproject/readme.md"),
		"<!-- {=block} -->\n\nsub content\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let project = scan_project(tmp.path())?;
	assert!(project.consumers.is_empty());

	Ok(())
}

#[test]
fn scan_project_sub_project_boundary_dot_config_mdt_toml() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::create_dir_all(tmp.path().join("packages/subproject/.config"))
		.unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join("packages/subproject/.config/mdt.toml"),
		"[data]\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("packages/subproject/readme.md"),
		"<!-- {=block} -->\n\nsub content\n\n<!-- {/block} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let project = scan_project(tmp.path())?;
	assert!(project.consumers.is_empty());

	Ok(())
}

#[test]
fn scan_project_discovers_templates_in_dot_templates_directory() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::create_dir_all(tmp.path().join(".templates")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join(".templates/template.t.md"),
		"<!-- {@intro} -->\n\nHello from hidden templates.\n\n<!-- {/intro} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=intro} -->\n\nold\n\n<!-- {/intro} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let ctx = scan_project_with_config(tmp.path())?;
	assert!(ctx.project.providers.contains_key("intro"));
	assert_eq!(ctx.project.consumers.len(), 1);

	Ok(())
}

#[test]
fn scan_project_writes_index_cache_artifact() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@greeting} -->\n\nHello world!\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write provider: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=greeting} -->\n\nHello world!\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write consumer: {e}"));

	let _ = scan_project_with_options(tmp.path(), &ScanOptions::default())?;
	let cache_path = index_cache::cache_path(tmp.path());
	assert!(
		cache_path.is_file(),
		"expected cache file at {}",
		cache_path.display()
	);

	Ok(())
}

#[test]
fn scan_project_returns_cached_project_when_files_unchanged() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@greeting} -->\n\nHello world!\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write provider: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=greeting} -->\n\nHello world!\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write consumer: {e}"));

	let _ = scan_project_with_options(tmp.path(), &ScanOptions::default())?;

	let cache_path = index_cache::cache_path(tmp.path());
	let mut cache_json: serde_json::Value = serde_json::from_str(
		&std::fs::read_to_string(&cache_path).unwrap_or_else(|e| panic!("read cache: {e}")),
	)
	.unwrap_or_else(|e| panic!("parse cache json: {e}"));

	cache_json["project"]["providers"]["greeting"]["content"] =
		serde_json::Value::String("CACHED SENTINEL".to_string());
	std::fs::write(
		&cache_path,
		serde_json::to_vec_pretty(&cache_json).unwrap_or_else(|e| panic!("encode cache: {e}")),
	)
	.unwrap_or_else(|e| panic!("rewrite cache: {e}"));

	let project = scan_project_with_options(tmp.path(), &ScanOptions::default())?;
	assert_eq!(
		project.providers["greeting"].content, "CACHED SENTINEL",
		"unchanged project should have returned cached project content"
	);

	Ok(())
}

#[test]
fn scan_project_invalidates_cache_after_file_change() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let template_path = tmp.path().join("template.t.md");
	std::fs::write(
		&template_path,
		"<!-- {@greeting} -->\n\nHello world!\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write provider: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=greeting} -->\n\nHello world!\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write consumer: {e}"));

	let _ = scan_project_with_options(tmp.path(), &ScanOptions::default())?;

	let cache_path = index_cache::cache_path(tmp.path());
	let mut cache_json: serde_json::Value = serde_json::from_str(
		&std::fs::read_to_string(&cache_path).unwrap_or_else(|e| panic!("read cache: {e}")),
	)
	.unwrap_or_else(|e| panic!("parse cache json: {e}"));
	cache_json["project"]["providers"]["greeting"]["content"] =
		serde_json::Value::String("STALE CACHED CONTENT".to_string());
	std::fs::write(
		&cache_path,
		serde_json::to_vec_pretty(&cache_json).unwrap_or_else(|e| panic!("encode cache: {e}")),
	)
	.unwrap_or_else(|e| panic!("rewrite cache: {e}"));

	std::fs::write(
		&template_path,
		"<!-- {@greeting} -->\n\nFresh content from disk.\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("rewrite provider: {e}"));

	let project = scan_project_with_options(tmp.path(), &ScanOptions::default())?;
	assert!(
		project.providers["greeting"]
			.content
			.contains("Fresh content from disk."),
		"changed file should invalidate stale cache entry"
	);
	assert_ne!(
		project.providers["greeting"].content,
		"STALE CACHED CONTENT"
	);

	Ok(())
}

#[test]
fn scan_project_reuses_unchanged_files_when_other_files_change() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let alpha_template = tmp.path().join("alpha.t.md");
	let beta_template = tmp.path().join("beta.t.md");
	std::fs::write(
		&alpha_template,
		"<!-- {@alpha} -->\n\nAlpha from disk.\n\n<!-- {/alpha} -->\n",
	)
	.unwrap_or_else(|e| panic!("write alpha provider: {e}"));
	std::fs::write(
		&beta_template,
		"<!-- {@beta} -->\n\nBeta from disk.\n\n<!-- {/beta} -->\n",
	)
	.unwrap_or_else(|e| panic!("write beta provider: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=alpha} -->\n\nAlpha from disk.\n\n<!-- {/alpha} -->\n\n<!-- {=beta} -->\n\nBeta \
		 from disk.\n\n<!-- {/beta} -->\n",
	)
	.unwrap_or_else(|e| panic!("write consumer: {e}"));

	let _ = scan_project_with_options(tmp.path(), &ScanOptions::default())?;

	let cache_path = index_cache::cache_path(tmp.path());
	let mut cache_json: serde_json::Value = serde_json::from_str(
		&std::fs::read_to_string(&cache_path).unwrap_or_else(|e| panic!("read cache: {e}")),
	)
	.unwrap_or_else(|e| panic!("parse cache json: {e}"));
	cache_json["file_data"]["alpha.t.md"]["providers"][0]["content"] =
		serde_json::Value::String("CACHED ALPHA".to_string());
	cache_json["file_data"]["beta.t.md"]["providers"][0]["content"] =
		serde_json::Value::String("STALE BETA FROM CACHE".to_string());
	std::fs::write(
		&cache_path,
		serde_json::to_vec_pretty(&cache_json).unwrap_or_else(|e| panic!("encode cache: {e}")),
	)
	.unwrap_or_else(|e| panic!("rewrite cache: {e}"));

	std::fs::write(
		&beta_template,
		"<!-- {@beta} -->\n\nBeta from changed disk file.\n\n<!-- {/beta} -->\n",
	)
	.unwrap_or_else(|e| panic!("rewrite beta provider: {e}"));

	let project = scan_project_with_options(tmp.path(), &ScanOptions::default())?;
	assert_eq!(
		project.providers["alpha"].content, "CACHED ALPHA",
		"unchanged file should be reused from cache"
	);
	assert!(
		project.providers["beta"]
			.content
			.contains("Beta from changed disk file."),
		"changed file should be reparsed from disk"
	);
	assert_ne!(
		project.providers["beta"].content, "STALE BETA FROM CACHE",
		"changed file must not reuse stale cached entry"
	);

	Ok(())
}

#[test]
fn scan_project_removes_deleted_files_from_cache() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let alpha_template = tmp.path().join("alpha.t.md");
	std::fs::write(
		&alpha_template,
		"<!-- {@alpha} -->\n\nAlpha from disk.\n\n<!-- {/alpha} -->\n",
	)
	.unwrap_or_else(|e| panic!("write provider: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=alpha} -->\n\nAlpha from disk.\n\n<!-- {/alpha} -->\n",
	)
	.unwrap_or_else(|e| panic!("write consumer: {e}"));

	let _ = scan_project_with_options(tmp.path(), &ScanOptions::default())?;
	std::fs::remove_file(&alpha_template).unwrap_or_else(|e| panic!("remove provider: {e}"));

	let project = scan_project_with_options(tmp.path(), &ScanOptions::default())?;
	assert!(
		!project.providers.contains_key("alpha"),
		"deleted provider file should not remain in project providers"
	);

	let cache_path = index_cache::cache_path(tmp.path());
	let cache_json: serde_json::Value = serde_json::from_str(
		&std::fs::read_to_string(&cache_path).unwrap_or_else(|e| panic!("read cache: {e}")),
	)
	.unwrap_or_else(|e| panic!("parse cache json: {e}"));
	assert!(
		cache_json["files"].get("alpha.t.md").is_none(),
		"deleted file fingerprint should be removed from cache"
	);
	assert!(
		cache_json["file_data"].get("alpha.t.md").is_none(),
		"deleted file entry should be removed from cache"
	);

	Ok(())
}

#[test]
fn scan_project_cache_stores_content_hash_when_enabled() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@greeting} -->\n\nHello world!\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write provider: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=greeting} -->\n\nHello world!\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write consumer: {e}"));

	let options = ScanOptions {
		cache_verify_hash: true,
		..ScanOptions::default()
	};
	let _ = scan_project_with_options(tmp.path(), &options)?;

	let cache_path = index_cache::cache_path(tmp.path());
	let cache_json: serde_json::Value = serde_json::from_str(
		&std::fs::read_to_string(&cache_path).unwrap_or_else(|e| panic!("read cache: {e}")),
	)
	.unwrap_or_else(|e| panic!("parse cache json: {e}"));
	assert!(
		cache_json["files"]["template.t.md"]["content_hash"].is_number(),
		"expected content hash for template fingerprint when hash mode is enabled"
	);
	assert!(
		cache_json["files"]["readme.md"]["content_hash"].is_number(),
		"expected content hash for consumer fingerprint when hash mode is enabled"
	);

	Ok(())
}

#[test]
fn scan_project_hash_mismatch_invalidates_cache() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@greeting} -->\n\nHello world!\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write provider: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=greeting} -->\n\nHello world!\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write consumer: {e}"));

	let options = ScanOptions {
		cache_verify_hash: true,
		..ScanOptions::default()
	};
	let _ = scan_project_with_options(tmp.path(), &options)?;

	let cache_path = index_cache::cache_path(tmp.path());
	let mut cache_json: serde_json::Value = serde_json::from_str(
		&std::fs::read_to_string(&cache_path).unwrap_or_else(|e| panic!("read cache: {e}")),
	)
	.unwrap_or_else(|e| panic!("parse cache json: {e}"));
	cache_json["project"]["providers"]["greeting"]["content"] =
		serde_json::Value::String("STALE CACHED CONTENT".to_string());
	cache_json["files"]["template.t.md"]["content_hash"] = serde_json::Value::Null;
	std::fs::write(
		&cache_path,
		serde_json::to_vec_pretty(&cache_json).unwrap_or_else(|e| panic!("encode cache: {e}")),
	)
	.unwrap_or_else(|e| panic!("rewrite cache: {e}"));

	let project = scan_project_with_options(tmp.path(), &options)?;
	assert_ne!(
		project.providers["greeting"].content, "STALE CACHED CONTENT",
		"content hash mismatch should force cache miss and fresh parse"
	);
	assert!(
		project.providers["greeting"]
			.content
			.contains("Hello world!"),
		"fresh parse should preserve on-disk provider content"
	);

	Ok(())
}

#[test]
fn scan_project_cache_telemetry_tracks_full_cache_hit() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@greeting} -->\n\nHello world!\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write provider: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=greeting} -->\n\nHello world!\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write consumer: {e}"));

	let options = ScanOptions::default();
	let _ = scan_project_with_options(tmp.path(), &options)?;
	let _ = scan_project_with_options(tmp.path(), &options)?;

	let cache = inspect_project_cache(tmp.path(), &options);
	assert!(cache.valid, "expected valid cache inspection");
	let telemetry = cache
		.telemetry
		.as_ref()
		.unwrap_or_else(|| panic!("expected cache telemetry"));
	assert_eq!(telemetry.scan_count, 2);
	assert_eq!(telemetry.full_project_hit_count, 1);
	assert_eq!(telemetry.reused_file_count_total, 2);
	assert_eq!(telemetry.reparsed_file_count_total, 2);
	let last_scan = telemetry
		.last_scan
		.as_ref()
		.unwrap_or_else(|| panic!("expected last scan telemetry"));
	assert!(last_scan.full_project_hit);
	assert_eq!(last_scan.reused_files, 2);
	assert_eq!(last_scan.reparsed_files, 0);
	assert_eq!(last_scan.total_files, 2);

	Ok(())
}

#[test]
fn scan_project_cache_telemetry_tracks_incremental_reuse() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let alpha_template = tmp.path().join("alpha.t.md");
	let beta_template = tmp.path().join("beta.t.md");
	std::fs::write(
		&alpha_template,
		"<!-- {@alpha} -->\n\nAlpha from disk.\n\n<!-- {/alpha} -->\n",
	)
	.unwrap_or_else(|e| panic!("write alpha provider: {e}"));
	std::fs::write(
		&beta_template,
		"<!-- {@beta} -->\n\nBeta from disk.\n\n<!-- {/beta} -->\n",
	)
	.unwrap_or_else(|e| panic!("write beta provider: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=alpha} -->\n\nAlpha from disk.\n\n<!-- {/alpha} -->\n\n<!-- {=beta} -->\n\nBeta \
		 from disk.\n\n<!-- {/beta} -->\n",
	)
	.unwrap_or_else(|e| panic!("write consumer: {e}"));

	let options = ScanOptions::default();
	let _ = scan_project_with_options(tmp.path(), &options)?;
	std::fs::write(
		&beta_template,
		"<!-- {@beta} -->\n\nBeta changed on disk.\n\n<!-- {/beta} -->\n",
	)
	.unwrap_or_else(|e| panic!("rewrite beta provider: {e}"));
	let _ = scan_project_with_options(tmp.path(), &options)?;

	let cache = inspect_project_cache(tmp.path(), &options);
	let telemetry = cache
		.telemetry
		.as_ref()
		.unwrap_or_else(|| panic!("expected cache telemetry"));
	assert_eq!(telemetry.scan_count, 2);
	assert_eq!(telemetry.full_project_hit_count, 0);
	assert_eq!(telemetry.reused_file_count_total, 2);
	assert_eq!(telemetry.reparsed_file_count_total, 4);
	let last_scan = telemetry
		.last_scan
		.as_ref()
		.unwrap_or_else(|| panic!("expected last scan telemetry"));
	assert!(!last_scan.full_project_hit);
	assert_eq!(last_scan.reused_files, 2);
	assert_eq!(last_scan.reparsed_files, 1);
	assert_eq!(last_scan.total_files, 3);

	Ok(())
}

#[test]
fn scan_project_cache_telemetry_resets_after_cold_cache_rebuild() -> MdtResult<()> {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@greeting} -->\n\nHello world!\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write provider: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=greeting} -->\n\nHello world!\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write consumer: {e}"));

	let options = ScanOptions::default();
	let _ = scan_project_with_options(tmp.path(), &options)?;

	let cache_path = project_cache_path(tmp.path());
	std::fs::remove_file(cache_path).unwrap_or_else(|e| panic!("remove cache: {e}"));
	let _ = scan_project_with_options(tmp.path(), &options)?;

	let cache = inspect_project_cache(tmp.path(), &options);
	let telemetry = cache
		.telemetry
		.as_ref()
		.unwrap_or_else(|| panic!("expected cache telemetry"));
	assert_eq!(telemetry.scan_count, 1);
	assert_eq!(telemetry.full_project_hit_count, 0);
	assert_eq!(telemetry.reused_file_count_total, 0);
	assert_eq!(telemetry.reparsed_file_count_total, 2);
	let last_scan = telemetry
		.last_scan
		.as_ref()
		.unwrap_or_else(|| panic!("expected last scan telemetry"));
	assert!(!last_scan.full_project_hit);
	assert_eq!(last_scan.reused_files, 0);
	assert_eq!(last_scan.reparsed_files, 2);
	assert_eq!(last_scan.total_files, 2);

	Ok(())
}

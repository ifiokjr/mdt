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

	let project = scan_project(tmp.path())?;
	let data = HashMap::new();
	let result = check_project(&project, &data)?;
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

	let project = scan_project(tmp.path())?;
	let data = HashMap::new();
	let result = check_project(&project, &data)?;
	assert!(!result.is_ok());
	assert_eq!(result.stale.len(), 1);
	assert_eq!(result.stale[0].block_name, "block");

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

	let project = scan_project(tmp.path())?;
	let data = HashMap::new();
	let updates = compute_updates(&project, &data)?;
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

	let project = scan_project(tmp.path())?;
	let data = HashMap::new();
	let updates = compute_updates(&project, &data)?;
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

	let project = scan_project(tmp.path())?;
	let data = HashMap::new();
	let updates = compute_updates(&project, &data)?;
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

	let project = scan_project(tmp.path())?;
	let data = HashMap::new();
	let updates = compute_updates(&project, &data)?;
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
	let project = scan_project(tmp.path())?;
	let data = HashMap::new();
	let updates = compute_updates(&project, &data)?;
	write_updates(&updates)?;
	assert_eq!(updates.updated_count, 1);

	// Second update should be noop
	let project = scan_project(tmp.path())?;
	let updates = compute_updates(&project, &data)?;
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

	let (project, data) = scan_project_with_config(tmp.path())?;
	let updates = compute_updates(&project, &data)?;
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

	let (project, _data) = scan_project_with_config(tmp.path())?;
	// Should find the readme consumer but not the vendor one
	assert_eq!(project.consumers.len(), 1);
	assert!(
		project.consumers[0]
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
		Argument::Number(n) => assert_eq!(*n, 4.0),
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

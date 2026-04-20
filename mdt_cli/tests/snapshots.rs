mod common;

use insta_cmd::assert_cmd_snapshot;

fn run_update(path: &std::path::Path) {
	let status = common::mdt_cmd_for_path(path)
		.arg("update")
		.status()
		.unwrap_or_else(|e| panic!("failed to run mdt update: {e}"));
	assert!(status.success(), "mdt update should succeed");
}

// ---------------------------------------------------------------------------
// init: create a sample template file
// ---------------------------------------------------------------------------

#[test]
fn init_fresh_directory() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"init_fresh_directory",
			common::mdt_cmd_for_path(tmp.path()).arg("init")
		);
	});

	let template = std::fs::read_to_string(tmp.path().join(".templates/template.t.md"))?;
	insta::assert_snapshot!("init_fresh_directory__template_t_md", template);

	let config = std::fs::read_to_string(tmp.path().join("mdt.toml"))?;
	insta::assert_snapshot!("init_fresh_directory__mdt_toml", config);

	Ok(())
}

#[test]
fn init_existing_template() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("init_existing", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"init_existing_template",
			common::mdt_cmd_for_path(tmp.path()).arg("init")
		);
	});

	let template = std::fs::read_to_string(tmp.path().join("template.t.md"))?;
	insta::assert_snapshot!("init_existing_template__template_t_md", template);

	Ok(())
}

// ---------------------------------------------------------------------------
// list: display all providers and consumers
// ---------------------------------------------------------------------------

#[test]
fn list_blocks() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("list_blocks", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"list_blocks",
			common::mdt_cmd_for_path(tmp.path()).arg("list")
		);
	});

	Ok(())
}

#[test]
fn list_empty_project() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"list_empty_project",
			common::mdt_cmd_for_path(tmp.path()).arg("list")
		);
	});

	Ok(())
}

#[test]
fn list_blocks_verbose() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("list_blocks", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"list_blocks_verbose",
			common::mdt_cmd_for_path(tmp.path())
				.arg("--verbose")
				.arg("list")
		);
	});

	Ok(())
}

// ---------------------------------------------------------------------------
// info: project diagnostics summary
// ---------------------------------------------------------------------------

#[test]
fn info_empty_project() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"info_empty_project",
			common::mdt_cmd_for_path(tmp.path()).arg("info")
		);
	});

	Ok(())
}

#[test]
fn info_project() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("info_project", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"info_project",
			common::mdt_cmd_for_path(tmp.path()).arg("info")
		);
	});

	Ok(())
}

#[test]
fn info_empty_project_json() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"info_empty_project_json",
			common::mdt_cmd_for_path(tmp.path())
				.arg("info")
				.arg("--format")
				.arg("json")
		);
	});

	Ok(())
}

#[test]
fn info_project_json() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("info_project", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"info_project_json",
			common::mdt_cmd_for_path(tmp.path())
				.arg("info")
				.arg("--format")
				.arg("json")
		);
	});

	Ok(())
}

// ---------------------------------------------------------------------------
// check output formats: text (default), json, github
// ---------------------------------------------------------------------------

#[test]
fn check_format_text_stale() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_formats", tmp.path());

	assert_cmd_snapshot!(
		"check_format_text_stale",
		common::mdt_cmd_for_path(tmp.path())
			.arg("check")
			.arg("--format")
			.arg("text")
	);

	Ok(())
}

#[test]
fn check_format_json_stale() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_formats", tmp.path());

	assert_cmd_snapshot!(
		"check_format_json_stale",
		common::mdt_cmd_for_path(tmp.path())
			.arg("check")
			.arg("--format")
			.arg("json")
	);

	Ok(())
}

#[test]
fn check_format_github_stale() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_formats", tmp.path());

	assert_cmd_snapshot!(
		"check_format_github_stale",
		common::mdt_cmd_for_path(tmp.path())
			.arg("check")
			.arg("--format")
			.arg("github")
	);

	Ok(())
}

#[test]
fn check_format_json_up_to_date() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_formats", tmp.path());
	run_update(tmp.path());

	assert_cmd_snapshot!(
		"check_format_json_up_to_date",
		common::mdt_cmd_for_path(tmp.path())
			.arg("check")
			.arg("--format")
			.arg("json")
	);

	Ok(())
}

#[test]
fn check_format_github_up_to_date() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_formats", tmp.path());
	run_update(tmp.path());

	assert_cmd_snapshot!(
		"check_format_github_up_to_date",
		common::mdt_cmd_for_path(tmp.path())
			.arg("check")
			.arg("--format")
			.arg("github")
	);

	Ok(())
}

#[test]
fn check_with_diff() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_formats", tmp.path());

	assert_cmd_snapshot!(
		"check_with_diff",
		common::mdt_cmd_for_path(tmp.path())
			.arg("check")
			.arg("--diff")
	);

	Ok(())
}

#[test]
fn formatter_check_json_reports_stale_files() -> std::io::Result<()> {
	if cfg!(windows) {
		return Ok(());
	}

	let tmp = tempfile::tempdir()?;
	std::fs::write(
		tmp.path().join("mdt.toml"),
		r#"[[formatters]]
command = "/usr/bin/perl -0pe 's/Draft title/Published title/g'"
patterns = ["**/*.md"]
"#,
	)?;
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@body} -->\n\nBody content.\n\n<!-- {/body} -->\n",
	)?;
	std::fs::write(
		tmp.path().join("readme.md"),
		"# Draft title\n\n<!-- {=body} -->\n\nBody content.\n\n<!-- {/body} -->\n",
	)?;

	assert_cmd_snapshot!(
		"formatter_check_json_reports_stale_files",
		common::mdt_cmd_for_path(tmp.path())
			.arg("check")
			.arg("--format")
			.arg("json")
	);

	Ok(())
}

#[test]
fn formatter_update_normalize_only() -> std::io::Result<()> {
	if cfg!(windows) {
		return Ok(());
	}

	let tmp = tempfile::tempdir()?;
	std::fs::write(
		tmp.path().join("mdt.toml"),
		r#"[[formatters]]
command = "/usr/bin/perl -0pe 's/Draft title/Published title/g'"
patterns = ["**/*.md"]
"#,
	)?;
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@body} -->\n\nBody content.\n\n<!-- {/body} -->\n",
	)?;
	std::fs::write(
		tmp.path().join("readme.md"),
		"# Draft title\n\n<!-- {=body} -->\n\nBody content.\n\n<!-- {/body} -->\n",
	)?;

	assert_cmd_snapshot!(
		"formatter_update_normalize_only",
		common::mdt_cmd_for_path(tmp.path()).arg("update")
	);
	let readme = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	insta::assert_snapshot!("formatter_update_normalize_only__readme_md", readme);

	Ok(())
}

// ---------------------------------------------------------------------------
// verbose output: scan details during update and check
// ---------------------------------------------------------------------------

#[test]
fn update_verbose() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("typescript_workspace", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"update_verbose",
			common::mdt_cmd_for_path(tmp.path())
				.arg("--verbose")
				.arg("update")
		);
	});

	Ok(())
}

#[test]
fn update_verbose_up_to_date() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("typescript_workspace", tmp.path());
	run_update(tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"update_verbose_up_to_date",
			common::mdt_cmd_for_path(tmp.path())
				.arg("--verbose")
				.arg("update")
		);
	});

	Ok(())
}

#[test]
fn check_verbose_up_to_date() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("typescript_workspace", tmp.path());
	run_update(tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"check_verbose_up_to_date",
			common::mdt_cmd_for_path(tmp.path())
				.arg("--verbose")
				.arg("check")
		);
	});

	Ok(())
}

// ---------------------------------------------------------------------------
// unused provider: diagnostic warning for orphaned providers
// ---------------------------------------------------------------------------

#[test]
fn unused_provider_check() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("unused_provider", tmp.path());

	assert_cmd_snapshot!(
		"unused_provider_check",
		common::mdt_cmd_for_path(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn unused_provider_check_verbose() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("unused_provider", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"unused_provider_check_verbose",
			common::mdt_cmd_for_path(tmp.path())
				.arg("--verbose")
				.arg("check")
		);
	});

	Ok(())
}

#[test]
fn unused_provider_ignore_flag() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("unused_provider", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"unused_provider_ignore_flag",
			common::mdt_cmd_for_path(tmp.path())
				.arg("--ignore-unused-blocks")
				.arg("--verbose")
				.arg("check")
		);
	});

	Ok(())
}

// ---------------------------------------------------------------------------
// unknown transformer: diagnostic error for unrecognized transformer names
// ---------------------------------------------------------------------------

#[test]
fn unknown_transformer_check() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("unknown_transformer", tmp.path());

	assert_cmd_snapshot!(
		"unknown_transformer_check",
		common::mdt_cmd_for_path(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn unknown_transformer_ignore_flag() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("unknown_transformer", tmp.path());

	assert_cmd_snapshot!(
		"unknown_transformer_ignore_flag",
		common::mdt_cmd_for_path(tmp.path())
			.arg("--ignore-invalid-transformers")
			.arg("check")
	);

	Ok(())
}

// ---------------------------------------------------------------------------
// missing provider: consumer references non-existent provider
// ---------------------------------------------------------------------------

#[test]
fn missing_provider_check() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("missing_provider", tmp.path());

	assert_cmd_snapshot!(
		"missing_provider_check",
		common::mdt_cmd_for_path(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn missing_provider_update() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("missing_provider", tmp.path());

	assert_cmd_snapshot!(
		"missing_provider_update",
		common::mdt_cmd_for_path(tmp.path()).arg("update")
	);

	let readme = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	insta::assert_snapshot!("missing_provider_update__readme_md", readme);

	Ok(())
}

// ---------------------------------------------------------------------------
// multiple providers: multiple blocks consumed by multiple files
// ---------------------------------------------------------------------------

#[test]
fn multiple_providers_update() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("multiple_providers", tmp.path());

	assert_cmd_snapshot!(
		"multiple_providers_update",
		common::mdt_cmd_for_path(tmp.path()).arg("update")
	);

	let readme = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	insta::assert_snapshot!("multiple_providers_update__readme_md", readme);

	let docs = std::fs::read_to_string(tmp.path().join("docs.md"))?;
	insta::assert_snapshot!("multiple_providers_update__docs_md", docs);

	Ok(())
}

#[test]
fn multiple_providers_check_after_update() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("multiple_providers", tmp.path());
	run_update(tmp.path());

	assert_cmd_snapshot!(
		"multiple_providers_check_after_update",
		common::mdt_cmd_for_path(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn multiple_providers_dry_run() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("multiple_providers", tmp.path());

	let readme_before = std::fs::read_to_string(tmp.path().join("readme.md"))?;

	assert_cmd_snapshot!(
		"multiple_providers_dry_run",
		common::mdt_cmd_for_path(tmp.path())
			.arg("update")
			.arg("--dry-run")
	);

	let readme_after = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	similar_asserts::assert_eq!(readme_before, readme_after);

	Ok(())
}

#[test]
fn multiple_providers_list() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("multiple_providers", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"multiple_providers_list",
			common::mdt_cmd_for_path(tmp.path()).arg("list")
		);
	});

	Ok(())
}

// ---------------------------------------------------------------------------
// no subcommand: running mdt with no subcommand should show an error
// ---------------------------------------------------------------------------

#[test]
fn no_subcommand() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;

	assert_cmd_snapshot!("no_subcommand", common::mdt_cmd_for_path(tmp.path()));

	Ok(())
}

// ---------------------------------------------------------------------------
// empty project: no providers or consumers
// ---------------------------------------------------------------------------

#[test]
fn empty_project_check() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;

	assert_cmd_snapshot!(
		"empty_project_check",
		common::mdt_cmd_for_path(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn empty_project_update() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;

	assert_cmd_snapshot!(
		"empty_project_update",
		common::mdt_cmd_for_path(tmp.path()).arg("update")
	);

	Ok(())
}

// ---------------------------------------------------------------------------
// pad_blocks_rust: Rust doc comments with pad_blocks enabled
// ---------------------------------------------------------------------------

#[test]
fn pad_blocks_rust_check_stale() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("pad_blocks_rust", tmp.path());

	assert_cmd_snapshot!(
		"pad_blocks_rust_check_stale",
		common::mdt_cmd_for_path(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn pad_blocks_rust_check_stale_diff() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("pad_blocks_rust", tmp.path());

	assert_cmd_snapshot!(
		"pad_blocks_rust_check_stale_diff",
		common::mdt_cmd_for_path(tmp.path())
			.arg("check")
			.arg("--diff")
	);

	Ok(())
}

#[test]
fn pad_blocks_rust_update() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("pad_blocks_rust", tmp.path());

	assert_cmd_snapshot!(
		"pad_blocks_rust_update",
		common::mdt_cmd_for_path(tmp.path()).arg("update")
	);

	let lib_rs = std::fs::read_to_string(tmp.path().join("lib.rs"))?;
	insta::assert_snapshot!("pad_blocks_rust_update__lib_rs", lib_rs);

	let readme = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	insta::assert_snapshot!("pad_blocks_rust_update__readme_md", readme);

	Ok(())
}

#[test]
fn pad_blocks_rust_check_after_update() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("pad_blocks_rust", tmp.path());
	run_update(tmp.path());

	assert_cmd_snapshot!(
		"pad_blocks_rust_check_after_update",
		common::mdt_cmd_for_path(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn pad_blocks_rust_update_idempotent() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("pad_blocks_rust", tmp.path());
	run_update(tmp.path());

	let lib_after_first = std::fs::read_to_string(tmp.path().join("lib.rs"))?;

	assert_cmd_snapshot!(
		"pad_blocks_rust_update_idempotent",
		common::mdt_cmd_for_path(tmp.path()).arg("update")
	);

	let lib_after_second = std::fs::read_to_string(tmp.path().join("lib.rs"))?;
	similar_asserts::assert_eq!(lib_after_first, lib_after_second);

	Ok(())
}

// ---------------------------------------------------------------------------
// pad_blocks_multi_lang: multiple source languages + data interpolation
// ---------------------------------------------------------------------------

#[test]
fn pad_blocks_multi_lang_check_stale() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("pad_blocks_multi_lang", tmp.path());

	assert_cmd_snapshot!(
		"pad_blocks_multi_lang_check_stale",
		common::mdt_cmd_for_path(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn pad_blocks_multi_lang_update() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("pad_blocks_multi_lang", tmp.path());

	assert_cmd_snapshot!(
		"pad_blocks_multi_lang_update",
		common::mdt_cmd_for_path(tmp.path()).arg("update")
	);

	let lib_rs = std::fs::read_to_string(tmp.path().join("src/lib.rs"))?;
	insta::assert_snapshot!("pad_blocks_multi_lang_update__lib_rs", lib_rs);

	let index_ts = std::fs::read_to_string(tmp.path().join("src/index.ts"))?;
	insta::assert_snapshot!("pad_blocks_multi_lang_update__index_ts", index_ts);

	let main_py = std::fs::read_to_string(tmp.path().join("src/main.py"))?;
	insta::assert_snapshot!("pad_blocks_multi_lang_update__main_py", main_py);

	let main_go = std::fs::read_to_string(tmp.path().join("src/main.go"))?;
	insta::assert_snapshot!("pad_blocks_multi_lang_update__main_go", main_go);

	Ok(())
}

#[test]
fn pad_blocks_multi_lang_check_after_update() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("pad_blocks_multi_lang", tmp.path());
	run_update(tmp.path());

	assert_cmd_snapshot!(
		"pad_blocks_multi_lang_check_after_update",
		common::mdt_cmd_for_path(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn pad_blocks_multi_lang_update_idempotent() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("pad_blocks_multi_lang", tmp.path());
	run_update(tmp.path());

	let lib_rs_first = std::fs::read_to_string(tmp.path().join("src/lib.rs"))?;
	let index_ts_first = std::fs::read_to_string(tmp.path().join("src/index.ts"))?;

	assert_cmd_snapshot!(
		"pad_blocks_multi_lang_update_idempotent",
		common::mdt_cmd_for_path(tmp.path()).arg("update")
	);

	let lib_rs_second = std::fs::read_to_string(tmp.path().join("src/lib.rs"))?;
	let index_ts_second = std::fs::read_to_string(tmp.path().join("src/index.ts"))?;

	similar_asserts::assert_eq!(lib_rs_first, lib_rs_second);
	similar_asserts::assert_eq!(index_ts_first, index_ts_second);

	Ok(())
}

#[test]
fn pad_blocks_multi_lang_dry_run() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("pad_blocks_multi_lang", tmp.path());

	let lib_rs_before = std::fs::read_to_string(tmp.path().join("src/lib.rs"))?;

	assert_cmd_snapshot!(
		"pad_blocks_multi_lang_dry_run",
		common::mdt_cmd_for_path(tmp.path())
			.arg("update")
			.arg("--dry-run")
	);

	let lib_rs_after = std::fs::read_to_string(tmp.path().join("src/lib.rs"))?;
	similar_asserts::assert_eq!(lib_rs_before, lib_rs_after);

	Ok(())
}

// ---------------------------------------------------------------------------
// padding_zero_rust: Rust doc comments with before=0, after=0
// ---------------------------------------------------------------------------

#[test]
fn padding_zero_rust_check_stale() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("padding_zero_rust", tmp.path());

	assert_cmd_snapshot!(
		"padding_zero_rust_check_stale",
		common::mdt_cmd_for_path(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn padding_zero_rust_update() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("padding_zero_rust", tmp.path());

	assert_cmd_snapshot!(
		"padding_zero_rust_update",
		common::mdt_cmd_for_path(tmp.path()).arg("update")
	);

	let lib_rs = std::fs::read_to_string(tmp.path().join("lib.rs"))?;
	insta::assert_snapshot!("padding_zero_rust_update__lib_rs", lib_rs);

	let readme = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	insta::assert_snapshot!("padding_zero_rust_update__readme_md", readme);

	Ok(())
}

#[test]
fn padding_zero_rust_check_after_update() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("padding_zero_rust", tmp.path());
	run_update(tmp.path());

	assert_cmd_snapshot!(
		"padding_zero_rust_check_after_update",
		common::mdt_cmd_for_path(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn padding_zero_rust_update_idempotent() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("padding_zero_rust", tmp.path());
	run_update(tmp.path());

	let lib_after_first = std::fs::read_to_string(tmp.path().join("lib.rs"))?;

	assert_cmd_snapshot!(
		"padding_zero_rust_update_idempotent",
		common::mdt_cmd_for_path(tmp.path()).arg("update")
	);

	let lib_after_second = std::fs::read_to_string(tmp.path().join("lib.rs"))?;
	similar_asserts::assert_eq!(lib_after_first, lib_after_second);

	Ok(())
}

// ---------------------------------------------------------------------------
// validation_errors: unclosed blocks produce error diagnostics
// ---------------------------------------------------------------------------

#[test]
fn validation_errors_check() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("validation_errors", tmp.path());

	assert_cmd_snapshot!(
		"validation_errors_check",
		common::mdt_cmd_for_path(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn validation_errors_update() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("validation_errors", tmp.path());

	assert_cmd_snapshot!(
		"validation_errors_update",
		common::mdt_cmd_for_path(tmp.path()).arg("update")
	);

	Ok(())
}

#[test]
fn validation_errors_ignore_flag() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("validation_errors", tmp.path());

	assert_cmd_snapshot!(
		"validation_errors_ignore_flag",
		common::mdt_cmd_for_path(tmp.path())
			.arg("--ignore-unclosed-blocks")
			.arg("check")
	);

	Ok(())
}

// ---------------------------------------------------------------------------
// include_empty: linePrefix with and without includeEmpty
// ---------------------------------------------------------------------------

#[test]
fn include_empty_update() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("include_empty", tmp.path());

	assert_cmd_snapshot!(
		"include_empty_update",
		common::mdt_cmd_for_path(tmp.path()).arg("update")
	);

	let lib_rs = std::fs::read_to_string(tmp.path().join("lib.rs"))?;
	insta::assert_snapshot!("include_empty_update__lib_rs", lib_rs);

	let no_include = std::fs::read_to_string(tmp.path().join("no_include_empty.rs"))?;
	insta::assert_snapshot!("include_empty_update__no_include_empty_rs", no_include);

	Ok(())
}

#[test]
fn include_empty_check_after_update() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("include_empty", tmp.path());
	run_update(tmp.path());

	assert_cmd_snapshot!(
		"include_empty_check_after_update",
		common::mdt_cmd_for_path(tmp.path()).arg("check")
	);

	Ok(())
}

// ---------------------------------------------------------------------------
// orphan_consumer: consumer references non-existent provider + transformers
// ---------------------------------------------------------------------------

#[test]
fn list_orphan_consumer() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("orphan_consumer", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"list_orphan_consumer",
			common::mdt_cmd_for_path(tmp.path()).arg("list")
		);
	});

	Ok(())
}

#[test]
fn list_orphan_consumer_verbose() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("orphan_consumer", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"list_orphan_consumer_verbose",
			common::mdt_cmd_for_path(tmp.path())
				.arg("--verbose")
				.arg("list")
		);
	});

	Ok(())
}

#[test]
fn orphan_consumer_check() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("orphan_consumer", tmp.path());

	assert_cmd_snapshot!(
		"orphan_consumer_check",
		common::mdt_cmd_for_path(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn orphan_consumer_update() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("orphan_consumer", tmp.path());

	assert_cmd_snapshot!(
		"orphan_consumer_update",
		common::mdt_cmd_for_path(tmp.path()).arg("update")
	);

	let readme = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	insta::assert_snapshot!("orphan_consumer_update__readme_md", readme);

	Ok(())
}

// ---------------------------------------------------------------------------
// verbose check/update with stale content
// ---------------------------------------------------------------------------

#[test]
fn check_verbose_stale() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_formats", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"check_verbose_stale",
			common::mdt_cmd_for_path(tmp.path())
				.arg("--verbose")
				.arg("check")
		);
	});

	Ok(())
}

#[test]
fn check_diff_text_verbose() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_formats", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"check_diff_text_verbose",
			common::mdt_cmd_for_path(tmp.path())
				.arg("--verbose")
				.arg("check")
				.arg("--diff")
		);
	});

	Ok(())
}

#[test]
fn update_dry_run_verbose() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("multiple_providers", tmp.path());

	let readme_before = std::fs::read_to_string(tmp.path().join("readme.md"))?;

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"update_dry_run_verbose",
			common::mdt_cmd_for_path(tmp.path())
				.arg("--verbose")
				.arg("update")
				.arg("--dry-run")
		);
	});

	let readme_after = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	similar_asserts::assert_eq!(readme_before, readme_after);

	Ok(())
}

#[test]
fn update_verbose_multiple_providers() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("multiple_providers", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"update_verbose_multiple_providers",
			common::mdt_cmd_for_path(tmp.path())
				.arg("--verbose")
				.arg("update")
		);
	});

	Ok(())
}

// ---------------------------------------------------------------------------
// verbose diagnostics: warning display with ignore flags
// ---------------------------------------------------------------------------

#[test]
fn validation_errors_check_verbose() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("validation_errors", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"validation_errors_check_verbose",
			common::mdt_cmd_for_path(tmp.path())
				.arg("--verbose")
				.arg("check")
		);
	});

	Ok(())
}

#[test]
fn validation_errors_ignore_verbose() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("validation_errors", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"validation_errors_ignore_verbose",
			common::mdt_cmd_for_path(tmp.path())
				.arg("--ignore-unclosed-blocks")
				.arg("--verbose")
				.arg("check")
		);
	});

	Ok(())
}

#[test]
fn unknown_transformer_check_verbose() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("unknown_transformer", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"unknown_transformer_check_verbose",
			common::mdt_cmd_for_path(tmp.path())
				.arg("--verbose")
				.arg("check")
		);
	});

	Ok(())
}

#[test]
fn unknown_transformer_ignore_verbose() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("unknown_transformer", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"unknown_transformer_ignore_verbose",
			common::mdt_cmd_for_path(tmp.path())
				.arg("--ignore-invalid-transformers")
				.arg("--verbose")
				.arg("check")
		);
	});

	Ok(())
}

// ---------------------------------------------------------------------------
// typescript_workspace: data interpolation from package.json
// ---------------------------------------------------------------------------

#[test]
fn typescript_workspace_check_stale() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("typescript_workspace", tmp.path());

	assert_cmd_snapshot!(
		"typescript_workspace_check_stale",
		common::mdt_cmd_for_path(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn typescript_workspace_update() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("typescript_workspace", tmp.path());

	assert_cmd_snapshot!(
		"typescript_workspace_update",
		common::mdt_cmd_for_path(tmp.path()).arg("update")
	);

	let readme = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	insta::assert_snapshot!("typescript_workspace_update__readme_md", readme);

	let index_ts = std::fs::read_to_string(tmp.path().join("src/index.ts"))?;
	insta::assert_snapshot!("typescript_workspace_update__index_ts", index_ts);

	Ok(())
}

#[test]
fn typescript_workspace_check_after_update() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("typescript_workspace", tmp.path());
	run_update(tmp.path());

	assert_cmd_snapshot!(
		"typescript_workspace_check_after_update",
		common::mdt_cmd_for_path(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn typescript_workspace_update_idempotent() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("typescript_workspace", tmp.path());
	run_update(tmp.path());

	let readme_first = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	let index_ts_first = std::fs::read_to_string(tmp.path().join("src/index.ts"))?;

	assert_cmd_snapshot!(
		"typescript_workspace_update_idempotent",
		common::mdt_cmd_for_path(tmp.path()).arg("update")
	);

	let readme_second = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	let index_ts_second = std::fs::read_to_string(tmp.path().join("src/index.ts"))?;

	similar_asserts::assert_eq!(readme_first, readme_second);
	similar_asserts::assert_eq!(index_ts_first, index_ts_second);

	Ok(())
}

#[test]
fn typescript_workspace_dry_run() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("typescript_workspace", tmp.path());

	let readme_before = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	let index_ts_before = std::fs::read_to_string(tmp.path().join("src/index.ts"))?;

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"typescript_workspace_dry_run",
			common::mdt_cmd_for_path(tmp.path())
				.arg("update")
				.arg("--dry-run")
		);
	});

	let readme_after = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	let index_ts_after = std::fs::read_to_string(tmp.path().join("src/index.ts"))?;

	similar_asserts::assert_eq!(readme_before, readme_after);
	similar_asserts::assert_eq!(index_ts_before, index_ts_after);

	Ok(())
}

// ---------------------------------------------------------------------------
// doctor: project health diagnostics
// ---------------------------------------------------------------------------

#[test]
fn doctor_empty_project() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"doctor_empty_project",
			common::mdt_cmd_for_path(tmp.path()).arg("doctor")
		);
	});

	Ok(())
}

#[test]
fn doctor_empty_project_json() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"doctor_empty_project_json",
			common::mdt_cmd_for_path(tmp.path())
				.arg("doctor")
				.arg("--format")
				.arg("json")
		);
	});

	Ok(())
}

#[test]
fn doctor_info_project_fails() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("info_project", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"doctor_info_project_fails",
			common::mdt_cmd_for_path(tmp.path()).arg("doctor")
		);
	});

	Ok(())
}

#[test]
fn doctor_duplicate_provider_fails() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("doctor_duplicate_provider", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"doctor_duplicate_provider_fails",
			common::mdt_cmd_for_path(tmp.path()).arg("doctor")
		);
	});

	Ok(())
}

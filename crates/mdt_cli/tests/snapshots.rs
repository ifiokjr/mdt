use std::path::Path;
use std::process::Command;

use insta_cmd::assert_cmd_snapshot;
use insta_cmd::get_cargo_bin;
use mdt_core::AnyEmptyResult;

fn copy_fixture(name: &str, dest: &Path) {
	let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("tests/fixtures")
		.join(name);
	copy_dir_recursive(&fixture, dest);
}

fn copy_dir_recursive(src: &Path, dst: &Path) {
	std::fs::create_dir_all(dst)
		.unwrap_or_else(|e| panic!("create_dir_all {}: {e}", dst.display()));
	for entry in
		std::fs::read_dir(src).unwrap_or_else(|e| panic!("read_dir {}: {e}", src.display()))
	{
		let entry = entry.unwrap_or_else(|e| panic!("entry: {e}"));
		let src_path = entry.path();
		let dst_path = dst.join(entry.file_name());

		if src_path.is_dir() {
			copy_dir_recursive(&src_path, &dst_path);
		} else {
			std::fs::copy(&src_path, &dst_path).unwrap_or_else(|e| {
				panic!("copy {} -> {}: {e}", src_path.display(), dst_path.display())
			});
		}
	}
}

fn mdt_cmd(path: &Path) -> Command {
	let mut cmd = Command::new(get_cargo_bin("mdt"));
	cmd.env("NO_COLOR", "1");
	cmd.arg("--path");
	cmd.arg(path);
	cmd
}

fn run_update(path: &Path) {
	let status = Command::new(get_cargo_bin("mdt"))
		.env("NO_COLOR", "1")
		.arg("--path")
		.arg(path)
		.arg("update")
		.status()
		.unwrap_or_else(|e| panic!("failed to run mdt update: {e}"));
	assert!(status.success(), "mdt update should succeed");
}

/// Bind insta settings that redact the temp directory path from snapshot
/// output, replacing it with `[TEMP_DIR]`. This ensures snapshots are
/// reproducible across machines and runs.
fn with_redacted_paths(tmp_path: &Path, f: impl FnOnce()) {
	let path_str = tmp_path.display().to_string();
	// Escape regex metacharacters in the path
	let mut escaped = String::with_capacity(path_str.len() * 2);
	for ch in path_str.chars() {
		if matches!(
			ch,
			'\\' | '.' | '+' | '*' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|'
		) {
			escaped.push('\\');
		}
		escaped.push(ch);
	}
	let mut settings = insta::Settings::clone_current();
	settings.add_filter(&escaped, "[TEMP_DIR]");
	settings.bind(f);
}

// ---------------------------------------------------------------------------
// init: create a sample template file
// ---------------------------------------------------------------------------

#[test]
fn init_fresh_directory() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	with_redacted_paths(tmp.path(), || {
		assert_cmd_snapshot!("init_fresh_directory", mdt_cmd(tmp.path()).arg("init"));
	});

	let template = std::fs::read_to_string(tmp.path().join("template.t.md"))?;
	insta::assert_snapshot!("init_fresh_directory_template", template);

	Ok(())
}

#[test]
fn init_existing_template() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("init_existing", tmp.path());

	with_redacted_paths(tmp.path(), || {
		assert_cmd_snapshot!("init_existing_template", mdt_cmd(tmp.path()).arg("init"));
	});

	let template = std::fs::read_to_string(tmp.path().join("template.t.md"))?;
	assert!(
		template.contains("{@greeting}"),
		"original template should be preserved"
	);

	Ok(())
}

// ---------------------------------------------------------------------------
// list: display all providers and consumers
// ---------------------------------------------------------------------------

#[test]
fn list_blocks() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("list_blocks", tmp.path());

	with_redacted_paths(tmp.path(), || {
		assert_cmd_snapshot!("list_blocks", mdt_cmd(tmp.path()).arg("list"));
	});

	Ok(())
}

#[test]
fn list_empty_project() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	with_redacted_paths(tmp.path(), || {
		assert_cmd_snapshot!("list_empty_project", mdt_cmd(tmp.path()).arg("list"));
	});

	Ok(())
}

#[test]
fn list_blocks_verbose() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("list_blocks", tmp.path());

	with_redacted_paths(tmp.path(), || {
		assert_cmd_snapshot!(
			"list_blocks_verbose",
			mdt_cmd(tmp.path()).arg("--verbose").arg("list")
		);
	});

	Ok(())
}

// ---------------------------------------------------------------------------
// check output formats: text (default), json, github
// ---------------------------------------------------------------------------

#[test]
fn check_format_text_stale() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("check_formats", tmp.path());

	assert_cmd_snapshot!(
		"check_format_text_stale",
		mdt_cmd(tmp.path()).arg("check").arg("--format").arg("text")
	);

	Ok(())
}

#[test]
fn check_format_json_stale() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("check_formats", tmp.path());

	assert_cmd_snapshot!(
		"check_format_json_stale",
		mdt_cmd(tmp.path()).arg("check").arg("--format").arg("json")
	);

	Ok(())
}

#[test]
fn check_format_github_stale() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("check_formats", tmp.path());

	assert_cmd_snapshot!(
		"check_format_github_stale",
		mdt_cmd(tmp.path())
			.arg("check")
			.arg("--format")
			.arg("github")
	);

	Ok(())
}

#[test]
fn check_format_json_up_to_date() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("check_formats", tmp.path());
	run_update(tmp.path());

	assert_cmd_snapshot!(
		"check_format_json_up_to_date",
		mdt_cmd(tmp.path()).arg("check").arg("--format").arg("json")
	);

	Ok(())
}

#[test]
fn check_format_github_up_to_date() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("check_formats", tmp.path());
	run_update(tmp.path());

	assert_cmd_snapshot!(
		"check_format_github_up_to_date",
		mdt_cmd(tmp.path())
			.arg("check")
			.arg("--format")
			.arg("github")
	);

	Ok(())
}

#[test]
fn check_with_diff() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("check_formats", tmp.path());

	assert_cmd_snapshot!(
		"check_with_diff",
		mdt_cmd(tmp.path()).arg("check").arg("--diff")
	);

	Ok(())
}

// ---------------------------------------------------------------------------
// verbose output: scan details during update and check
// ---------------------------------------------------------------------------

#[test]
fn update_verbose() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("typescript_workspace", tmp.path());

	with_redacted_paths(tmp.path(), || {
		assert_cmd_snapshot!(
			"update_verbose",
			mdt_cmd(tmp.path()).arg("--verbose").arg("update")
		);
	});

	Ok(())
}

#[test]
fn update_verbose_up_to_date() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("typescript_workspace", tmp.path());
	run_update(tmp.path());

	with_redacted_paths(tmp.path(), || {
		assert_cmd_snapshot!(
			"update_verbose_up_to_date",
			mdt_cmd(tmp.path()).arg("--verbose").arg("update")
		);
	});

	Ok(())
}

#[test]
fn check_verbose_up_to_date() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("typescript_workspace", tmp.path());
	run_update(tmp.path());

	with_redacted_paths(tmp.path(), || {
		assert_cmd_snapshot!(
			"check_verbose_up_to_date",
			mdt_cmd(tmp.path()).arg("--verbose").arg("check")
		);
	});

	Ok(())
}

// ---------------------------------------------------------------------------
// unused provider: diagnostic warning for orphaned providers
// ---------------------------------------------------------------------------

#[test]
fn unused_provider_check() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("unused_provider", tmp.path());

	assert_cmd_snapshot!("unused_provider_check", mdt_cmd(tmp.path()).arg("check"));

	Ok(())
}

#[test]
fn unused_provider_check_verbose() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("unused_provider", tmp.path());

	with_redacted_paths(tmp.path(), || {
		assert_cmd_snapshot!(
			"unused_provider_check_verbose",
			mdt_cmd(tmp.path()).arg("--verbose").arg("check")
		);
	});

	Ok(())
}

#[test]
fn unused_provider_ignore_flag() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("unused_provider", tmp.path());

	with_redacted_paths(tmp.path(), || {
		assert_cmd_snapshot!(
			"unused_provider_ignore_flag",
			mdt_cmd(tmp.path())
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
fn unknown_transformer_check() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("unknown_transformer", tmp.path());

	assert_cmd_snapshot!(
		"unknown_transformer_check",
		mdt_cmd(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn unknown_transformer_ignore_flag() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("unknown_transformer", tmp.path());

	assert_cmd_snapshot!(
		"unknown_transformer_ignore_flag",
		mdt_cmd(tmp.path())
			.arg("--ignore-invalid-transformers")
			.arg("check")
	);

	Ok(())
}

// ---------------------------------------------------------------------------
// missing provider: consumer references non-existent provider
// ---------------------------------------------------------------------------

#[test]
fn missing_provider_check() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("missing_provider", tmp.path());

	assert_cmd_snapshot!("missing_provider_check", mdt_cmd(tmp.path()).arg("check"));

	Ok(())
}

#[test]
fn missing_provider_update() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("missing_provider", tmp.path());

	assert_cmd_snapshot!("missing_provider_update", mdt_cmd(tmp.path()).arg("update"));

	let readme = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	insta::assert_snapshot!("missing_provider_update_readme_md", readme);

	Ok(())
}

// ---------------------------------------------------------------------------
// multiple providers: multiple blocks consumed by multiple files
// ---------------------------------------------------------------------------

#[test]
fn multiple_providers_update() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("multiple_providers", tmp.path());

	assert_cmd_snapshot!(
		"multiple_providers_update",
		mdt_cmd(tmp.path()).arg("update")
	);

	let readme = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	insta::assert_snapshot!("multiple_providers_update_readme_md", readme);

	let docs = std::fs::read_to_string(tmp.path().join("docs.md"))?;
	insta::assert_snapshot!("multiple_providers_update_docs_md", docs);

	Ok(())
}

#[test]
fn multiple_providers_check_after_update() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("multiple_providers", tmp.path());
	run_update(tmp.path());

	assert_cmd_snapshot!(
		"multiple_providers_check_after_update",
		mdt_cmd(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn multiple_providers_dry_run() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("multiple_providers", tmp.path());

	let readme_before = std::fs::read_to_string(tmp.path().join("readme.md"))?;

	assert_cmd_snapshot!(
		"multiple_providers_dry_run",
		mdt_cmd(tmp.path()).arg("update").arg("--dry-run")
	);

	let readme_after = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	similar_asserts::assert_eq!(readme_before, readme_after);

	Ok(())
}

#[test]
fn multiple_providers_list() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("multiple_providers", tmp.path());

	with_redacted_paths(tmp.path(), || {
		assert_cmd_snapshot!("multiple_providers_list", mdt_cmd(tmp.path()).arg("list"));
	});

	Ok(())
}

// ---------------------------------------------------------------------------
// no subcommand: running mdt with no subcommand should show an error
// ---------------------------------------------------------------------------

#[test]
fn no_subcommand() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	assert_cmd_snapshot!(
		"no_subcommand",
		Command::new(get_cargo_bin("mdt"))
			.env("NO_COLOR", "1")
			.arg("--path")
			.arg(tmp.path())
	);

	Ok(())
}

// ---------------------------------------------------------------------------
// empty project: no providers or consumers
// ---------------------------------------------------------------------------

#[test]
fn empty_project_check() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	assert_cmd_snapshot!("empty_project_check", mdt_cmd(tmp.path()).arg("check"));

	Ok(())
}

#[test]
fn empty_project_update() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	assert_cmd_snapshot!("empty_project_update", mdt_cmd(tmp.path()).arg("update"));

	Ok(())
}

// ---------------------------------------------------------------------------
// pad_blocks_rust: Rust doc comments with pad_blocks enabled
// ---------------------------------------------------------------------------

#[test]
fn pad_blocks_rust_check_stale() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("pad_blocks_rust", tmp.path());

	assert_cmd_snapshot!(
		"pad_blocks_rust_check_stale",
		mdt_cmd(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn pad_blocks_rust_check_stale_diff() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("pad_blocks_rust", tmp.path());

	assert_cmd_snapshot!(
		"pad_blocks_rust_check_stale_diff",
		mdt_cmd(tmp.path()).arg("check").arg("--diff")
	);

	Ok(())
}

#[test]
fn pad_blocks_rust_update() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("pad_blocks_rust", tmp.path());

	assert_cmd_snapshot!("pad_blocks_rust_update", mdt_cmd(tmp.path()).arg("update"));

	let lib_rs = std::fs::read_to_string(tmp.path().join("lib.rs"))?;
	insta::assert_snapshot!("pad_blocks_rust_update_lib_rs", lib_rs);

	let readme = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	insta::assert_snapshot!("pad_blocks_rust_update_readme_md", readme);

	Ok(())
}

#[test]
fn pad_blocks_rust_check_after_update() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("pad_blocks_rust", tmp.path());
	run_update(tmp.path());

	assert_cmd_snapshot!(
		"pad_blocks_rust_check_after_update",
		mdt_cmd(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn pad_blocks_rust_update_idempotent() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("pad_blocks_rust", tmp.path());
	run_update(tmp.path());

	let lib_after_first = std::fs::read_to_string(tmp.path().join("lib.rs"))?;

	assert_cmd_snapshot!(
		"pad_blocks_rust_update_idempotent",
		mdt_cmd(tmp.path()).arg("update")
	);

	let lib_after_second = std::fs::read_to_string(tmp.path().join("lib.rs"))?;
	similar_asserts::assert_eq!(lib_after_first, lib_after_second);

	Ok(())
}

// ---------------------------------------------------------------------------
// pad_blocks_multi_lang: multiple source languages + data interpolation
// ---------------------------------------------------------------------------

#[test]
fn pad_blocks_multi_lang_check_stale() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("pad_blocks_multi_lang", tmp.path());

	assert_cmd_snapshot!(
		"pad_blocks_multi_lang_check_stale",
		mdt_cmd(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn pad_blocks_multi_lang_update() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("pad_blocks_multi_lang", tmp.path());

	assert_cmd_snapshot!(
		"pad_blocks_multi_lang_update",
		mdt_cmd(tmp.path()).arg("update")
	);

	let lib_rs = std::fs::read_to_string(tmp.path().join("src/lib.rs"))?;
	insta::assert_snapshot!("pad_blocks_multi_lang_update_lib_rs", lib_rs);

	let index_ts = std::fs::read_to_string(tmp.path().join("src/index.ts"))?;
	insta::assert_snapshot!("pad_blocks_multi_lang_update_index_ts", index_ts);

	let main_py = std::fs::read_to_string(tmp.path().join("src/main.py"))?;
	insta::assert_snapshot!("pad_blocks_multi_lang_update_main_py", main_py);

	let main_go = std::fs::read_to_string(tmp.path().join("src/main.go"))?;
	insta::assert_snapshot!("pad_blocks_multi_lang_update_main_go", main_go);

	Ok(())
}

#[test]
fn pad_blocks_multi_lang_check_after_update() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("pad_blocks_multi_lang", tmp.path());
	run_update(tmp.path());

	assert_cmd_snapshot!(
		"pad_blocks_multi_lang_check_after_update",
		mdt_cmd(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn pad_blocks_multi_lang_update_idempotent() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("pad_blocks_multi_lang", tmp.path());
	run_update(tmp.path());

	let lib_rs_first = std::fs::read_to_string(tmp.path().join("src/lib.rs"))?;
	let index_ts_first = std::fs::read_to_string(tmp.path().join("src/index.ts"))?;

	assert_cmd_snapshot!(
		"pad_blocks_multi_lang_update_idempotent",
		mdt_cmd(tmp.path()).arg("update")
	);

	let lib_rs_second = std::fs::read_to_string(tmp.path().join("src/lib.rs"))?;
	let index_ts_second = std::fs::read_to_string(tmp.path().join("src/index.ts"))?;

	similar_asserts::assert_eq!(lib_rs_first, lib_rs_second);
	similar_asserts::assert_eq!(index_ts_first, index_ts_second);

	Ok(())
}

#[test]
fn pad_blocks_multi_lang_dry_run() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("pad_blocks_multi_lang", tmp.path());

	let lib_rs_before = std::fs::read_to_string(tmp.path().join("src/lib.rs"))?;

	assert_cmd_snapshot!(
		"pad_blocks_multi_lang_dry_run",
		mdt_cmd(tmp.path()).arg("update").arg("--dry-run")
	);

	let lib_rs_after = std::fs::read_to_string(tmp.path().join("src/lib.rs"))?;
	similar_asserts::assert_eq!(lib_rs_before, lib_rs_after);

	Ok(())
}

// ---------------------------------------------------------------------------
// validation_errors: unclosed blocks produce error diagnostics
// ---------------------------------------------------------------------------

#[test]
fn validation_errors_check() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("validation_errors", tmp.path());

	assert_cmd_snapshot!("validation_errors_check", mdt_cmd(tmp.path()).arg("check"));

	Ok(())
}

#[test]
fn validation_errors_update() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("validation_errors", tmp.path());

	assert_cmd_snapshot!(
		"validation_errors_update",
		mdt_cmd(tmp.path()).arg("update")
	);

	Ok(())
}

#[test]
fn validation_errors_ignore_flag() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("validation_errors", tmp.path());

	assert_cmd_snapshot!(
		"validation_errors_ignore_flag",
		mdt_cmd(tmp.path())
			.arg("--ignore-unclosed-blocks")
			.arg("check")
	);

	Ok(())
}

// ---------------------------------------------------------------------------
// include_empty: linePrefix with and without includeEmpty
// ---------------------------------------------------------------------------

#[test]
fn include_empty_update() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("include_empty", tmp.path());

	assert_cmd_snapshot!("include_empty_update", mdt_cmd(tmp.path()).arg("update"));

	let lib_rs = std::fs::read_to_string(tmp.path().join("lib.rs"))?;
	insta::assert_snapshot!("include_empty_update_lib_rs", lib_rs);

	let no_include = std::fs::read_to_string(tmp.path().join("no_include_empty.rs"))?;
	insta::assert_snapshot!("include_empty_update_no_include_empty_rs", no_include);

	Ok(())
}

#[test]
fn include_empty_check_after_update() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("include_empty", tmp.path());
	run_update(tmp.path());

	assert_cmd_snapshot!(
		"include_empty_check_after_update",
		mdt_cmd(tmp.path()).arg("check")
	);

	Ok(())
}

// ---------------------------------------------------------------------------
// typescript_workspace: data interpolation from package.json
// ---------------------------------------------------------------------------

#[test]
fn typescript_workspace_check_stale() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("typescript_workspace", tmp.path());

	assert_cmd_snapshot!(
		"typescript_workspace_check_stale",
		mdt_cmd(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn typescript_workspace_update() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("typescript_workspace", tmp.path());

	assert_cmd_snapshot!(
		"typescript_workspace_update",
		mdt_cmd(tmp.path()).arg("update")
	);

	let readme = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	insta::assert_snapshot!("typescript_workspace_update_readme_md", readme);

	let index_ts = std::fs::read_to_string(tmp.path().join("src/index.ts"))?;
	insta::assert_snapshot!("typescript_workspace_update_index_ts", index_ts);

	Ok(())
}

#[test]
fn typescript_workspace_check_after_update() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("typescript_workspace", tmp.path());
	run_update(tmp.path());

	assert_cmd_snapshot!(
		"typescript_workspace_check_after_update",
		mdt_cmd(tmp.path()).arg("check")
	);

	Ok(())
}

#[test]
fn typescript_workspace_update_idempotent() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("typescript_workspace", tmp.path());
	run_update(tmp.path());

	let readme_first = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	let index_ts_first = std::fs::read_to_string(tmp.path().join("src/index.ts"))?;

	assert_cmd_snapshot!(
		"typescript_workspace_update_idempotent",
		mdt_cmd(tmp.path()).arg("update")
	);

	let readme_second = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	let index_ts_second = std::fs::read_to_string(tmp.path().join("src/index.ts"))?;

	similar_asserts::assert_eq!(readme_first, readme_second);
	similar_asserts::assert_eq!(index_ts_first, index_ts_second);

	Ok(())
}

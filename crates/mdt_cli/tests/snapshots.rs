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

	// Verify the Rust file was updated correctly — no mangled doc comments
	let lib_rs = std::fs::read_to_string(tmp.path().join("lib.rs"))?;
	insta::assert_snapshot!("pad_blocks_rust_update_lib_rs", lib_rs);

	// Verify the readme was updated
	let readme = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	insta::assert_snapshot!("pad_blocks_rust_update_readme_md", readme);

	Ok(())
}

#[test]
fn pad_blocks_rust_check_after_update() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("pad_blocks_rust", tmp.path());

	// First update
	let status = Command::new(get_cargo_bin("mdt"))
		.env("NO_COLOR", "1")
		.arg("--path")
		.arg(tmp.path())
		.arg("update")
		.status()?;
	assert!(status.success(), "update should succeed");

	// Then check — should pass
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

	// First update
	let status = Command::new(get_cargo_bin("mdt"))
		.env("NO_COLOR", "1")
		.arg("--path")
		.arg(tmp.path())
		.arg("update")
		.status()?;
	assert!(status.success(), "first update should succeed");

	// Capture state after first update
	let lib_after_first = std::fs::read_to_string(tmp.path().join("lib.rs"))?;

	// Second update — should be a no-op
	assert_cmd_snapshot!(
		"pad_blocks_rust_update_idempotent",
		mdt_cmd(tmp.path()).arg("update")
	);

	// File should be unchanged
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

	// Verify each source file — no mangled comments
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

	// Update first
	let status = Command::new(get_cargo_bin("mdt"))
		.env("NO_COLOR", "1")
		.arg("--path")
		.arg(tmp.path())
		.arg("update")
		.status()?;
	assert!(status.success(), "update should succeed");

	// Check should pass
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

	// First update
	let status = Command::new(get_cargo_bin("mdt"))
		.env("NO_COLOR", "1")
		.arg("--path")
		.arg(tmp.path())
		.arg("update")
		.status()?;
	assert!(status.success(), "first update should succeed");

	let lib_rs_first = std::fs::read_to_string(tmp.path().join("src/lib.rs"))?;
	let index_ts_first = std::fs::read_to_string(tmp.path().join("src/index.ts"))?;

	// Second update — no-op
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

	// Files should NOT have changed
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

	// With includeEmpty:true — blank lines get the prefix
	let lib_rs = std::fs::read_to_string(tmp.path().join("lib.rs"))?;
	insta::assert_snapshot!("include_empty_update_lib_rs", lib_rs);

	// Without includeEmpty — blank lines stay empty
	let no_include = std::fs::read_to_string(tmp.path().join("no_include_empty.rs"))?;
	insta::assert_snapshot!("include_empty_update_no_include_empty_rs", no_include);

	Ok(())
}

#[test]
fn include_empty_check_after_update() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture("include_empty", tmp.path());

	// Update first
	let status = Command::new(get_cargo_bin("mdt"))
		.env("NO_COLOR", "1")
		.arg("--path")
		.arg(tmp.path())
		.arg("update")
		.status()?;
	assert!(status.success(), "update should succeed");

	// Check should pass
	assert_cmd_snapshot!(
		"include_empty_check_after_update",
		mdt_cmd(tmp.path()).arg("check")
	);

	Ok(())
}

// ---------------------------------------------------------------------------
// typescript_workspace: snapshot the existing fixture (was only using asserts)
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

	let status = Command::new(get_cargo_bin("mdt"))
		.env("NO_COLOR", "1")
		.arg("--path")
		.arg(tmp.path())
		.arg("update")
		.status()?;
	assert!(status.success(), "update should succeed");

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

	let status = Command::new(get_cargo_bin("mdt"))
		.env("NO_COLOR", "1")
		.arg("--path")
		.arg(tmp.path())
		.arg("update")
		.status()?;
	assert!(status.success(), "first update should succeed");

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

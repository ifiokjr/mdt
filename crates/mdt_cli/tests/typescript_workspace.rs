use std::path::Path;

use assert_cmd::Command;
use mdt::AnyEmptyResult;

fn copy_fixture(dest: &Path) {
	let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/typescript_workspace");
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

#[test]
fn update_typescript_workspace() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture(tmp.path());

	let mut cmd = Command::cargo_bin("mdt")?;
	cmd.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("Updated"));

	// Check readme was updated with rendered template variables
	let readme = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert!(
		readme.contains("my-lib"),
		"readme should contain package name"
	);
	assert!(readme.contains("1.2.3"), "readme should contain version");
	assert!(
		readme.contains("npm install my-lib"),
		"readme should contain install command"
	);
	assert!(
		!readme.contains("Old installation instructions"),
		"old content should be replaced"
	);

	// Check TypeScript source file was updated
	let index_ts = std::fs::read_to_string(tmp.path().join("src/index.ts"))?;
	assert!(
		index_ts.contains("A sample TypeScript library"),
		"index.ts should contain rendered apiDocs"
	);
	assert!(
		!index_ts.contains("Old JSDoc content"),
		"old JSDoc should be replaced"
	);

	Ok(())
}

#[test]
fn check_typescript_workspace_after_update() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture(tmp.path());

	// First update
	Command::cargo_bin("mdt")?
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success();

	// Then check — should pass
	Command::cargo_bin("mdt")?
		.arg("check")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("up to date"));

	Ok(())
}

#[test]
fn check_typescript_workspace_stale() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture(tmp.path());

	// Check without updating — should fail because content is stale
	Command::cargo_bin("mdt")?
		.arg("check")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.failure();

	Ok(())
}

#[test]
fn dry_run_typescript_workspace() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	copy_fixture(tmp.path());

	let readme_before = std::fs::read_to_string(tmp.path().join("readme.md"))?;

	Command::cargo_bin("mdt")?
		.arg("update")
		.arg("--dry-run")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("Dry run"));

	// File should NOT have changed
	let readme_after = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert_eq!(
		readme_before, readme_after,
		"dry run should not modify files"
	);

	Ok(())
}

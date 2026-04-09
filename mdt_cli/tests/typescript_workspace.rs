mod common;

use mdt_core::AnyEmptyResult;

#[test]
fn update_typescript_workspace() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("typescript_workspace", tmp.path());

	common::mdt_cmd()
		.arg("update")
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
	common::copy_fixture("typescript_workspace", tmp.path());

	// First update
	common::mdt_cmd()
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success();

	// Then check — should pass
	common::mdt_cmd()
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
	common::copy_fixture("typescript_workspace", tmp.path());

	// Check without updating — should fail because content is stale
	common::mdt_cmd()
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
	common::copy_fixture("typescript_workspace", tmp.path());

	let readme_before = std::fs::read_to_string(tmp.path().join("readme.md"))?;

	common::mdt_cmd()
		.arg("update")
		.arg("--dry-run")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("would update"));

	// File should NOT have changed
	let readme_after = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert_eq!(
		readme_before, readme_after,
		"dry run should not modify files"
	);

	Ok(())
}

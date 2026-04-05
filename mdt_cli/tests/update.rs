mod common;

use mdt_core::AnyEmptyResult;

#[test]
fn update_replaces_stale_content() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("update_stale", tmp.path());

	common::mdt_cmd()
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("Updated"));

	let content = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert!(content.contains("Hello world!"));
	assert!(!content.contains("Old content."));

	Ok(())
}

#[test]
fn update_noop_when_in_sync() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_up_to_date", tmp.path());

	common::mdt_cmd()
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("already up to date"));

	Ok(())
}

#[test]
fn update_dry_run_does_not_write() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("update_stale", tmp.path());

	let consumer_before = std::fs::read_to_string(tmp.path().join("readme.md"))?;

	common::mdt_cmd()
		.arg("update")
		.arg("--dry-run")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("would update"));

	// File should not have changed
	let content = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert_eq!(content, consumer_before);

	Ok(())
}

#[test]
fn update_with_transformers() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("update_with_transformer", tmp.path());

	common::mdt_cmd()
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success();

	let content = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert!(content.contains("Some documentation content."));

	Ok(())
}

#[test]
fn update_verbose_shows_files() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_stale_named", tmp.path());

	common::mdt_cmd()
		.arg("update")
		.arg("--verbose")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("readme.md"));

	Ok(())
}

#[test]
fn update_warns_missing_provider() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("update_orphan", tmp.path());

	common::mdt_cmd()
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stderr(predicates::str::contains(
			"consumer block `orphan` has no matching provider",
		));

	Ok(())
}

#[test]
fn update_multiple_blocks_in_one_file() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("update_multiple_blocks", tmp.path());

	common::mdt_cmd()
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("Updated 2 block(s)"));

	let content = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert!(content.contains("alpha"));
	assert!(content.contains("beta"));
	assert!(!content.contains("old"));

	Ok(())
}

#[test]
fn update_dry_run_shows_file_list() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_stale_named", tmp.path());

	common::mdt_cmd()
		.arg("update")
		.arg("--dry-run")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("readme.md"));

	Ok(())
}

#[test]
fn update_with_config_and_data() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("update_with_data", tmp.path());

	common::mdt_cmd()
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success();

	let content = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert!(content.contains("npm install my-app@3.0.0"));

	Ok(())
}

#[test]
fn update_inline_table_cell_with_data() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("update_inline_data", tmp.path());

	common::mdt_cmd()
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("Updated"));

	let content = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert!(content.contains(
		"| mdt     | <!-- {~version:\"{{ pkg.version }}\"} -->3.1.4<!-- {/version} --> |"
	));

	Ok(())
}

#[test]
fn update_preserves_multiline_link_definitions() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("update_multiline_links", tmp.path());

	common::mdt_cmd()
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success();

	let content = std::fs::read_to_string(tmp.path().join("readme.md"))?;

	// Each link definition must be on its own line — newlines must be preserved.
	assert!(
		content.contains("\n[crate-image]:"),
		"[crate-image] should be on its own line"
	);
	assert!(
		content.contains("\n[crate-link]:"),
		"[crate-link] should be on its own line"
	);
	assert!(
		content.contains("\n[docs-image]:"),
		"[docs-image] should be on its own line"
	);
	assert!(
		content.contains("\n[docs-link]:"),
		"[docs-link] should be on its own line"
	);
	assert!(
		content.contains("\n[ci-image]:"),
		"[ci-image] should be on its own line"
	);
	assert!(
		content.contains("\n[ci-link]:"),
		"[ci-link] should be on its own line"
	);

	// Verify template variables were rendered
	assert!(content.contains("my_crate"));
	assert!(!content.contains("{{ crateName }}"));

	Ok(())
}

#[test]
fn update_multiline_idempotent_after_write() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("update_multiline_idempotent", tmp.path());

	// First update
	common::mdt_cmd()
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("Updated"));

	let after_first = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert!(after_first.contains("\n[repo]:"));
	assert!(after_first.contains("\n[docs]:"));
	assert!(after_first.contains("\n[ci]:"));

	// Second update — should be idempotent
	common::mdt_cmd()
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("already up to date"));

	let after_second = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert_eq!(
		after_first, after_second,
		"Second update should not change the file"
	);

	Ok(())
}

#[test]
fn update_preserves_surrounding_content() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("update_preserves_surrounding", tmp.path());

	common::mdt_cmd()
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success();

	let content = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert!(content.contains("# Header"));
	assert!(content.contains("Paragraph before."));
	assert!(content.contains("new content"));
	assert!(content.contains("Paragraph after."));
	assert!(!content.contains("old"));

	Ok(())
}

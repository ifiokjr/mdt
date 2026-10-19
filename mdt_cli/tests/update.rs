mod common;

use insta_cmd::assert_cmd_snapshot;
use rstest::rstest;

fn assert_update_snapshot(
	fixture: &str,
	snapshot_name: &str,
	relative_path: &str,
) -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture(fixture, tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			format!("{snapshot_name}__stdout"),
			common::mdt_cmd_for_path(tmp.path()).arg("update")
		);
	});

	let content = std::fs::read_to_string(tmp.path().join(relative_path))?;
	insta::assert_snapshot!(
		format!(
			"{snapshot_name}__{}",
			common::snapshot_path_id(relative_path)
		),
		content
	);

	Ok(())
}

fn assert_update_dry_run_snapshot(
	fixture: &str,
	snapshot_name: &str,
	relative_path: &str,
) -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture(fixture, tmp.path());

	let before = std::fs::read_to_string(tmp.path().join(relative_path))?;

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			format!("{snapshot_name}__stdout"),
			common::mdt_cmd_for_path(tmp.path())
				.arg("update")
				.arg("--dry-run")
		);
	});

	let after = std::fs::read_to_string(tmp.path().join(relative_path))?;
	similar_asserts::assert_eq!(before, after);

	Ok(())
}

#[rstest]
#[case("update_stale", "update_replaces_stale_content", "readme.md")]
#[case("update_with_transformer", "update_with_transformers", "readme.md")]
#[case(
	"update_multiple_blocks",
	"update_multiple_blocks_in_one_file",
	"readme.md"
)]
#[case("update_with_data", "update_with_config_and_data", "readme.md")]
#[case(
	"update_inline_data",
	"update_inline_table_cell_with_data",
	"readme.md"
)]
#[case(
	"update_multiline_links",
	"update_preserves_multiline_link_definitions",
	"readme.md"
)]
#[case(
	"update_preserves_surrounding",
	"update_preserves_surrounding_content",
	"readme.md"
)]
fn update_rewrites_files_from_fixtures(
	#[case] fixture: &str,
	#[case] snapshot_name: &str,
	#[case] relative_path: &str,
) -> std::io::Result<()> {
	assert_update_snapshot(fixture, snapshot_name, relative_path)
}

#[test]
fn update_noop_when_in_sync() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_up_to_date", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"update_noop_when_in_sync",
			common::mdt_cmd_for_path(tmp.path()).arg("update")
		);
	});

	Ok(())
}

#[rstest]
#[case("update_stale", "update_dry_run_does_not_write", "readme.md")]
#[case("check_stale_named", "update_dry_run_shows_file_list", "readme.md")]
fn update_dry_run_preserves_files(
	#[case] fixture: &str,
	#[case] snapshot_name: &str,
	#[case] relative_path: &str,
) -> std::io::Result<()> {
	assert_update_dry_run_snapshot(fixture, snapshot_name, relative_path)
}

#[test]
fn update_verbose_shows_files() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_stale_named", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"update_verbose_shows_files",
			common::mdt_cmd_for_path(tmp.path())
				.arg("--verbose")
				.arg("update")
		);
	});

	Ok(())
}

#[test]
fn update_warns_missing_provider() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("update_orphan", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"update_warns_missing_provider",
			common::mdt_cmd_for_path(tmp.path()).arg("update")
		);
	});

	Ok(())
}

#[test]
fn update_multiline_idempotent_after_write() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("update_multiline_idempotent", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"update_multiline_idempotent_after_write__first_stdout",
			common::mdt_cmd_for_path(tmp.path()).arg("update")
		);
	});

	let after_first = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	insta::assert_snapshot!(
		"update_multiline_idempotent_after_write__readme_md",
		after_first.as_str()
	);

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"update_multiline_idempotent_after_write__second_stdout",
			common::mdt_cmd_for_path(tmp.path()).arg("update")
		);
	});

	let after_second = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	similar_asserts::assert_eq!(after_first, after_second);

	Ok(())
}

// ---------------------------------------------------------------------------
// Scoped updates (--block / --file)
// ---------------------------------------------------------------------------
#[test]
fn update_block_scopes_to_matching_consumers() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("update_scoped", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"update_block_scopes_to_matching_consumers__stdout",
			common::mdt_cmd_for_path(tmp.path())
				.arg("update")
				.arg("--block")
				.arg("alpha")
		);
	});

	// Only the in-scope file was rewritten.
	let one = std::fs::read_to_string(tmp.path().join("one.md"))?;
	insta::assert_snapshot!(
		"update_block_scopes_to_matching_consumers__one_md",
		one.as_str()
	);
	let two = std::fs::read_to_string(tmp.path().join("two.md"))?;
	assert!(
		two.contains("Stale beta."),
		"out-of-scope file must stay stale"
	);

	// A later full update still fixes the rest.
	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"update_block_scopes_to_matching_consumers__then_full_stdout",
			common::mdt_cmd_for_path(tmp.path()).arg("update")
		);
	});
	let two = std::fs::read_to_string(tmp.path().join("two.md"))?;
	assert!(two.contains("Beta content."));

	Ok(())
}

#[test]
fn update_block_with_unknown_name_fails_with_suggestions() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("update_scoped", tmp.path());

	common::mdt_cmd()
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.arg("--block")
		.arg("alpga")
		.assert()
		.code(2)
		.stderr(predicates::str::contains("unknown block name(s)"))
		.stderr(predicates::str::contains("alpga"))
		.stderr(predicates::str::contains("alpha"));

	let one = std::fs::read_to_string(tmp.path().join("one.md"))?;
	assert!(
		one.contains("Stale alpha."),
		"failed scoped update must not write"
	);

	Ok(())
}

#[test]
fn update_file_scopes_to_matching_file() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("update_scoped", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"update_file_scopes_to_matching_file__stdout",
			common::mdt_cmd_for_path(tmp.path())
				.arg("update")
				.arg("--file")
				.arg(tmp.path().join("one.md"))
		);
	});

	let one = std::fs::read_to_string(tmp.path().join("one.md"))?;
	assert!(one.contains("Alpha content."));
	let two = std::fs::read_to_string(tmp.path().join("two.md"))?;
	assert!(
		two.contains("Stale beta."),
		"out-of-scope file must stay stale"
	);

	Ok(())
}

#[test]
fn update_file_outside_project_root_fails() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("update_scoped", tmp.path());
	let outside = tmp
		.path()
		.parent()
		.unwrap_or(tmp.path())
		.join("elsewhere.md");

	common::mdt_cmd()
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.arg("--file")
		.arg(&outside)
		.assert()
		.code(2)
		.stderr(predicates::str::contains("is outside the project root"));

	Ok(())
}

#[test]
fn update_file_matching_no_scanned_file_fails() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("update_scoped", tmp.path());

	common::mdt_cmd()
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.arg("--file")
		.arg(tmp.path().join("nope.md"))
		.assert()
		.code(2)
		.stderr(predicates::str::contains(
			"no scanned file with consumer blocks matches",
		));

	Ok(())
}

#[test]
fn update_dry_run_scoped_to_block_keeps_files() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("update_scoped", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"update_dry_run_scoped_to_block__stdout",
			common::mdt_cmd_for_path(tmp.path())
				.arg("update")
				.arg("--dry-run")
				.arg("--block")
				.arg("alpha")
		);
	});

	let one = std::fs::read_to_string(tmp.path().join("one.md"))?;
	assert!(one.contains("Stale alpha."), "dry run must not write");

	Ok(())
}

#[test]
fn update_scoped_to_in_sync_block_reports_no_changes() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("update_scoped", tmp.path());

	common::mdt_cmd()
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.arg("--block")
		.arg("alpha")
		.assert()
		.success();

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			"update_scoped_to_in_sync_block__stdout",
			common::mdt_cmd_for_path(tmp.path())
				.arg("update")
				.arg("--block")
				.arg("alpha")
		);
	});

	Ok(())
}

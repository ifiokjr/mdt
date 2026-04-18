mod common;

use mdt_core::MdtResult;
use mdt_core::check_project;
use mdt_core::compute_updates;
use mdt_core::project::scan_project_with_config;
use rstest::rstest;

fn load_fixture(name: &str) -> (tempfile::TempDir, mdt_core::project::ProjectContext) {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	common::copy_fixture(name, tmp.path());
	let ctx = scan_project_with_config(tmp.path()).unwrap_or_else(|e| panic!("scan {name}: {e}"));
	(tmp, ctx)
}

#[rstest]
#[case("lenient_whitespace_only")]
#[case("lenient_extra_blank_lines")]
#[case("lenient_trailing_whitespace")]
#[case("lenient_mixed_blank_counts")]
#[case("lenient_trailing_newline_diff")]
fn lenient_whitespace_only_differences_pass(#[case] fixture: &str) -> MdtResult<()> {
	let (_tmp, ctx) = load_fixture(fixture);
	let result = check_project(&ctx)?;

	assert!(result.is_ok(), "{fixture} should be treated as up to date");
	insta::allow_duplicates! {
		insta::assert_debug_snapshot!(result.stale, @r#"[]"#);
	}

	Ok(())
}

#[rstest]
#[case("lenient_word_change", "lenient_word_change_is_stale")]
#[case("lenient_added_line", "lenient_added_line_is_stale")]
#[case("lenient_removed_line", "lenient_removed_line_is_stale")]
#[case(
	"lenient_completely_different",
	"lenient_completely_different_is_stale"
)]
#[case("lenient_code_block_change", "lenient_code_block_change_is_stale")]
#[case("lenient_inline_change", "lenient_inline_change_is_stale")]
#[case(
	"lenient_mixed_stale_clean",
	"lenient_mixed_stale_clean_detects_only_content_change"
)]
fn lenient_content_changes_are_snapshotted(
	#[case] fixture: &str,
	#[case] snapshot_name: &str,
) -> MdtResult<()> {
	let (tmp, ctx) = load_fixture(fixture);
	let result = check_project(&ctx)?;

	assert!(!result.is_ok(), "{fixture} should be stale");
	common::with_redacted_temp_dir(tmp.path(), || {
		insta::assert_debug_snapshot!(snapshot_name, &result.stale);
	});

	Ok(())
}

#[test]
fn lenient_update_writes_exact_source_bytes() -> MdtResult<()> {
	let (tmp, ctx) = load_fixture("lenient_word_change");
	let updates = compute_updates(&ctx)?;

	insta::assert_debug_snapshot!(updates.updated_count, @r#"1"#);

	let content = updates
		.updated_files
		.get(&tmp.path().join("readme.md"))
		.unwrap_or_else(|| panic!("expected updated readme"));
	insta::assert_snapshot!(
		"lenient_update_writes_exact_source_bytes__readme_md",
		content
	);

	Ok(())
}

#[rstest]
#[case("strict_identical")]
fn strict_identical_content_passes(#[case] fixture: &str) -> MdtResult<()> {
	let (_tmp, ctx) = load_fixture(fixture);
	let result = check_project(&ctx)?;

	assert!(result.is_ok(), "{fixture} should be byte-identical");
	insta::allow_duplicates! {
		insta::assert_debug_snapshot!(result.stale, @r#"[]"#);
	}

	Ok(())
}

#[rstest]
#[case("strict_extra_blank_lines", "strict_extra_blank_lines_is_stale")]
#[case("strict_trailing_whitespace", "strict_trailing_whitespace_is_stale")]
#[case("strict_single_extra_newline", "strict_single_extra_newline_is_stale")]
#[case("strict_content_change", "strict_content_change_is_stale")]
#[case("strict_multiple_blocks", "strict_multiple_blocks_all_detected")]
fn strict_differences_are_snapshotted(
	#[case] fixture: &str,
	#[case] snapshot_name: &str,
) -> MdtResult<()> {
	let (tmp, ctx) = load_fixture(fixture);
	let result = check_project(&ctx)?;

	assert!(!result.is_ok(), "{fixture} should be stale");
	common::with_redacted_temp_dir(tmp.path(), || {
		insta::assert_debug_snapshot!(snapshot_name, &result.stale);
	});

	Ok(())
}

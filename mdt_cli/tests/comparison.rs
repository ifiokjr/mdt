mod common;

use mdt_core::MdtResult;
use mdt_core::check_project;
use mdt_core::compute_updates;
use mdt_core::project::scan_project_with_config;

fn load_fixture(name: &str) -> (tempfile::TempDir, mdt_core::project::ProjectContext) {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	common::copy_fixture(name, tmp.path());
	let ctx = scan_project_with_config(tmp.path()).unwrap_or_else(|e| panic!("scan {name}: {e}"));
	(tmp, ctx)
}

// ===================================================================
// Lenient mode: whitespace-only differences MUST pass
// ===================================================================

#[test]
fn lenient_whitespace_only_passes() -> MdtResult<()> {
	let (_tmp, ctx) = load_fixture("lenient_whitespace_only");
	let result = check_project(&ctx)?;
	assert!(
		result.is_ok(),
		"lenient should pass with whitespace-only diff"
	);
	Ok(())
}

#[test]
fn lenient_extra_blank_lines_passes() -> MdtResult<()> {
	let (_tmp, ctx) = load_fixture("lenient_extra_blank_lines");
	let result = check_project(&ctx)?;
	assert!(result.is_ok(), "lenient should ignore extra blank lines");
	Ok(())
}

#[test]
fn lenient_trailing_whitespace_passes() -> MdtResult<()> {
	let (_tmp, ctx) = load_fixture("lenient_trailing_whitespace");
	let result = check_project(&ctx)?;
	assert!(result.is_ok(), "lenient should ignore trailing whitespace");
	Ok(())
}

#[test]
fn lenient_mixed_blank_counts_passes() -> MdtResult<()> {
	let (_tmp, ctx) = load_fixture("lenient_mixed_blank_counts");
	let result = check_project(&ctx)?;
	assert!(
		result.is_ok(),
		"lenient should ignore different blank line counts"
	);
	Ok(())
}

#[test]
fn lenient_trailing_newline_diff_passes() -> MdtResult<()> {
	let (_tmp, ctx) = load_fixture("lenient_trailing_newline_diff");
	let result = check_project(&ctx)?;
	assert!(
		result.is_ok(),
		"lenient should ignore trailing newline differences"
	);
	Ok(())
}

// ===================================================================
// Lenient mode: content changes MUST be detected
// ===================================================================

#[test]
fn lenient_word_change_is_stale() -> MdtResult<()> {
	let (_tmp, ctx) = load_fixture("lenient_word_change");
	let result = check_project(&ctx)?;
	assert_eq!(result.stale.len(), 1, "lenient must detect changed words");
	assert_eq!(result.stale[0].block_name, "docs");
	Ok(())
}

#[test]
fn lenient_added_line_is_stale() -> MdtResult<()> {
	let (_tmp, ctx) = load_fixture("lenient_added_line");
	let result = check_project(&ctx)?;
	assert_eq!(result.stale.len(), 1, "lenient must detect added lines");
	Ok(())
}

#[test]
fn lenient_removed_line_is_stale() -> MdtResult<()> {
	let (_tmp, ctx) = load_fixture("lenient_removed_line");
	let result = check_project(&ctx)?;
	assert_eq!(result.stale.len(), 1, "lenient must detect removed lines");
	Ok(())
}

#[test]
fn lenient_completely_different_is_stale() -> MdtResult<()> {
	let (_tmp, ctx) = load_fixture("lenient_completely_different");
	let result = check_project(&ctx)?;
	assert_eq!(
		result.stale.len(),
		1,
		"lenient must detect completely different content"
	);
	Ok(())
}

#[test]
fn lenient_code_block_change_is_stale() -> MdtResult<()> {
	let (_tmp, ctx) = load_fixture("lenient_code_block_change");
	let result = check_project(&ctx)?;
	assert_eq!(
		result.stale.len(),
		1,
		"lenient must detect changed code block content"
	);
	Ok(())
}

#[test]
fn lenient_inline_change_is_stale() -> MdtResult<()> {
	let (_tmp, ctx) = load_fixture("lenient_inline_change");
	let result = check_project(&ctx)?;
	assert_eq!(
		result.stale.len(),
		1,
		"lenient must detect inline block content change"
	);
	assert_eq!(result.stale[0].block_name, "ver");
	Ok(())
}

#[test]
fn lenient_mixed_stale_clean_detects_only_content_change() -> MdtResult<()> {
	let (_tmp, ctx) = load_fixture("lenient_mixed_stale_clean");
	let result = check_project(&ctx)?;
	assert_eq!(result.stale.len(), 1, "only beta should be stale");
	assert_eq!(result.stale[0].block_name, "beta");
	Ok(())
}

// ===================================================================
// Lenient mode: update still writes exact bytes
// ===================================================================

#[test]
fn lenient_update_writes_exact_source_bytes() -> MdtResult<()> {
	let (tmp, ctx) = load_fixture("lenient_word_change");
	let updates = compute_updates(&ctx)?;
	assert_eq!(updates.updated_count, 1);
	let content = updates
		.updated_files
		.get(&tmp.path().join("readme.md"))
		.unwrap_or_else(|| panic!("expected updated readme"));
	assert!(
		content.contains("Install with npm."),
		"update must write exact source content, not normalized"
	);
	Ok(())
}

// ===================================================================
// Strict mode (default): whitespace differences MUST be detected
// ===================================================================

#[test]
fn strict_extra_blank_lines_is_stale() -> MdtResult<()> {
	let (_tmp, ctx) = load_fixture("strict_extra_blank_lines");
	let result = check_project(&ctx)?;
	assert_eq!(
		result.stale.len(),
		1,
		"strict must detect extra blank lines"
	);
	Ok(())
}

#[test]
fn strict_trailing_whitespace_is_stale() -> MdtResult<()> {
	let (_tmp, ctx) = load_fixture("strict_trailing_whitespace");
	let result = check_project(&ctx)?;
	assert_eq!(
		result.stale.len(),
		1,
		"strict must detect trailing whitespace"
	);
	Ok(())
}

#[test]
fn strict_single_extra_newline_is_stale() -> MdtResult<()> {
	let (_tmp, ctx) = load_fixture("strict_single_extra_newline");
	let result = check_project(&ctx)?;
	assert_eq!(
		result.stale.len(),
		1,
		"strict must detect single extra newline"
	);
	Ok(())
}

#[test]
fn strict_identical_passes() -> MdtResult<()> {
	let (_tmp, ctx) = load_fixture("strict_identical");
	let result = check_project(&ctx)?;
	assert!(
		result.is_ok(),
		"strict should pass when content is byte-identical"
	);
	Ok(())
}

#[test]
fn strict_content_change_is_stale() -> MdtResult<()> {
	let (_tmp, ctx) = load_fixture("strict_content_change");
	let result = check_project(&ctx)?;
	assert_eq!(result.stale.len(), 1, "strict must detect content change");
	Ok(())
}

#[test]
fn strict_multiple_blocks_all_detected() -> MdtResult<()> {
	let (_tmp, ctx) = load_fixture("strict_multiple_blocks");
	let result = check_project(&ctx)?;
	assert_eq!(
		result.stale.len(),
		2,
		"strict must detect whitespace diff in both blocks"
	);
	Ok(())
}

//! Formatting evals: template-side vs output-side table formatting.
//!
//! These tests pin the contract `[format] tables` exists for: with the
//! feature on, `mdt update` aligns provider tables inside `*.t.md` files and
//! injects the same aligned bytes into consumers — including source-code
//! comment consumers — so an external markdown formatter such as dprint
//! finds nothing left to change, without ignoring a single file.
//!
//! When dprint is on `PATH`, the suite also runs the real formatter over
//! mdt's output (using this repository's own dprint config) and asserts it is
//! a no-op. That is the symbiosis eval: internal formatting and the external
//! formatter must agree byte-for-byte.

mod common;

use std::io::Write as _;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

/// Read a file with line endings normalized to `\n`.
///
/// Windows runners check fixtures out with CRLF and mdt preserves a file's
/// line endings, so the assertions below compare LF-normalized text.
fn read_text(path: impl AsRef<Path>) -> std::io::Result<String> {
	std::fs::read_to_string(path).map(|content| content.replace("\r\n", "\n"))
}

fn block_content(file: &str, opening: &str, closing: &str) -> String {
	let start = file.find(opening).unwrap_or_else(|| {
		panic!("opening tag {opening} not found in:\n{file}");
	}) + opening.len();
	let end = start
		+ file[start..]
			.find(closing)
			.unwrap_or_else(|| panic!("closing tag {closing} not found in:\n{file}"));

	file[start..end].to_string()
}

fn run_update(tmp: &Path) {
	let output = common::mdt_cmd_for_path(tmp).arg("update").output();
	let output = output.unwrap_or_else(|error| panic!("run mdt update: {error}"));

	assert!(
		output.status.success(),
		"mdt update failed:\n{}",
		String::from_utf8_lossy(&output.stderr)
	);
}

fn run_check(tmp: &Path) -> bool {
	let output = common::mdt_cmd_for_path(tmp).arg("check").output();
	let output = output.unwrap_or_else(|error| panic!("run mdt check: {error}"));

	output.status.success()
}

/// dprint is available on `PATH`; when it is missing (for example in the
/// cross-platform CI jobs) the equivalence evals skip.
fn dprint_available() -> bool {
	Command::new("dprint")
		.arg("--version")
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.status()
		.is_ok_and(|status| status.success())
}

/// Run `dprint fmt` over `content`, resolving plugins and config against this
/// repository's dprint setup.
fn dprint_format(content: &str) -> String {
	let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.unwrap_or_else(|| panic!("mdt_cli sits inside the workspace root"));

	let mut child = Command::new("dprint")
		.arg("fmt")
		.arg("--stdin")
		.arg("formatting-eval-probe.md")
		.current_dir(repo_root)
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.unwrap_or_else(|error| panic!("spawn dprint: {error}"));

	child
		.stdin
		.take()
		.unwrap_or_else(|| panic!("dprint stdin"))
		.write_all(content.as_bytes())
		.unwrap_or_else(|error| panic!("write dprint stdin: {error}"));
	let output = child
		.wait_with_output()
		.unwrap_or_else(|error| panic!("dprint output: {error}"));

	assert!(
		output.status.success(),
		"dprint fmt failed:\n{}",
		String::from_utf8_lossy(&output.stderr)
	);

	String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn format_eval_update_aligns_template_and_output_tables() {
	let tmp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
	common::copy_fixture("format_eval_tables", tmp.path());

	assert!(
		!run_check(tmp.path()),
		"check must report the unaligned provider table as stale"
	);

	run_update(tmp.path());

	let template = read_text(tmp.path().join("template.t.md"))
		.unwrap_or_else(|error| panic!("read template: {error}"));
	let readme = read_text(tmp.path().join("readme.md"))
		.unwrap_or_else(|error| panic!("read readme: {error}"));

	let provider_block = block_content(&template, "<!-- {@table} -->", "<!-- {/table} -->");
	let consumer_block = block_content(&readme, "<!-- {=table} -->", "<!-- {/table} -->");

	assert_eq!(
		provider_block, consumer_block,
		"template and output must carry identical table bytes"
	);

	assert_eq!(
		provider_block,
		"\n\n| Key        |     Align      | Value       | Notes |\n| ---------- | :------------: \
		 | ----------- | ----: |\n| `one`      |   center-me    | first row   | right |\n| \
		 `three`    | 名前のテキスト | third value |     r |\n| `esc\\|ape` |       c        | a \
		 \\| b      |     x |\n\n",
		"aligned table must match dprint's column padding"
	);

	assert!(run_check(tmp.path()), "check must pass after update");

	// A second update must be a no-op.
	run_update(tmp.path());
	let template_again = read_text(tmp.path().join("template.t.md"))
		.unwrap_or_else(|error| panic!("read template: {error}"));
	let readme_again = read_text(tmp.path().join("readme.md"))
		.unwrap_or_else(|error| panic!("read readme: {error}"));
	similar_asserts::assert_eq!(template, template_again);
	similar_asserts::assert_eq!(readme, readme_again);
}

#[test]
fn format_eval_preset_enables_table_and_whitespace_rules() {
	let tmp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
	common::copy_fixture("format_eval_preset", tmp.path());

	assert!(
		!run_check(tmp.path()),
		"check must report the preset-normalized provider as stale"
	);

	run_update(tmp.path());

	let template = read_text(tmp.path().join("template.t.md"))
		.unwrap_or_else(|error| panic!("read template: {error}"));
	let readme = read_text(tmp.path().join("readme.md"))
		.unwrap_or_else(|error| panic!("read readme: {error}"));

	let provider_block = block_content(&template, "<!-- {@mixed} -->", "<!-- {/mixed} -->");
	let consumer_block = block_content(&readme, "<!-- {=mixed} -->", "<!-- {/mixed} -->");

	assert_eq!(
		provider_block, consumer_block,
		"template and output must carry identical bytes under the preset"
	);

	assert!(
		provider_block.contains("| `one`   | first row   |"),
		"table must be aligned with cells padded to column width: {provider_block}"
	);
	assert!(
		provider_block.contains("Trailing spaces on this line\n"),
		"prose lines must lose their trailing whitespace: {provider_block}"
	);
	assert!(
		provider_block.contains("kept as-is   \n"),
		"trailing whitespace inside fenced code blocks must survive: {provider_block}"
	);

	assert!(run_check(tmp.path()), "check must pass after update");

	// A second update must be a no-op.
	run_update(tmp.path());
	let template_again = read_text(tmp.path().join("template.t.md"))
		.unwrap_or_else(|error| panic!("read template: {error}"));
	similar_asserts::assert_eq!(template, template_again);
}

#[test]
fn format_eval_disabled_by_default_preserves_provider_bytes() {
	let tmp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
	common::copy_fixture("format_eval_tables_disabled", tmp.path());

	run_update(tmp.path());

	let template = read_text(tmp.path().join("template.t.md"))
		.unwrap_or_else(|error| panic!("read template: {error}"));
	let readme = read_text(tmp.path().join("readme.md"))
		.unwrap_or_else(|error| panic!("read readme: {error}"));
	let source_template = read_text(
		Path::new(env!("CARGO_MANIFEST_DIR"))
			.join("tests/fixtures/format_eval_tables_disabled/template.t.md"),
	)
	.unwrap_or_else(|error| panic!("read fixture template: {error}"));

	similar_asserts::assert_eq!(
		template,
		source_template,
		"without [format] tables the template file stays untouched"
	);

	assert_eq!(
		block_content(&readme, "<!-- {=table} -->", "<!-- {/table} -->"),
		block_content(&source_template, "<!-- {@table} -->", "<!-- {/table} -->"),
		"without [format] tables consumers receive provider bytes verbatim"
	);
}

#[test]
fn format_eval_tables_normalize_before_comment_transformers() {
	let tmp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
	common::copy_fixture("format_eval_tables_source", tmp.path());

	run_update(tmp.path());

	let lib =
		read_text(tmp.path().join("lib.rs")).unwrap_or_else(|error| panic!("read lib.rs: {error}"));

	assert_eq!(
		block_content(
			&lib,
			"<!-- {=rustdoc|trim|linePrefix:\"/// \":true} -->",
			"<!-- {/rustdoc} -->"
		),
		// The trailing `/// ` is the closing tag's own comment prefix, which
		// the extraction helper includes.
		"\n/// | Param   | Description                |\n/// | ------- | \
		 -------------------------- |\n/// | `count` | number of items to process |\n/// | `name`  \
		 | identifier                 |\n/// ",
		"tables align before linePrefix wraps them in doc-comment prefixes"
	);

	run_update(tmp.path());
	let lib_again =
		read_text(tmp.path().join("lib.rs")).unwrap_or_else(|error| panic!("read lib.rs: {error}"));
	similar_asserts::assert_eq!(lib, lib_again);
}

#[test]
fn format_eval_tables_leave_fenced_code_blocks_untouched() {
	let tmp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
	common::copy_fixture("format_eval_tables_fence", tmp.path());

	run_update(tmp.path());

	let readme = read_text(tmp.path().join("readme.md"))
		.unwrap_or_else(|error| panic!("read readme: {error}"));
	let consumer_block = block_content(&readme, "<!-- {=mixed} -->", "<!-- {/mixed} -->");

	assert!(
		consumer_block.contains("| untouched | fence |\n| --------- | ----- |\n| raw | bytes |"),
		"fenced example tables keep their bytes:\n{consumer_block}"
	);
	assert!(
		consumer_block.contains("| real    | table |\n| ------- | ----- |\n| aligned | yes   |"),
		"real tables outside fences are aligned:\n{consumer_block}"
	);

	let template = read_text(tmp.path().join("template.t.md"))
		.unwrap_or_else(|error| panic!("read template: {error}"));
	assert_eq!(
		block_content(&template, "<!-- {@mixed} -->", "<!-- {/mixed} -->"),
		consumer_block,
		"template and output stay byte-identical, fence included"
	);
}

#[test]
fn format_eval_output_matches_real_dprint_when_available() {
	if !dprint_available() {
		eprintln!("skipping: dprint is not on PATH");
		return;
	}

	let tmp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
	common::copy_fixture("format_eval_tables", tmp.path());

	run_update(tmp.path());

	let readme = read_text(tmp.path().join("readme.md"))
		.unwrap_or_else(|error| panic!("read readme: {error}"));
	let template = read_text(tmp.path().join("template.t.md"))
		.unwrap_or_else(|error| panic!("read template: {error}"));

	// The external formatter must be a no-op on both sides. Combined with the
	// exact-bytes assertions above, this proves the two formatters agree:
	// dprint's canonical form of the unaligned fixture equals what mdt wrote.
	similar_asserts::assert_eq!(
		dprint_format(&readme),
		readme,
		"dprint disagrees with mdt's table alignment in the output file"
	);
	similar_asserts::assert_eq!(
		dprint_format(&template),
		template,
		"dprint disagrees with mdt's table alignment in the template file"
	);
}

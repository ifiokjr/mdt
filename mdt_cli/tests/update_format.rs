mod common;

use serde_json::Value;

fn run_update_json(tmp: &std::path::Path, extra_args: &[&str]) -> (i32, Value, String) {
	let mut cmd = common::mdt_std_cmd();
	cmd.arg("--path")
		.arg(tmp)
		.arg("update")
		.arg("--format")
		.arg("json");

	for arg in extra_args {
		cmd.arg(arg);
	}

	let output = cmd
		.output()
		.unwrap_or_else(|error| panic!("run mdt update --format json: {error}"));
	let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
	let report: Value = serde_json::from_str(&stdout)
		.unwrap_or_else(|error| panic!("stdout is not JSON ({error}): {stdout}"));

	(
		output.status.code().unwrap_or(-1),
		report,
		String::from_utf8_lossy(&output.stderr).into_owned(),
	)
}

#[test]
fn update_format_json_reports_written_updates() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_stale", tmp.path());

	let (code, report, _) = run_update_json(tmp.path(), &[]);

	assert_eq!(code, 0);
	assert_eq!(report["ok"], Value::Bool(true));
	assert_eq!(report["dry_run"], Value::Bool(false));
	assert!(
		report["updated_blocks"].as_u64().unwrap_or(0) >= 1,
		"expected at least one updated block: {report}"
	);
	let files = report["updated_files"].as_array().expect("updated_files");
	assert!(
		files.iter().any(|file| {
			file.as_str()
				.is_some_and(|file| file.ends_with("readme.md"))
		}),
		"expected readme.md in updated_files: {report}"
	);

	// The update really wrote the file.
	let readme = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert!(!readme.contains("Old content."), "readme was not updated");

	Ok(())
}

#[test]
fn update_format_json_dry_run_previews_without_writing() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_stale", tmp.path());
	let before = std::fs::read_to_string(tmp.path().join("readme.md"))?;

	let (code, report, _) = run_update_json(tmp.path(), &["--dry-run"]);

	assert_eq!(code, 0);
	assert_eq!(report["ok"], Value::Bool(true));
	assert_eq!(report["dry_run"], Value::Bool(true));
	assert!(
		report["updated_blocks"].as_u64().unwrap_or(0) >= 1,
		"expected at least one pending block: {report}"
	);

	let after = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert_eq!(before, after, "dry run must not write files");

	Ok(())
}

#[test]
fn update_format_json_reports_clean_project() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_up_to_date", tmp.path());

	let (code, report, _) = run_update_json(tmp.path(), &[]);

	assert_eq!(code, 0);
	assert_eq!(report["ok"], Value::Bool(true));
	assert_eq!(report["updated_blocks"], Value::from(0u64));
	assert_eq!(report["updated_files"], Value::Array(Vec::new()));
	assert_eq!(report["errors"], Value::Array(Vec::new()));

	Ok(())
}

#[test]
fn update_format_json_reports_validation_errors() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@greeting} -->\n\nHello!\n\n<!-- {/greeting} -->\n",
	)?;
	// An unclosed consumer block stops the update with status 2.
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=greeting} -->\n\nStale content.\n",
	)?;

	let (code, report, _) = run_update_json(tmp.path(), &[]);

	assert_eq!(code, 2);
	assert_eq!(report["ok"], Value::Bool(false));
	let diagnostics = report["diagnostics"].as_array().expect("diagnostics");
	assert!(
		diagnostics.iter().any(|diag| diag["severity"] == "error"),
		"expected an error diagnostic: {report}"
	);

	Ok(())
}

#[test]
fn update_format_json_reports_config_errors() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	std::fs::write(tmp.path().join("mdt.toml"), "[templates\nbroken\n")?;

	let (code, report, _) = run_update_json(tmp.path(), &[]);

	assert_eq!(code, 2);
	assert_eq!(report["ok"], Value::Bool(false));
	assert!(
		!report["diagnostics"]
			.as_array()
			.expect("diagnostics")
			.is_empty(),
		"expected a diagnostic for the broken config: {report}"
	);

	Ok(())
}

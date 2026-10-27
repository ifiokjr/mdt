mod common;

use serde_json::Value;

#[test]
fn list_text_output_shows_providers_and_consumers() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_up_to_date", tmp.path());

	let output = common::mdt_cmd_for_path(tmp.path()).arg("list").output()?;

	assert!(output.status.success());
	let stdout = String::from_utf8_lossy(&output.stdout);
	assert!(stdout.contains("Providers:"), "missing providers: {stdout}");
	assert!(
		stdout.contains("@greeting"),
		"missing provider name: {stdout}"
	);
	assert!(stdout.contains("Consumers:"), "missing consumers: {stdout}");
	assert!(
		stdout.contains("=greeting"),
		"missing consumer name: {stdout}"
	);
	assert!(stdout.contains("[linked]"), "missing link status: {stdout}");

	Ok(())
}

#[test]
fn list_format_json_describes_every_block() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_up_to_date", tmp.path());

	let output = common::mdt_cmd_for_path(tmp.path())
		.arg("list")
		.arg("--format")
		.arg("json")
		.output()?;

	assert!(output.status.success());

	let stdout = String::from_utf8_lossy(&output.stdout);
	let report: Value = serde_json::from_str(&stdout)
		.unwrap_or_else(|error| panic!("stdout is not JSON ({error}): {stdout}"));

	let providers = report["providers"].as_array().expect("providers");
	assert_eq!(providers.len(), 1);
	assert_eq!(providers[0]["name"], Value::String("greeting".to_string()));
	assert!(
		providers[0]["file"]
			.as_str()
			.is_some_and(|file| file.ends_with("template.t.md"))
	);
	assert!(
		providers[0]["consumers"].as_u64().unwrap_or(0) >= 1,
		"expected a linked consumer: {report}"
	);

	let consumers = report["consumers"].as_array().expect("consumers");
	assert_eq!(consumers.len(), 1);
	assert_eq!(consumers[0]["type"], Value::String("consumer".to_string()));
	assert_eq!(consumers[0]["status"], Value::String("linked".to_string()));

	assert_eq!(
		report["summary"]["providers"],
		Value::from(1u64),
		"unexpected summary: {report}"
	);
	assert_eq!(
		report["summary"]["consumers"],
		Value::from(1u64),
		"unexpected summary: {report}"
	);
	assert_eq!(report["diagnostics"], Value::Array(Vec::new()));

	Ok(())
}

#[test]
fn list_format_json_marks_orphan_consumers() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	// A consumer whose provider does not exist anywhere.
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=ghost} -->\n\nContent.\n\n<!-- {/ghost} -->\n",
	)?;

	let output = common::mdt_cmd_for_path(tmp.path())
		.arg("list")
		.arg("--format")
		.arg("json")
		.output()?;

	// Orphans are warnings; the listing still succeeds.
	assert!(output.status.success());

	let stdout = String::from_utf8_lossy(&output.stdout);
	let report: Value = serde_json::from_str(&stdout)
		.unwrap_or_else(|error| panic!("stdout is not JSON ({error}): {stdout}"));

	let consumers = report["consumers"].as_array().expect("consumers");
	assert_eq!(consumers[0]["name"], Value::String("ghost".to_string()));
	assert_eq!(consumers[0]["status"], Value::String("orphan".to_string()));

	Ok(())
}

#[test]
fn list_format_json_reports_validation_errors() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@greeting} -->\n\nHello!\n\n<!-- {/greeting} -->\n",
	)?;
	// An unclosed consumer block is a validation error; the listing still
	// prints but the command exits with status 2.
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=greeting} -->\n\nStale content.\n",
	)?;

	let output = common::mdt_cmd_for_path(tmp.path())
		.arg("list")
		.arg("--format")
		.arg("json")
		.output()?;

	assert_eq!(output.status.code(), Some(2));

	let stdout = String::from_utf8_lossy(&output.stdout);
	let report: Value = serde_json::from_str(&stdout)
		.unwrap_or_else(|error| panic!("stdout is not JSON ({error}): {stdout}"));

	assert!(
		report["diagnostics"]
			.as_array()
			.expect("diagnostics")
			.iter()
			.any(|diag| diag["severity"] == "error"),
		"expected an error diagnostic: {report}"
	);
	assert!(
		report["providers"].as_array().expect("providers").len() == 1,
		"the listing must still describe blocks: {report}"
	);

	Ok(())
}

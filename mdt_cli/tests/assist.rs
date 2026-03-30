mod common;

use mdt_core::AnyEmptyResult;
use predicates::prelude::*;

#[test]
fn assist_text_prints_mcp_snippet_and_guidance() {
	let mut cmd = common::mdt_cmd();
	cmd.arg("assist").arg("claude");
	cmd.assert()
		.success()
		.stdout(predicate::str::contains("Claude"))
		.stdout(predicate::str::contains("\"mcpServers\": {"))
		.stdout(predicate::str::contains("\"command\": \"mdt\""))
		.stdout(predicate::str::contains("Prefer reuse before creation"))
		.stdout(predicate::str::contains(
			"Use `.templates/` as the canonical location",
		));
}

#[test]
fn assist_json_prints_machine_readable_profile() -> AnyEmptyResult {
	let mut cmd = common::mdt_cmd();
	let output = cmd
		.arg("assist")
		.arg("pi")
		.arg("--format")
		.arg("json")
		.assert()
		.success()
		.get_output()
		.stdout
		.clone();

	let json: serde_json::Value = serde_json::from_slice(&output)?;
	assert_eq!(json["assistant"], "Pi");
	assert_eq!(json["mcp_config"]["mcpServers"]["mdt"]["command"], "mdt");
	assert_eq!(json["mcp_config"]["mcpServers"]["mdt"]["args"][0], "mcp");
	assert!(
		json["repo_guidance"]
			.as_array()
			.is_some_and(|items| !items.is_empty())
	);
	assert!(
		json["notes"]
			.as_array()
			.is_some_and(|items| !items.is_empty())
	);

	Ok(())
}

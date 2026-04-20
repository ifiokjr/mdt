mod common;

use insta_cmd::assert_cmd_snapshot;
use rstest::rstest;

#[rstest]
#[case("claude", None, "assist_text_prints_mcp_snippet_and_guidance")]
#[case("pi", Some("json"), "assist_json_prints_machine_readable_profile")]
fn assist_outputs_expected_profiles(
	#[case] assistant: &str,
	#[case] format: Option<&str>,
	#[case] snapshot_name: &str,
) {
	let mut cmd = common::mdt_std_cmd();
	cmd.arg("assist");
	cmd.arg(assistant);

	if let Some(format) = format {
		cmd.arg("--format");
		cmd.arg(format);
	}

	assert_cmd_snapshot!(snapshot_name, cmd);
}

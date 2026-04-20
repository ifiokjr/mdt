mod common;

use insta_cmd::assert_cmd_snapshot;
use rstest::rstest;

#[rstest]
#[case("config_dot_mdt_toml", "info_resolves_dot_mdt_toml")]
#[case("config_dot_config_mdt_toml", "info_resolves_dot_config_mdt_toml")]
#[case("config_all_candidates", "info_prefers_mdt_toml_over_other_candidates")]
fn info_resolves_config_candidates(
	#[case] fixture: &str,
	#[case] snapshot_name: &str,
) -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture(fixture, tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			snapshot_name,
			common::mdt_cmd_for_path(tmp.path()).arg("info")
		);
	});

	Ok(())
}

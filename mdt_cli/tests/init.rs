mod common;

use insta_cmd::assert_cmd_snapshot;
use mdt_core::AnyEmptyResult;
use rstest::rstest;

fn assert_init_snapshot(fixture: &str, snapshot_name: &str) -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture(fixture, tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			snapshot_name,
			common::mdt_cmd_for_path(tmp.path()).arg("init")
		);
	});

	let template = std::fs::read_to_string(tmp.path().join("template.t.md"))?;
	insta::assert_snapshot!(format!("{snapshot_name}__template_t_md"), template);

	let config = std::fs::read_to_string(tmp.path().join("mdt.toml"))?;
	insta::assert_snapshot!(format!("{snapshot_name}__mdt_toml"), config);

	Ok(())
}

#[rstest]
#[case("init_overwrite_both", "init_does_not_overwrite")]
#[case(
	"init_existing_template_only",
	"init_creates_config_when_template_exists"
)]
fn init_preserves_existing_files_and_writes_missing_ones(
	#[case] fixture: &str,
	#[case] snapshot_name: &str,
) -> AnyEmptyResult {
	assert_init_snapshot(fixture, snapshot_name)
}

#[test]
fn init_creates_valid_template() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	common::mdt_cmd()
		.arg("init")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success();

	let template_content = std::fs::read_to_string(tmp.path().join(".templates/template.t.md"))?;
	let blocks = mdt_core::parse(&template_content)?;
	assert!(!blocks.is_empty(), "init should create at least one block");
	assert_eq!(blocks[0].r#type, mdt_core::BlockType::Provider);

	Ok(())
}

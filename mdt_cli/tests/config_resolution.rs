mod common;

use mdt_core::AnyEmptyResult;

#[test]
fn info_resolves_dot_mdt_toml() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	std::fs::write(tmp.path().join(".mdt.toml"), "")?;

	let expected_path = tmp.path().join(".mdt.toml").display().to_string();

	common::mdt_cmd()
		.arg("info")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("Resolved config"))
		.stdout(predicates::str::contains(expected_path));

	Ok(())
}

#[test]
fn info_resolves_dot_config_mdt_toml() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	std::fs::create_dir_all(tmp.path().join(".config"))?;
	std::fs::write(tmp.path().join(".config/mdt.toml"), "")?;

	let expected_path = tmp.path().join(".config/mdt.toml").display().to_string();

	common::mdt_cmd()
		.arg("info")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("Resolved config"))
		.stdout(predicates::str::contains(expected_path));

	Ok(())
}

#[test]
fn info_prefers_mdt_toml_over_other_candidates() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	std::fs::create_dir_all(tmp.path().join(".config"))?;
	std::fs::write(tmp.path().join("mdt.toml"), "")?;
	std::fs::write(tmp.path().join(".mdt.toml"), "")?;
	std::fs::write(tmp.path().join(".config/mdt.toml"), "")?;

	let expected_path = tmp.path().join("mdt.toml").display().to_string();

	common::mdt_cmd()
		.arg("info")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("Resolved config"))
		.stdout(predicates::str::contains(expected_path));

	Ok(())
}

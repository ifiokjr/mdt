mod common;

#[test]
fn info_resolves_dot_mdt_toml() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("config_dot_mdt_toml", tmp.path());

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
fn info_resolves_dot_config_mdt_toml() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("config_dot_config_mdt_toml", tmp.path());

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
fn info_prefers_mdt_toml_over_other_candidates() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("config_all_candidates", tmp.path());

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

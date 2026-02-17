use assert_cmd::Command;
use mdt::AnyEmptyResult;

#[test]
fn check_passes_when_up_to_date() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	// Create a provider template file
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@greeting} -->\n\nHello world!\n\n<!-- {/greeting} -->\n",
	)?;

	// Create a consumer file with matching content
	std::fs::write(
		tmp.path().join("readme.md"),
		"# Readme\n\n<!-- {=greeting} -->\n\nHello world!\n\n<!-- {/greeting} -->\n",
	)?;

	let mut cmd = Command::cargo_bin("mdt")?;
	cmd.arg("check")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("up to date"));

	Ok(())
}

#[test]
fn check_fails_when_stale() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	// Create a provider template file
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@greeting} -->\n\nHello world!\n\n<!-- {/greeting} -->\n",
	)?;

	// Create a consumer file with outdated content
	std::fs::write(
		tmp.path().join("readme.md"),
		"# Readme\n\n<!-- {=greeting} -->\n\nOld content.\n\n<!-- {/greeting} -->\n",
	)?;

	let mut cmd = Command::cargo_bin("mdt")?;
	cmd.arg("check")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.failure()
		.stderr(predicates::str::contains("out of date"));

	Ok(())
}

#[test]
fn check_with_no_blocks() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	std::fs::write(tmp.path().join("readme.md"), "# Just a readme\n")?;

	let mut cmd = Command::cargo_bin("mdt")?;
	cmd.arg("check")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("up to date"));

	Ok(())
}

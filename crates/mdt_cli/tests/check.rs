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
	cmd.env("NO_COLOR", "1")
		.arg("check")
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
	cmd.env("NO_COLOR", "1")
		.arg("check")
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
	cmd.env("NO_COLOR", "1")
		.arg("check")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("up to date"));

	Ok(())
}

#[test]
fn check_verbose_shows_provider_count() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\ncontent\n\n<!-- {/block} -->\n",
	)?;
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=block} -->\n\ncontent\n\n<!-- {/block} -->\n",
	)?;

	let mut cmd = Command::cargo_bin("mdt")?;
	cmd.env("NO_COLOR", "1")
		.arg("check")
		.arg("--verbose")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("1 provider(s)"))
		.stdout(predicates::str::contains("1 consumer(s)"));

	Ok(())
}

#[test]
fn check_warns_missing_provider() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	// Consumer with no matching provider
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=orphan} -->\n\nstuff\n\n<!-- {/orphan} -->\n",
	)?;

	let mut cmd = Command::cargo_bin("mdt")?;
	cmd.env("NO_COLOR", "1")
		.arg("check")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stderr(predicates::str::contains(
			"consumer block `orphan` has no matching provider",
		));

	Ok(())
}

#[test]
fn check_stale_shows_block_name_and_file() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@myBlock} -->\n\nnew\n\n<!-- {/myBlock} -->\n",
	)?;
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=myBlock} -->\n\nold\n\n<!-- {/myBlock} -->\n",
	)?;

	let mut cmd = Command::cargo_bin("mdt")?;
	cmd.env("NO_COLOR", "1")
		.arg("check")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.failure()
		.stderr(predicates::str::contains("Stale: block `myBlock`"))
		.stderr(predicates::str::contains("readme.md"));

	Ok(())
}

#[test]
fn check_multiple_stale_blocks() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@a} -->\n\nnew a\n\n<!-- {/a} -->\n\n<!-- {@b} -->\n\nnew b\n\n<!-- {/b} -->\n",
	)?;
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=a} -->\n\nold a\n\n<!-- {/a} -->\n\n<!-- {=b} -->\n\nold b\n\n<!-- {/b} -->\n",
	)?;

	let mut cmd = Command::cargo_bin("mdt")?;
	cmd.env("NO_COLOR", "1")
		.arg("check")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.failure()
		.stderr(predicates::str::contains("2 consumer block(s)"));

	Ok(())
}

use assert_cmd::Command;
use mdt::AnyEmptyResult;

#[test]
fn update_replaces_stale_content() -> AnyEmptyResult {
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
	cmd.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("Updated"));

	let content = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert!(content.contains("Hello world!"));
	assert!(!content.contains("Old content."));

	Ok(())
}

#[test]
fn update_noop_when_in_sync() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@greeting} -->\n\nHello world!\n\n<!-- {/greeting} -->\n",
	)?;

	std::fs::write(
		tmp.path().join("readme.md"),
		"# Readme\n\n<!-- {=greeting} -->\n\nHello world!\n\n<!-- {/greeting} -->\n",
	)?;

	let mut cmd = Command::cargo_bin("mdt")?;
	cmd.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("already up to date"));

	Ok(())
}

#[test]
fn update_dry_run_does_not_write() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@greeting} -->\n\nHello world!\n\n<!-- {/greeting} -->\n",
	)?;

	let consumer_content =
		"# Readme\n\n<!-- {=greeting} -->\n\nOld content.\n\n<!-- {/greeting} -->\n";
	std::fs::write(tmp.path().join("readme.md"), consumer_content)?;

	let mut cmd = Command::cargo_bin("mdt")?;
	cmd.arg("update")
		.arg("--dry-run")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("Dry run"));

	// File should not have changed
	let content = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert_eq!(content, consumer_content);

	Ok(())
}

#[test]
fn update_with_transformers() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@docs} -->\n\nSome documentation content.\n\n<!-- {/docs} -->\n",
	)?;

	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=docs|trim} -->\n\nold\n\n<!-- {/docs} -->\n",
	)?;

	let mut cmd = Command::cargo_bin("mdt")?;
	cmd.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success();

	let content = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert!(content.contains("Some documentation content."));

	Ok(())
}

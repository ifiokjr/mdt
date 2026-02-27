use assert_cmd::Command;
use mdt_cli::Commands;
use mdt_cli::MdtCli;
use mdt_core::AnyEmptyResult;
use predicates::prelude::PredicateBooleanExt;

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
		.stderr(predicates::str::contains("Stale consumers:"))
		.stderr(predicates::str::contains("block `myBlock`"))
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

#[test]
fn check_warns_undefined_template_variables() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npkg = \"package.json\"\n",
	)?;
	std::fs::write(
		tmp.path().join("package.json"),
		r#"{"name": "my-lib", "version": "1.0.0"}"#,
	)?;
	// Provider with a typo: "pkgg" instead of "pkg" — renders to "npm install "
	// (empty string for undefined variable due to Chainable behavior)
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@install} -->\n\nnpm install {{ pkgg.name }}\n\n<!-- {/install} -->\n",
	)?;
	// Consumer content must match the rendered output (empty string for undefined)
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=install} -->\n\nnpm install \n\n<!-- {/install} -->\n",
	)?;

	let mut cmd = Command::cargo_bin("mdt")?;
	cmd.env("NO_COLOR", "1")
		.arg("check")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stderr(predicates::str::contains("undefined variable(s)"))
		.stderr(predicates::str::contains("pkgg.name"));

	Ok(())
}

#[test]
fn check_no_warnings_for_valid_template_variables() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npkg = \"package.json\"\n",
	)?;
	std::fs::write(
		tmp.path().join("package.json"),
		r#"{"name": "my-lib", "version": "1.0.0"}"#,
	)?;
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@install} -->\n\nnpm install {{ pkg.name }}@{{ pkg.version }}\n\n<!-- {/install} \
		 -->\n",
	)?;
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=install} -->\n\nnpm install my-lib@1.0.0\n\n<!-- {/install} -->\n",
	)?;

	let mut cmd = Command::cargo_bin("mdt")?;
	cmd.env("NO_COLOR", "1")
		.arg("check")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		// Should not contain "undefined" in output
		.stderr(predicates::str::contains("undefined").not());

	Ok(())
}

#[test]
fn check_watch_flag_is_accepted_by_cli_parser() {
	use clap::Parser;

	// Verify the --watch flag parses correctly for the check command.
	let cli = MdtCli::parse_from(["mdt", "check", "--watch"]);
	match cli.command {
		Some(Commands::Check { watch, diff, .. }) => {
			assert!(watch);
			assert!(!diff);
		}
		_ => panic!("expected Check command"),
	}

	// Verify --watch defaults to false when not specified.
	let cli = MdtCli::parse_from(["mdt", "check"]);
	match cli.command {
		Some(Commands::Check { watch, .. }) => {
			assert!(!watch);
		}
		_ => panic!("expected Check command"),
	}
}

#[test]
fn check_watch_flag_accepted_by_binary() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@greeting} -->\n\nHello world!\n\n<!-- {/greeting} -->\n",
	)?;
	std::fs::write(
		tmp.path().join("readme.md"),
		"# Readme\n\n<!-- {=greeting} -->\n\nHello world!\n\n<!-- {/greeting} -->\n",
	)?;

	// We cannot test the full watch loop (it runs forever), but we can verify
	// the binary accepts --watch without error by checking that it produces the
	// expected "up to date" + "Watching" output before we kill it.
	let mut cmd = Command::cargo_bin("mdt")?;
	cmd.env("NO_COLOR", "1")
		.arg("check")
		.arg("--watch")
		.arg("--path")
		.arg(tmp.path())
		.timeout(std::time::Duration::from_secs(3))
		.assert()
		// The process will be killed by timeout, but stdout should contain
		// the initial check result and the watching message.
		.stdout(predicates::str::contains("up to date"))
		.stdout(predicates::str::contains("Watching for file changes"));

	Ok(())
}

#[test]
fn info_command_is_accepted_by_cli_parser() {
	use clap::Parser;

	let cli = MdtCli::parse_from(["mdt", "info"]);
	match cli.command {
		Some(Commands::Info) => {}
		_ => panic!("expected Info command"),
	}
}

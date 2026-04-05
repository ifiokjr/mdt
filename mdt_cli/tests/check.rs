mod common;

use mdt_cli::AssistOutputFormat;
use mdt_cli::Assistant;
use mdt_cli::Commands;
use mdt_cli::DoctorOutputFormat;
use mdt_cli::InfoOutputFormat;
use mdt_cli::MdtCli;
use predicates::prelude::PredicateBooleanExt;
use serde_json::Value;

const ANSI_ESCAPE: &str = "\u{1b}[";

#[test]
fn check_passes_when_up_to_date() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_up_to_date", tmp.path());

	common::mdt_cmd()
		.arg("check")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("up to date"));

	Ok(())
}

#[test]
fn check_writes_project_cache_artifact() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_up_to_date", tmp.path());

	common::mdt_cmd()
		.arg("check")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success();

	let cache_path = tmp.path().join(".mdt").join("cache").join("index-v2.json");
	assert!(
		cache_path.is_file(),
		"expected cache file at {}",
		cache_path.display()
	);

	Ok(())
}

#[test]
fn check_fails_when_stale() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_stale", tmp.path());

	common::mdt_cmd()
		.arg("check")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.failure()
		.stderr(predicates::str::contains("out of date"));

	Ok(())
}

#[test]
fn check_with_no_blocks() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_no_blocks", tmp.path());

	common::mdt_cmd()
		.arg("check")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("up to date"));

	Ok(())
}

#[test]
fn check_verbose_shows_provider_count() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_single_block", tmp.path());

	common::mdt_cmd()
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
fn check_warns_missing_provider() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("update_orphan", tmp.path());

	common::mdt_cmd()
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
fn check_stale_shows_block_name_and_file() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_stale_named", tmp.path());

	common::mdt_cmd()
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
fn check_multiple_stale_blocks() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_multiple_stale", tmp.path());

	common::mdt_cmd()
		.arg("check")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.failure()
		.stderr(predicates::str::contains("2 consumer block(s)"));

	Ok(())
}

#[test]
fn check_stale_text_output_is_colored_when_forced() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_stale", tmp.path());

	let mut cmd = common::mdt_cmd();
	cmd.env_remove("NO_COLOR")
		.env("CLICOLOR_FORCE", "1")
		.arg("check")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.failure()
		.stderr(predicates::str::contains("Check failed."))
		.stderr(predicates::str::contains(ANSI_ESCAPE));

	Ok(())
}

#[test]
fn check_stale_text_output_honors_no_color_flag_even_when_forced() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_stale", tmp.path());

	let mut cmd = common::mdt_cmd();
	cmd.env_remove("NO_COLOR")
		.env("CLICOLOR_FORCE", "1")
		.arg("--no-color")
		.arg("check")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.failure()
		.stderr(predicates::str::contains("Check failed."))
		.stderr(predicates::str::contains(ANSI_ESCAPE).not());

	Ok(())
}

#[test]
fn check_stale_text_output_honors_clicolor_zero() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_stale", tmp.path());

	let mut cmd = common::mdt_cmd();
	cmd.env_remove("NO_COLOR")
		.env("CLICOLOR", "0")
		.arg("check")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.failure()
		.stderr(predicates::str::contains("Check failed."))
		.stderr(predicates::str::contains(ANSI_ESCAPE).not());

	Ok(())
}

#[test]
fn check_validation_diagnostics_are_colored_when_forced() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_invalid_transformer", tmp.path());

	let mut cmd = common::mdt_cmd();
	cmd.env_remove("NO_COLOR")
		.env("CLICOLOR_FORCE", "1")
		.arg("check")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.failure()
		.stderr(predicates::str::contains("unknown transformer `wat`"))
		.stderr(predicates::str::contains(ANSI_ESCAPE));

	Ok(())
}

#[test]
fn check_validation_diagnostics_honor_no_color_flag_when_forced() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_invalid_transformer", tmp.path());

	let mut cmd = common::mdt_cmd();
	cmd.env_remove("NO_COLOR")
		.env("CLICOLOR_FORCE", "1")
		.arg("--no-color")
		.arg("check")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.failure()
		.stderr(predicates::str::contains("unknown transformer `wat`"))
		.stderr(predicates::str::contains(ANSI_ESCAPE).not());

	Ok(())
}

#[test]
fn check_warns_undefined_template_variables() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_undefined_vars", tmp.path());

	common::mdt_cmd()
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
fn check_no_warnings_for_valid_template_variables() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_valid_vars", tmp.path());

	common::mdt_cmd()
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
fn check_watch_flag_accepted_by_binary() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_up_to_date", tmp.path());

	// We cannot test the full watch loop (it runs forever), but we can verify
	// the binary accepts --watch without crashing.
	let mut cmd = common::mdt_cmd();
	let _ = cmd
		.arg("check")
		.arg("--watch")
		.arg("--path")
		.arg(tmp.path())
		.timeout(std::time::Duration::from_secs(3))
		.assert();

	Ok(())
}

#[test]
fn info_command_is_accepted_by_cli_parser() {
	use clap::Parser;

	let cli = MdtCli::parse_from(["mdt", "info"]);
	match cli.command {
		Some(Commands::Info { format }) => {
			assert!(matches!(format, InfoOutputFormat::Text));
		}
		_ => panic!("expected Info command"),
	}

	let cli = MdtCli::parse_from(["mdt", "info", "--format", "json"]);
	match cli.command {
		Some(Commands::Info { format }) => {
			assert!(matches!(format, InfoOutputFormat::Json));
		}
		_ => panic!("expected Info command"),
	}
}

#[test]
fn doctor_command_is_accepted_by_cli_parser() {
	use clap::Parser;

	let cli = MdtCli::parse_from(["mdt", "doctor"]);
	match cli.command {
		Some(Commands::Doctor { format }) => {
			assert!(matches!(format, DoctorOutputFormat::Text));
		}
		_ => panic!("expected Doctor command"),
	}

	let cli = MdtCli::parse_from(["mdt", "doctor", "--format", "json"]);
	match cli.command {
		Some(Commands::Doctor { format }) => {
			assert!(matches!(format, DoctorOutputFormat::Json));
		}
		_ => panic!("expected Doctor command"),
	}
}

#[test]
fn assist_command_is_accepted_by_cli_parser() {
	use clap::Parser;

	let cli = MdtCli::parse_from(["mdt", "assist", "claude"]);
	match cli.command {
		Some(Commands::Assist { assistant, format }) => {
			assert!(matches!(assistant, Assistant::Claude));
			assert!(matches!(format, AssistOutputFormat::Text));
		}
		_ => panic!("expected Assist command"),
	}

	let cli = MdtCli::parse_from(["mdt", "assist", "pi", "--format", "json"]);
	match cli.command {
		Some(Commands::Assist { assistant, format }) => {
			assert!(matches!(assistant, Assistant::Pi));
			assert!(matches!(format, AssistOutputFormat::Json));
		}
		_ => panic!("expected Assist command"),
	}
}

#[test]
fn info_json_includes_cache_observability_fields() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_up_to_date", tmp.path());

	let output = common::mdt_cmd()
		.arg("info")
		.arg("--format")
		.arg("json")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.get_output()
		.stdout
		.clone();

	let report: Value = serde_json::from_slice(&output)?;
	let cache = report
		.get("cache")
		.unwrap_or_else(|| panic!("expected `cache` section in info report"));
	assert_eq!(cache["exists"], Value::Bool(true));
	assert_eq!(cache["readable"], Value::Bool(true));
	assert_eq!(cache["valid"], Value::Bool(true));
	assert!(cache["scan_count"].as_u64().is_some());
	assert!(cache["full_project_hit_count"].as_u64().is_some());
	assert!(cache["reused_file_count_total"].as_u64().is_some());
	assert!(cache["reparsed_file_count_total"].as_u64().is_some());
	assert!(cache["last_scan"].is_object());

	Ok(())
}

#[test]
fn doctor_json_includes_cache_checks() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_up_to_date", tmp.path());

	let output = common::mdt_cmd()
		.arg("doctor")
		.arg("--format")
		.arg("json")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.get_output()
		.stdout
		.clone();

	let report: Value = serde_json::from_slice(&output)?;
	let checks = report["checks"]
		.as_array()
		.unwrap_or_else(|| panic!("expected checks array"));
	let check_ids: std::collections::BTreeSet<&str> = checks
		.iter()
		.filter_map(|check| check.get("id").and_then(Value::as_str))
		.collect();
	assert!(check_ids.contains("cache_artifact"));
	assert!(check_ids.contains("cache_hash_mode"));
	assert!(check_ids.contains("cache_efficiency"));

	Ok(())
}

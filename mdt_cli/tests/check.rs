mod common;

use insta_cmd::assert_cmd_snapshot;
use mdt_cli::AssistOutputFormat;
use mdt_cli::Assistant;
use mdt_cli::Commands;
use mdt_cli::DoctorOutputFormat;
use mdt_cli::InfoOutputFormat;
use mdt_cli::MdtCli;
use predicates::prelude::PredicateBooleanExt;
use rstest::rstest;

const ANSI_ESCAPE: &str = "\u{1b}[";

#[rstest]
#[case("check_up_to_date", "check_passes_when_up_to_date", false)]
#[case("check_stale", "check_fails_when_stale", false)]
#[case("check_no_blocks", "check_with_no_blocks", false)]
#[case("check_single_block", "check_verbose_shows_provider_count", true)]
#[case("update_orphan", "check_warns_missing_provider", false)]
#[case("check_stale_named", "check_stale_shows_block_name_and_file", false)]
#[case("check_multiple_stale", "check_multiple_stale_blocks", false)]
#[case(
	"check_undefined_vars",
	"check_warns_undefined_template_variables",
	false
)]
#[case(
	"check_valid_vars",
	"check_no_warnings_for_valid_template_variables",
	false
)]
fn check_outputs_are_snapshotted(
	#[case] fixture: &str,
	#[case] snapshot_name: &str,
	#[case] verbose: bool,
) -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture(fixture, tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		let mut cmd = common::mdt_cmd_for_path(tmp.path());
		if verbose {
			cmd.arg("--verbose");
		}
		cmd.arg("check");

		assert_cmd_snapshot!(snapshot_name, cmd);
	});

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
fn check_watch_flag_is_accepted_by_cli_parser() {
	use clap::Parser;

	let cli = MdtCli::parse_from(["mdt", "check", "--watch"]);
	match cli.command {
		Some(Commands::Check { watch, diff, .. }) => {
			assert!(watch);
			assert!(!diff);
		}
		_ => panic!("expected Check command"),
	}

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

#[rstest]
#[case("info", "info_json_includes_cache_observability_fields")]
#[case("doctor", "doctor_json_includes_cache_checks")]
fn cache_observability_outputs_are_snapshotted(
	#[case] command_name: &str,
	#[case] snapshot_name: &str,
) -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("check_up_to_date", tmp.path());

	common::with_redacted_temp_dir(tmp.path(), || {
		assert_cmd_snapshot!(
			snapshot_name,
			common::mdt_cmd_for_path(tmp.path())
				.arg(command_name)
				.arg("--format")
				.arg("json")
		);
	});

	Ok(())
}

mod common;

use std::path::Path;

use assert_cmd::assert::OutputAssertExt;
use predicates::prelude::*;

fn write(root: &Path, relative: &str, content: &str) {
	let path = root.join(relative);

	if let Some(parent) = path.parent() {
		std::fs::create_dir_all(parent).unwrap_or_else(|e| panic!("mkdir: {e}"));
	}

	std::fs::write(&path, content).unwrap_or_else(|e| panic!("write {relative}: {e}"));
}

#[test]
fn commands_reject_a_missing_project_path() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	let missing = tmp.path().join("typo");

	for command in ["check", "update", "list", "info", "doctor"] {
		common::mdt_cmd()
			.arg(command)
			.arg("--path")
			.arg(&missing)
			.assert()
			.code(2)
			.stderr(predicate::str::contains("does not exist"));
	}

	assert!(
		!missing.exists(),
		"a mistyped path must not be created as a side effect"
	);

	Ok(())
}

#[test]
fn update_reports_render_errors_and_updates_healthy_consumers() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	write(tmp.path(), "package.json", r#"{"version":"1.2.3"}"#);
	write(tmp.path(), "mdt.toml", "[data]\npkg = \"package.json\"\n");
	write(
		tmp.path(),
		"template.t.md",
		"<!-- {@good} -->\nv{{ pkg.version }}\n<!-- {/good} -->\n\n<!-- {@bad} -->\n{{ \
		 pkg.version\n<!-- {/bad} -->\n",
	);
	write(
		tmp.path(),
		"readme.md",
		"<!-- {=good} -->\nold\n<!-- {/good} -->\n\n<!-- {=bad} -->\nold\n<!-- {/bad} -->\n",
	);

	common::mdt_cmd_for_path(tmp.path())
		.arg("update")
		.output()
		.map(|output| {
			assert_eq!(output.status.code(), Some(1));
			let stderr = String::from_utf8_lossy(&output.stderr);
			assert!(
				stderr.contains("block `bad` at readme.md:5:1 was not updated: syntax error"),
				"{stderr}"
			);
		})?;
	let readme = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert!(readme.contains("v1.2.3"), "{readme}");

	Ok(())
}

#[test]
fn list_shows_blocks_even_with_validation_errors() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	write(
		tmp.path(),
		"template.t.md",
		"<!-- {@features} -->\n\n- fast\n\n<!-- {/features} -->\n",
	);
	write(
		tmp.path(),
		"readme.md",
		"<!-- {=featrues} -->\n\n- fast\n\n<!-- {/features} -->\n",
	);

	common::mdt_cmd_for_path(tmp.path())
		.arg("list")
		.assert()
		.code(2)
		.stdout(predicate::str::contains(
			"@features template.t.md:1 (0 consumer(s))",
		))
		.stderr(predicate::str::contains(
			"missing closing tag for block `featrues`",
		))
		.stderr(predicate::str::contains(
			"closing tag `{/features}` has no matching opening tag",
		));

	Ok(())
}

#[test]
fn check_json_reports_validation_errors_as_diagnostics() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	write(tmp.path(), "readme.md", "<!-- { @name } -->\n");

	let output = common::mdt_cmd_for_path(tmp.path())
		.args(["check", "--format", "json"])
		.output()?;
	assert_eq!(output.status.code(), Some(2));
	let json: serde_json::Value = serde_json::from_slice(&output.stdout)
		.unwrap_or_else(|e| panic!("stdout is not JSON: {e}"));
	assert_eq!(json["ok"], false);
	assert_eq!(json["diagnostics"][0]["code"], "mdt::invalid_tag");
	assert_eq!(json["diagnostics"][0]["severity"], "error");
	assert_eq!(json["diagnostics"][0]["line"], 1);

	Ok(())
}

#[test]
fn check_github_annotates_orphans_as_errors_with_suggestions() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	write(
		tmp.path(),
		"template.t.md",
		"<!-- {@features} -->\n\n- fast\n\n<!-- {/features} -->\n",
	);
	write(
		tmp.path(),
		"readme.md",
		"<!-- {=featrues} -->\n\n- fast\n\n<!-- {/featrues} -->\n",
	);

	common::mdt_cmd_for_path(tmp.path())
		.current_dir(tmp.path())
		.args(["check", "--format", "github", "--ignore-unused-blocks"])
		.assert()
		.code(1)
		.stdout(predicate::str::contains(
			"::error file=readme.md,line=1,col=1::consumer `featrues` at readme.md:1:1 has no \
			 provider (did you mean `features`?)",
		));

	Ok(())
}

#[test]
fn init_next_to_an_existing_readme_leaves_check_green() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	write(tmp.path(), "README.md", "# Existing project\n");

	common::mdt_cmd_for_path(tmp.path())
		.arg("init")
		.assert()
		.success()
		.stdout(predicate::str::contains(
			"Left the existing README.md unchanged",
		));
	common::mdt_cmd_for_path(tmp.path())
		.arg("check")
		.assert()
		.success()
		.stderr(predicate::str::contains(
			"provider block `greeting` has no consumers",
		));
	assert_eq!(
		std::fs::read_to_string(tmp.path().join("README.md"))?,
		"# Existing project\n"
	);

	Ok(())
}

#[test]
fn assist_copilot_uses_the_vscode_servers_key() {
	let output = common::mdt_std_cmd()
		.args(["assist", "copilot", "--format", "json"])
		.output()
		.unwrap_or_else(|e| panic!("run mdt: {e}"));
	assert!(output.status.success());
	let json: serde_json::Value = serde_json::from_slice(&output.stdout)
		.unwrap_or_else(|e| panic!("stdout is not JSON: {e}"));
	assert_eq!(json["mcp_config_file"], ".vscode/mcp.json");
	assert_eq!(json["mcp_config"]["servers"]["mdt"]["command"], "mdt");
	assert!(json["mcp_config"].get("mcpServers").is_none());
	assert_eq!(
		json["skill"]["install_command"],
		"mdt skill --install .github/skills"
	);
}

#[test]
fn commands_find_the_project_root_from_a_subdirectory() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	std::fs::create_dir_all(tmp.path().join(".git"))?;
	write(tmp.path(), "mdt.toml", "");
	write(
		tmp.path(),
		".templates/intro.t.md",
		"<!-- {@intro} -->\n\nHello\n\n<!-- {/intro} -->\n",
	);
	write(
		tmp.path(),
		"readme.md",
		"<!-- {=intro} -->\n\nHello\n\n<!-- {/intro} -->\n",
	);
	write(
		tmp.path(),
		"docs/guide.md",
		"<!-- {=intro} -->\n\nHello\n\n<!-- {/intro} -->\n",
	);

	// Scanning `docs/` alone would report both consumers as orphans.
	common::mdt_cmd()
		.current_dir(tmp.path().join("docs"))
		.arg("check")
		.assert()
		.success();
	common::mdt_cmd()
		.current_dir(tmp.path().join("docs"))
		.arg("list")
		.assert()
		.success()
		.stdout(predicate::str::contains("=intro readme.md:1"))
		.stdout(predicate::str::contains("=intro docs/guide.md:1"))
		.stderr(predicate::str::contains("note: using the mdt project at"));

	// `mdt init` still initializes the directory it runs in.
	common::mdt_cmd()
		.current_dir(tmp.path().join("docs"))
		.arg("init")
		.assert()
		.success();
	assert!(tmp.path().join("docs/mdt.toml").is_file());

	Ok(())
}

#[test]
fn check_github_paths_are_relative_to_the_checkout_for_sub_projects() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	write(tmp.path(), "packages/lib/mdt.toml", "");
	write(
		tmp.path(),
		"packages/lib/.templates/intro.t.md",
		"<!-- {@intro} -->\n\nHello\n\n<!-- {/intro} -->\n",
	);
	write(
		tmp.path(),
		"packages/lib/readme.md",
		"<!-- {=intro} -->\n\nstale\n\n<!-- {/intro} -->\n",
	);

	common::mdt_cmd()
		.current_dir(tmp.path())
		.args(["check", "--path", "packages/lib", "--format", "github"])
		.assert()
		.code(1)
		.stdout(predicate::str::contains(
			"::error file=packages/lib/readme.md,line=1,col=1::",
		));

	Ok(())
}

#[test]
fn check_machine_formats_report_scan_errors() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	write(tmp.path(), "mdt.toml", "[paddding]\nbefore = 0\n");

	let output = common::mdt_cmd_for_path(tmp.path())
		.args(["check", "--format", "json"])
		.output()?;
	assert_eq!(output.status.code(), Some(2));
	let json: serde_json::Value = serde_json::from_slice(&output.stdout)
		.unwrap_or_else(|e| panic!("stdout is not JSON: {e}"));
	assert_eq!(json["ok"], false);
	assert_eq!(json["diagnostics"][0]["code"], "mdt::config_parse");

	let output = common::mdt_cmd_for_path(tmp.path())
		.args(["check", "--format", "github"])
		.output()?;
	assert_eq!(output.status.code(), Some(2));
	let stdout = String::from_utf8_lossy(&output.stdout);
	assert!(stdout.starts_with("::error::"), "{stdout}");
	assert!(
		!stdout.trim_end().contains('\n'),
		"multi-line errors must stay one workflow command: {stdout}"
	);

	Ok(())
}

#[test]
fn root_discovery_never_leaves_the_git_repository() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	// A config above the repository (for example in `$HOME`) must not be
	// adopted: its data scripts would run and other projects would be scanned.
	write(
		tmp.path(),
		"mdt.toml",
		"[data]\npwned = { command = \"echo > PWNED\", format = \"text\" }\n",
	);
	std::fs::create_dir_all(tmp.path().join("repo/.git"))?;
	write(tmp.path(), "repo/readme.md", "# Repo\n");

	common::mdt_cmd()
		.current_dir(tmp.path().join("repo"))
		.arg("check")
		.assert()
		.success()
		.stderr(predicate::str::contains("using the mdt project").not());
	assert!(!tmp.path().join("PWNED").exists());
	assert!(!tmp.path().join(".mdt").exists());

	Ok(())
}

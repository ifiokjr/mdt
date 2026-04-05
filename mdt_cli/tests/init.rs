mod common;

use mdt_core::AnyEmptyResult;

/// Tests that run `mdt init` on an empty directory don't need a fixture — an
/// empty tempdir *is* the fixture scenario (nothing pre-exists).

#[test]
fn can_init() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	let assert = common::mdt_cmd()
		.arg("init")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success();
	assert
		.stdout(predicates::str::contains("Created template file"))
		.stdout(predicates::str::contains("Created mdt.toml"));

	let template_path = tmp.path().join(".templates/template.t.md");
	assert!(template_path.exists());

	let content = std::fs::read_to_string(&template_path)?;
	assert!(content.contains("{@greeting}"));
	assert!(content.contains("{/greeting}"));

	let config_path = tmp.path().join("mdt.toml");
	assert!(config_path.exists());

	let config_content = std::fs::read_to_string(&config_path)?;
	assert!(config_content.contains("# Welcome to mdt."));
	assert!(config_content.contains("# max_file_size = 10485760"));
	assert!(config_content.contains("# [data]"));
	assert!(config_content.contains("# [padding]"));
	assert!(config_content.contains("# [exclude]"));
	assert!(config_content.contains("# [include]"));
	assert!(config_content.contains("# [templates]"));
	assert!(config_content.contains("# [[formatters]]"));
	assert!(config_content.contains("{{ filePath }}"));

	Ok(())
}

#[test]
fn init_does_not_overwrite() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("init_overwrite_both", tmp.path());

	let assert = common::mdt_cmd()
		.arg("init")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success();
	assert.stdout(predicates::str::contains("already exists"));

	let content = std::fs::read_to_string(tmp.path().join("template.t.md"))?;
	assert_eq!(content, "existing content");

	let config_content = std::fs::read_to_string(tmp.path().join("mdt.toml"))?;
	assert_eq!(config_content, "existing config");

	Ok(())
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

	// The generated template should be parseable by mdt
	let template_content = std::fs::read_to_string(tmp.path().join(".templates/template.t.md"))?;
	let blocks = mdt_core::parse(&template_content)?;
	assert!(!blocks.is_empty(), "init should create at least one block");
	assert_eq!(blocks[0].r#type, mdt_core::BlockType::Provider);

	Ok(())
}

#[test]
fn init_shows_next_steps() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	common::mdt_cmd()
		.arg("init")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("Next steps"))
		.stdout(predicates::str::contains("mdt update"));

	Ok(())
}

#[test]
fn init_creates_both_template_and_config() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	common::mdt_cmd()
		.arg("init")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("Created template file"))
		.stdout(predicates::str::contains("Created mdt.toml"));

	let template_path = tmp.path().join(".templates/template.t.md");
	let config_path = tmp.path().join("mdt.toml");

	assert!(
		template_path.exists(),
		".templates/template.t.md should be created"
	);
	assert!(config_path.exists(), "mdt.toml should be created");

	// Verify template content
	let template_content = std::fs::read_to_string(&template_path)?;
	assert!(template_content.contains("{@greeting}"));

	// Verify config is valid TOML (all lines are comments or blank, so it parses as empty)
	let config_content = std::fs::read_to_string(&config_path)?;
	assert!(config_content.contains("# Welcome to mdt."));
	assert!(config_content.contains("# [data]"));
	assert!(config_content.contains("# [padding]"));
	assert!(config_content.contains("# [exclude]"));
	assert!(config_content.contains("# [include]"));
	assert!(config_content.contains("# [templates]"));
	assert!(config_content.contains("# [[formatters]]"));
	assert!(config_content.contains("# package = \"package.json\""));
	assert!(config_content.contains("# release = { path = \"release-info\", format = \"json\" }"));

	Ok(())
}

#[test]
fn init_creates_config_when_template_exists() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	common::copy_fixture("init_existing_template_only", tmp.path());

	common::mdt_cmd()
		.arg("init")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("already exists"))
		.stdout(predicates::str::contains("Created mdt.toml"));

	// Template should not be modified
	let content = std::fs::read_to_string(tmp.path().join("template.t.md"))?;
	assert_eq!(content, "existing template");

	// Config should be created
	let config_path = tmp.path().join("mdt.toml");
	assert!(config_path.exists());

	Ok(())
}

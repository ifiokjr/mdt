use assert_cmd::Command;
use mdt::AnyEmptyResult;

#[test]
fn can_init() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	let mut cmd = Command::cargo_bin("mdt")?;
	let assert = cmd
		.arg("init")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success();
	assert.stdout(predicates::str::contains("Created template file"));

	let template_path = tmp.path().join("template.t.md");
	assert!(template_path.exists());

	let content = std::fs::read_to_string(&template_path)?;
	assert!(content.contains("{@greeting}"));
	assert!(content.contains("{/greeting}"));

	Ok(())
}

#[test]
fn init_does_not_overwrite() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;
	let template_path = tmp.path().join("template.t.md");
	std::fs::write(&template_path, "existing content")?;

	let mut cmd = Command::cargo_bin("mdt")?;
	let assert = cmd
		.arg("init")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success();
	assert.stdout(predicates::str::contains("already exists"));

	let content = std::fs::read_to_string(&template_path)?;
	assert_eq!(content, "existing content");

	Ok(())
}

#[test]
fn init_creates_valid_template() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	Command::cargo_bin("mdt")?
		.arg("init")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success();

	// The generated template should be parseable by mdt
	let template_content = std::fs::read_to_string(tmp.path().join("template.t.md"))?;
	let blocks = mdt::parse(&template_content)?;
	assert!(!blocks.is_empty(), "init should create at least one block");
	assert_eq!(blocks[0].r#type, mdt::BlockType::Provider);

	Ok(())
}

#[test]
fn init_shows_next_steps() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	Command::cargo_bin("mdt")?
		.arg("init")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("Next steps"))
		.stdout(predicates::str::contains("mdt update"));

	Ok(())
}

mod common;

use mdt_core::AnyEmptyResult;

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

	let mut cmd = common::mdt_cmd();
	cmd.env("NO_COLOR", "1")
		.arg("update")
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

	let mut cmd = common::mdt_cmd();
	cmd.env("NO_COLOR", "1")
		.arg("update")
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

	let mut cmd = common::mdt_cmd();
	cmd.env("NO_COLOR", "1")
		.arg("update")
		.arg("--dry-run")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("would update"));

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

	let mut cmd = common::mdt_cmd();
	cmd.env("NO_COLOR", "1")
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success();

	let content = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert!(content.contains("Some documentation content."));

	Ok(())
}

#[test]
fn update_verbose_shows_files() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\nnew\n\n<!-- {/block} -->\n",
	)?;
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=block} -->\n\nold\n\n<!-- {/block} -->\n",
	)?;

	let mut cmd = common::mdt_cmd();
	cmd.env("NO_COLOR", "1")
		.arg("update")
		.arg("--verbose")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("readme.md"));

	Ok(())
}

#[test]
fn update_warns_missing_provider() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=orphan} -->\n\nstuff\n\n<!-- {/orphan} -->\n",
	)?;

	let mut cmd = common::mdt_cmd();
	cmd.env("NO_COLOR", "1")
		.arg("update")
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
fn update_multiple_blocks_in_one_file() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@a} -->\n\nalpha\n\n<!-- {/a} -->\n\n<!-- {@b} -->\n\nbeta\n\n<!-- {/b} -->\n",
	)?;
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=a} -->\n\nold\n\n<!-- {/a} -->\n\n<!-- {=b} -->\n\nold\n\n<!-- {/b} -->\n",
	)?;

	let mut cmd = common::mdt_cmd();
	cmd.env("NO_COLOR", "1")
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("Updated 2 block(s)"));

	let content = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert!(content.contains("alpha"));
	assert!(content.contains("beta"));
	assert!(!content.contains("old"));

	Ok(())
}

#[test]
fn update_dry_run_shows_file_list() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\nnew\n\n<!-- {/block} -->\n",
	)?;
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=block} -->\n\nold\n\n<!-- {/block} -->\n",
	)?;

	let mut cmd = common::mdt_cmd();
	cmd.env("NO_COLOR", "1")
		.arg("update")
		.arg("--dry-run")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("readme.md"));

	Ok(())
}

#[test]
fn update_with_config_and_data() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npkg = \"package.json\"\n",
	)?;
	std::fs::write(
		tmp.path().join("package.json"),
		r#"{"name": "my-app", "version": "3.0.0"}"#,
	)?;
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@install} -->\n\nnpm install {{ pkg.name }}@{{ pkg.version }}\n\n<!-- {/install} \
		 -->\n",
	)?;
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=install} -->\n\nold\n\n<!-- {/install} -->\n",
	)?;

	let mut cmd = common::mdt_cmd();
	cmd.env("NO_COLOR", "1")
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success();

	let content = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert!(content.contains("npm install my-app@3.0.0"));

	Ok(())
}

#[test]
fn update_inline_table_cell_with_data() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npkg = \"package.json\"\n",
	)?;
	std::fs::write(
		tmp.path().join("package.json"),
		r#"{"name": "mdt", "version": "3.1.4"}"#,
	)?;
	std::fs::write(
		tmp.path().join("readme.md"),
		"| Package | Version |\n| ------- | ------- |\n| mdt     | <!-- {~version:\"{{ \
		 pkg.version }}\"} -->0.0.0<!-- {/version} --> |\n",
	)?;

	let mut cmd = common::mdt_cmd();
	cmd.env("NO_COLOR", "1")
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("Updated"));

	let content = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert!(content.contains(
		"| mdt     | <!-- {~version:\"{{ pkg.version }}\"} -->3.1.4<!-- {/version} --> |"
	));

	Ok(())
}

#[test]
fn update_preserves_multiline_link_definitions() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	let template = r#"<!-- {@badge:"crateName"} -->

[crate-image]: https://img.shields.io/crates/v/{{ crateName }}.svg
[crate-link]: https://crates.io/crates/{{ crateName }}
[docs-image]: https://docs.rs/{{ crateName }}/badge.svg
[docs-link]: https://docs.rs/{{ crateName }}/
[ci-image]: https://github.com/example/repo/workflows/ci/badge.svg
[ci-link]: https://github.com/example/repo/actions

<!-- {/badge} -->
"#;
	std::fs::write(tmp.path().join("template.t.md"), template)?;
	std::fs::write(
		tmp.path().join("readme.md"),
		r#"# Readme

<!-- {=badge:"my_crate"} -->

old

<!-- {/badge} -->
"#,
	)?;

	let mut cmd = common::mdt_cmd();
	cmd.env("NO_COLOR", "1")
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success();

	let content = std::fs::read_to_string(tmp.path().join("readme.md"))?;

	// Each link definition must be on its own line — newlines must be preserved.
	assert!(
		content.contains("\n[crate-image]:"),
		"[crate-image] should be on its own line"
	);
	assert!(
		content.contains("\n[crate-link]:"),
		"[crate-link] should be on its own line"
	);
	assert!(
		content.contains("\n[docs-image]:"),
		"[docs-image] should be on its own line"
	);
	assert!(
		content.contains("\n[docs-link]:"),
		"[docs-link] should be on its own line"
	);
	assert!(
		content.contains("\n[ci-image]:"),
		"[ci-image] should be on its own line"
	);
	assert!(
		content.contains("\n[ci-link]:"),
		"[ci-link] should be on its own line"
	);

	// Verify template variables were rendered
	assert!(content.contains("my_crate"));
	assert!(!content.contains("{{ crateName }}"));

	Ok(())
}

#[test]
fn update_multiline_idempotent_after_write() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	let template = r#"<!-- {@links} -->

[repo]: https://github.com/example/repo
[docs]: https://docs.example.com
[ci]: https://ci.example.com/badge.svg

<!-- {/links} -->
"#;
	std::fs::write(tmp.path().join("template.t.md"), template)?;
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=links} -->\n\nold\n\n<!-- {/links} -->\n",
	)?;

	// First update
	let mut cmd = common::mdt_cmd();
	cmd.env("NO_COLOR", "1")
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("Updated"));

	let after_first = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert!(after_first.contains("\n[repo]:"));
	assert!(after_first.contains("\n[docs]:"));
	assert!(after_first.contains("\n[ci]:"));

	// Second update — should be idempotent
	let mut cmd2 = common::mdt_cmd();
	cmd2.env("NO_COLOR", "1")
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success()
		.stdout(predicates::str::contains("already up to date"));

	let after_second = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert_eq!(
		after_first, after_second,
		"Second update should not change the file"
	);

	Ok(())
}

#[test]
fn update_preserves_surrounding_content() -> AnyEmptyResult {
	let tmp = tempfile::tempdir()?;

	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@block} -->\n\nnew content\n\n<!-- {/block} -->\n",
	)?;
	std::fs::write(
		tmp.path().join("readme.md"),
		"# Header\n\nParagraph before.\n\n<!-- {=block} -->\n\nold\n\n<!-- {/block} \
		 -->\n\nParagraph after.\n",
	)?;

	let mut cmd = common::mdt_cmd();
	cmd.env("NO_COLOR", "1")
		.arg("update")
		.arg("--path")
		.arg(tmp.path())
		.assert()
		.success();

	let content = std::fs::read_to_string(tmp.path().join("readme.md"))?;
	assert!(content.contains("# Header"));
	assert!(content.contains("Paragraph before."));
	assert!(content.contains("new content"));
	assert!(content.contains("Paragraph after."));
	assert!(!content.contains("old"));

	Ok(())
}

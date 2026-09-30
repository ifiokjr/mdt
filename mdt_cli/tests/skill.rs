mod common;

use std::path::Path;
use std::path::PathBuf;

use predicates::prelude::*;

const SKILL_FILES: [&str; 2] = ["SKILL.md", "REFERENCE.md"];

fn embedded_skill_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("skill")
}

fn read(path: &Path) -> String {
	std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn skill_prints_skill_md() {
	let expected = read(&embedded_skill_dir().join("SKILL.md"));
	assert!(
		expected.starts_with("---\nname: mdt\n"),
		"SKILL.md must start with frontmatter naming the `mdt` skill"
	);

	common::mdt_cmd()
		.arg("skill")
		.assert()
		.success()
		.stdout(expected);
}

#[test]
fn skill_reference_prints_reference_md() {
	common::mdt_cmd()
		.args(["skill", "--reference"])
		.assert()
		.success()
		.stdout(read(&embedded_skill_dir().join("REFERENCE.md")));
}

#[test]
fn skill_install_writes_and_replaces_skill_files() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	let skills_dir = tmp.path().join(".claude/skills");
	std::fs::create_dir_all(skills_dir.join("mdt"))?;
	std::fs::write(skills_dir.join("mdt/SKILL.md"), "outdated skill")?;

	common::mdt_cmd()
		.arg("skill")
		.arg("--install")
		.arg(&skills_dir)
		.assert()
		.success()
		.stdout(predicate::str::starts_with("Installed the mdt skill:\n"))
		.stdout(predicate::str::contains(".claude/skills/mdt/SKILL.md"))
		.stdout(predicate::str::contains(".claude/skills/mdt/REFERENCE.md"));

	for name in SKILL_FILES {
		assert_eq!(
			read(&skills_dir.join("mdt").join(name)),
			read(&embedded_skill_dir().join(name)),
			"installed {name} should match the embedded skill"
		);
	}

	Ok(())
}

#[test]
fn skill_install_fails_when_target_is_a_file() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	let not_a_dir = tmp.path().join("skills");
	std::fs::write(&not_a_dir, "")?;

	common::mdt_cmd()
		.arg("skill")
		.arg("--install")
		.arg(&not_a_dir)
		.assert()
		.code(2)
		.stderr(predicate::str::contains(
			"failed to install the mdt skill into",
		));

	Ok(())
}

#[test]
fn skill_reference_conflicts_with_install() {
	common::mdt_cmd()
		.args(["skill", "--reference", "--install", "skills"])
		.assert()
		.code(2)
		.stderr(predicate::str::contains("cannot be used with"));
}

/// `mdt_cli/skill/` is a copy of the canonical skill package, because cargo
/// only publishes files inside the crate. Fail loudly when the two drift.
/// The package directory is absent when testing from a published crate.
#[test]
fn embedded_skill_matches_skill_package() {
	let package_dir =
		Path::new(env!("CARGO_MANIFEST_DIR")).join("../packages/m-d-t__skills/skills/mdt");
	if !package_dir.is_dir() {
		return;
	}

	let mut package_files: Vec<String> = std::fs::read_dir(&package_dir)
		.unwrap_or_else(|e| panic!("read_dir {}: {e}", package_dir.display()))
		.map(|entry| {
			entry
				.unwrap_or_else(|e| panic!("entry: {e}"))
				.file_name()
				.to_string_lossy()
				.into_owned()
		})
		.collect();
	package_files.sort();
	let mut embedded_files = SKILL_FILES.map(String::from).to_vec();
	embedded_files.sort();
	assert_eq!(
		package_files, embedded_files,
		"the skill package gained or lost files; update `SKILL_FILES` in mdt_cli/src/skill.rs"
	);

	for name in SKILL_FILES {
		assert!(
			read(&package_dir.join(name)) == read(&embedded_skill_dir().join(name)),
			"mdt_cli/skill/{name} is out of sync with packages/m-d-t__skills/skills/mdt/{name}; \
			 run `fix:skill` to copy the canonical skill into the CLI"
		);
	}
}

mod common;

use std::path::Path;

fn hook_path(git_dir: &Path) -> std::path::PathBuf {
	git_dir.join(".git").join("hooks").join("pre-commit")
}

#[test]
fn hook_install_writes_executable_pre_commit_hook() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	std::fs::create_dir_all(tmp.path().join(".git"))?;

	let output = common::mdt_cmd()
		.arg("--path")
		.arg(tmp.path())
		.arg("hook")
		.arg("install")
		.output()?;

	assert!(
		output.status.success(),
		"hook install failed: {}",
		String::from_utf8_lossy(&output.stderr)
	);

	let hook = hook_path(tmp.path());
	let content = std::fs::read_to_string(&hook)?;
	assert!(content.starts_with("#!/bin/sh"));
	assert!(content.contains("mdt check"));

	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt as _;

		let mode = std::fs::metadata(&hook)?.permissions().mode();
		assert_ne!(mode & 0o111, 0, "pre-commit hook must be executable");
	}

	let stdout = String::from_utf8_lossy(&output.stdout);
	assert!(stdout.contains("Installed"), "unexpected output: {stdout}");

	Ok(())
}

#[test]
fn hook_install_finds_repository_above_the_project_root() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	std::fs::create_dir_all(tmp.path().join(".git"))?;
	let nested = tmp.path().join("docs");
	std::fs::create_dir_all(&nested)?;

	let output = common::mdt_cmd()
		.arg("--path")
		.arg(&nested)
		.arg("hook")
		.arg("install")
		.output()?;

	assert!(output.status.success());
	assert!(hook_path(tmp.path()).is_file());

	Ok(())
}

#[test]
fn hook_install_is_idempotent() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	std::fs::create_dir_all(tmp.path().join(".git"))?;

	let first = common::mdt_cmd()
		.arg("--path")
		.arg(tmp.path())
		.arg("hook")
		.arg("install")
		.output()?;
	assert!(first.status.success());

	let second = common::mdt_cmd()
		.arg("--path")
		.arg(tmp.path())
		.arg("hook")
		.arg("install")
		.output()?;

	assert!(second.status.success());
	let stdout = String::from_utf8_lossy(&second.stdout);
	assert!(
		stdout.contains("already installed"),
		"unexpected output: {stdout}"
	);

	Ok(())
}

#[test]
fn hook_install_refuses_to_replace_a_different_hook() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	let hooks = tmp.path().join(".git").join("hooks");
	std::fs::create_dir_all(&hooks)?;
	std::fs::write(hooks.join("pre-commit"), "#!/bin/sh\necho custom hook\n")?;

	let output = common::mdt_cmd()
		.arg("--path")
		.arg(tmp.path())
		.arg("hook")
		.arg("install")
		.output()?;

	assert_eq!(output.status.code(), Some(2));
	let stderr = String::from_utf8_lossy(&output.stderr);
	assert!(stderr.contains("--force"), "unexpected error: {stderr}");
	// The custom hook is left untouched.
	assert_eq!(
		std::fs::read_to_string(hooks.join("pre-commit"))?,
		"#!/bin/sh\necho custom hook\n"
	);

	Ok(())
}

#[test]
fn hook_install_force_replaces_a_different_hook() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	let hooks = tmp.path().join(".git").join("hooks");
	std::fs::create_dir_all(&hooks)?;
	std::fs::write(hooks.join("pre-commit"), "#!/bin/sh\necho custom hook\n")?;

	let output = common::mdt_cmd()
		.arg("--path")
		.arg(tmp.path())
		.arg("hook")
		.arg("install")
		.arg("--force")
		.output()?;

	assert!(output.status.success());
	let stdout = String::from_utf8_lossy(&output.stdout);
	assert!(stdout.contains("Replaced"), "unexpected output: {stdout}");
	assert!(std::fs::read_to_string(hooks.join("pre-commit"))?.contains("mdt check"));

	Ok(())
}

#[test]
fn hook_install_outside_a_git_repository_fails() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;

	let output = common::mdt_cmd()
		.arg("--path")
		.arg(tmp.path())
		.arg("hook")
		.arg("install")
		.output()?;

	assert_eq!(output.status.code(), Some(2));
	let stderr = String::from_utf8_lossy(&output.stderr);
	assert!(
		stderr.contains("no git repository"),
		"unexpected error: {stderr}"
	);

	Ok(())
}

#[test]
fn hook_install_follows_a_linked_worktree_gitdir_pointer() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	// A linked worktree keeps its hooks beside the real git dir and stores a
	// `gitdir:` pointer in `.git`.
	let real_git = tmp.path().join("real-git");
	let hooks = real_git.join("hooks");
	std::fs::create_dir_all(&hooks)?;
	let worktree = tmp.path().join("docs");
	std::fs::create_dir_all(&worktree)?;
	std::fs::write(
		worktree.join(".git"),
		format!("gitdir: {}\n", real_git.display()),
	)?;

	let output = common::mdt_cmd()
		.arg("--path")
		.arg(&worktree)
		.arg("hook")
		.arg("install")
		.output()?;

	assert!(
		output.status.success(),
		"hook install failed: {}",
		String::from_utf8_lossy(&output.stderr)
	);
	assert!(hooks.join("pre-commit").is_file());

	Ok(())
}

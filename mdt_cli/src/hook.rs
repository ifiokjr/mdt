use std::io::Read as _;
use std::path::Path;
use std::path::PathBuf;

/// The git hook file `mdt hook install` manages.
const HOOK_FILE_NAME: &str = "pre-commit";

/// The pre-commit script, kept identical to the snippet in the CI integration
/// guide. The body is unindented to match the documented copy exactly.
const HOOK_SCRIPT: &str = concat!(
	"#!/bin/sh\n",
	"# Installed by `mdt hook install`; remove this file to uninstall.\n",
	"if ! mdt check; then\n",
	"echo \"mdt check failed. Run 'mdt update' or fix the errors above before committing.\"\n",
	"exit 1\n",
	"fi\n",
);

/// Outcome of installing the pre-commit hook.
pub enum InstallOutcome {
	/// The hook was written for the first time.
	Installed(PathBuf),
	/// An identical mdt hook was already in place; nothing changed.
	AlreadyInstalled(PathBuf),
	/// A different existing hook was replaced after `--force`.
	Replaced(PathBuf),
}

/// Locate the hooks directory of the repository enclosing `root`.
///
/// Walks upward looking for `.git` — a directory for standard checkouts, or
/// the `gitdir: <path>` pointer file linked worktrees use. Returns `None`
/// when `root` is not inside a git repository.
fn hooks_directory(root: &Path) -> Option<PathBuf> {
	for dir in root.ancestors() {
		let dot_git = dir.join(".git");

		if let Ok(metadata) = std::fs::symlink_metadata(&dot_git) {
			if metadata.is_dir() {
				return Some(dot_git.join("hooks"));
			}

			let mut content = String::new();
			if std::fs::File::open(&dot_git)
				.and_then(|mut file| file.read_to_string(&mut content))
				.is_ok()
			{
				let gitdir = content
					.lines()
					.find_map(|line| line.strip_prefix("gitdir:"))
					.map(str::trim)?;
				return Some(PathBuf::from(gitdir).join("hooks"));
			}
		}
	}

	None
}

/// Install the mdt pre-commit hook for the repository enclosing `root`.
///
/// Fails when no enclosing repository exists, or when a differing hook is
/// already installed and `force` is not set.
pub fn install(root: &Path, force: bool) -> Result<InstallOutcome, String> {
	let hooks = hooks_directory(root).ok_or_else(|| {
		format!(
			"no git repository found at or above {}; initialize one first",
			root.display()
		)
	})?;
	std::fs::create_dir_all(&hooks)
		.map_err(|error| format!("failed to create {}: {error}", hooks.display()))?;

	let hook_path = hooks.join(HOOK_FILE_NAME);
	let replacing = match std::fs::read_to_string(&hook_path) {
		Ok(existing) => {
			if existing == HOOK_SCRIPT {
				return Ok(InstallOutcome::AlreadyInstalled(hook_path));
			}
			if !force {
				return Err(format!(
					"{} already exists and differs from the mdt hook; pass --force to replace it",
					hook_path.display()
				));
			}
			true
		}
		Err(_) => false,
	};

	std::fs::write(&hook_path, HOOK_SCRIPT)
		.map_err(|error| format!("failed to write {}: {error}", hook_path.display()))?;

	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt as _;

		let executable = std::fs::Permissions::from_mode(0o755);
		std::fs::set_permissions(&hook_path, executable).map_err(|error| {
			format!("failed to make {} executable: {error}", hook_path.display())
		})?;
	}

	Ok(if replacing {
		InstallOutcome::Replaced(hook_path)
	} else {
		InstallOutcome::Installed(hook_path)
	})
}

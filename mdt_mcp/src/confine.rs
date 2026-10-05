//! Confinement of caller-supplied project paths to the server root.
//!
//! mdt executes config-declared shell commands (`[data]` scripts,
//! formatters) and writes files relative to the project root, so a tool
//! `path` must never reach a directory outside the one the server serves.

use std::io;
use std::path::Path;
use std::path::PathBuf;

use mdt_core::project::normalize_lexically;

use crate::response::ToolError;

const PATH_OUTSIDE_ROOT: &str = "mdt::path_outside_root";
const PATH_NOT_FOUND: &str = "mdt::path_not_found";
const PATH_NOT_DIRECTORY: &str = "mdt::path_not_directory";
const PATH_UNRESOLVABLE: &str = "mdt::path_unresolvable";

/// What a tool needs from its project root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RootRequirement {
	/// The root must be an existing directory.
	ExistingDirectory,
	/// The root may be created (`mdt_init`); if it exists it must be a
	/// directory.
	Creatable,
}

/// Resolve the caller-supplied `requested` path against the canonical `base`,
/// confine the result to `base`, and check it meets `requirement`.
///
/// `None` or a blank path selects `base` itself.
pub(crate) fn confine_root(
	base: &Path,
	requested: Option<&str>,
	requirement: RootRequirement,
) -> Result<PathBuf, ToolError> {
	let requested = requested.filter(|path| !path.trim().is_empty());

	let root = match requested {
		Some(requested) => resolve_within(base, requested)?,
		None => base.to_path_buf(),
	};

	let shown = requested.unwrap_or(".");

	if root.is_dir() {
		return Ok(root);
	}

	if root.exists() {
		return Err(ToolError::new(
			PATH_NOT_DIRECTORY,
			format!("path `{shown}` is not a directory"),
		));
	}

	match requirement {
		RootRequirement::Creatable => Ok(root),
		RootRequirement::ExistingDirectory => {
			Err(ToolError::new(
				PATH_NOT_FOUND,
				format!("path `{shown}` does not exist"),
			))
		}
	}
}

/// Resolve `requested` against `base` and reject it unless it stays inside
/// `base`.
///
/// Symlinks are resolved through the deepest existing ancestor, so a path
/// that does not exist yet cannot escape `base` through a symlinked parent.
fn resolve_within(base: &Path, requested: &str) -> Result<PathBuf, ToolError> {
	// `join` keeps an absolute `requested` as is.
	let root = canonicalize_existing_prefix(&normalize_lexically(&base.join(requested))).map_err(
		|error| {
			ToolError::new(
				PATH_UNRESOLVABLE,
				format!("path `{requested}` cannot be resolved: {error}"),
			)
		},
	)?;

	if root.starts_with(base) {
		return Ok(root);
	}

	Err(ToolError::new(
		PATH_OUTSIDE_ROOT,
		format!(
			"path `{requested}` resolves outside the mdt MCP server root `{}`. Restart the server \
			 in the project you want to manage (`mdt mcp --path <dir>`).",
			base.display()
		),
	))
}

/// Canonicalize the deepest existing ancestor of the absolute, lexically
/// normalized `path`, then re-append the components that do not exist yet.
///
/// An entry that exists but cannot be canonicalized — a dangling symlink, or
/// one behind a permission error — is an error rather than a missing
/// component: creating directories through a dangling symlink would follow it
/// wherever it points.
fn canonicalize_existing_prefix(path: &Path) -> io::Result<PathBuf> {
	let mut missing = Vec::new();
	let mut current = path;

	loop {
		match current.canonicalize() {
			Ok(canonical) => {
				return Ok(missing
					.iter()
					.rev()
					.fold(canonical, |resolved, component| resolved.join(component)));
			}

			Err(error) if current.symlink_metadata().is_ok() => return Err(error),
			Err(error) => {
				let (Some(parent), Some(name)) = (current.parent(), current.file_name()) else {
					return Err(error);
				};
				missing.push(name);
				current = parent;
			}
		}
	}
}

//! Project initialization shared by `mdt init` and the MCP `mdt_init` tool.
//!
//! [`init_project`] adds only what a project is missing: a sample provider,
//! an annotated `mdt.toml`, a synced sample consumer, and a `.gitignore`
//! entry for the local cache. It never overwrites existing files and always
//! leaves the project passing `mdt check`.

use std::path::Path;
use std::path::PathBuf;

use crate::MdtConfig;
use crate::MdtResult;
use crate::compute_updates;
use crate::default_mdt_toml::DEFAULT_MDT_TOML;
use crate::project::scan_project_with_config;
use crate::write_updates;

/// Name of the sample provider block written by [`init_project`].
pub const SAMPLE_BLOCK_NAME: &str = "greeting";

/// Where the sample provider is written, relative to the project root.
pub const SAMPLE_TEMPLATE_PATH: &str = ".templates/template.t.md";

/// Template locations from older mdt versions that also count as an
/// existing sample.
const LEGACY_TEMPLATE_PATHS: [&str; 2] = ["template.t.md", "templates/template.t.md"];

/// An mdt tag for the sample block. Built at runtime so this source file
/// never contains a literal tag, which mdt would scan as a live block.
fn sample_tag(sigil: char) -> String {
	format!("<!-- {{{sigil}{SAMPLE_BLOCK_NAME}}} -->")
}

fn sample_provider() -> String {
	format!(
		"{}\n\nHello from mdt! This is a provider block.\n\n{}\n",
		sample_tag('@'),
		sample_tag('/')
	)
}

/// The sample readme starts with an empty consumer; [`init_project`] then
/// syncs it through the engine so it honours an existing `[padding]`.
fn sample_readme() -> String {
	format!(
		"# My Project\n\nWelcome to my project.\n\n{}\n{}\n",
		sample_tag('='),
		sample_tag('/')
	)
}

const CACHE_IGNORE_ENTRY: &str = ".mdt/";

/// What [`init_project`] did about the sample provider.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SampleOutcome {
	/// Wrote the sample provider and a `readme.md` whose consumer is already
	/// in sync.
	CreatedWithReadme { template: PathBuf, readme: PathBuf },
	/// Wrote the sample provider. The project already has a README, which is
	/// left untouched, so the provider has no consumer yet.
	CreatedWithoutConsumer { template: PathBuf, readme: PathBuf },
	/// A sample template already exists; nothing was added.
	TemplateExists { template: PathBuf },
	/// The project already defines providers; adding a sample would only
	/// clutter it.
	ProvidersExist { count: usize },
}

/// What [`init_project`] did about the config file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConfigOutcome {
	/// Wrote the annotated starter `mdt.toml`.
	Created(PathBuf),
	/// A config file already exists (`mdt.toml`, `.mdt.toml`, or
	/// `.config/mdt.toml`) and was left untouched.
	Exists(PathBuf),
}

/// What [`init_project`] did about ignoring the `.mdt/` cache directory.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GitignoreOutcome {
	/// Appended `.mdt/` to an existing `.gitignore`.
	Updated(PathBuf),
	/// Created `.gitignore` containing `.mdt/` in a git repository.
	Created(PathBuf),
	/// `.gitignore` already ignores `.mdt/`.
	AlreadyIgnored(PathBuf),
	/// Not a git repository and no `.gitignore` exists.
	NotApplicable,
}

/// Everything [`init_project`] did, for callers to report.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct InitReport {
	/// The project root, created when it did not exist.
	pub root: PathBuf,
	/// Whether the root directory itself was created.
	pub created_root: bool,
	pub sample: SampleOutcome,
	pub config: ConfigOutcome,
	pub gitignore: GitignoreOutcome,
}

impl InitReport {
	/// Every file this run created or modified, in the order they were
	/// written.
	pub fn written_files(&self) -> Vec<&Path> {
		let mut files: Vec<&Path> = Vec::new();
		if let ConfigOutcome::Created(config) = &self.config {
			files.push(config);
		}
		match &self.sample {
			SampleOutcome::CreatedWithReadme { template, readme } => {
				files.push(template);
				files.push(readme);
			}
			SampleOutcome::CreatedWithoutConsumer { template, .. } => files.push(template),
			SampleOutcome::TemplateExists { .. } | SampleOutcome::ProvidersExist { .. } => {}
		}
		if let GitignoreOutcome::Updated(path) | GitignoreOutcome::Created(path) = &self.gitignore {
			files.push(path);
		}
		files
	}
}

/// Initialize mdt in `root`, adding only what the project is missing.
///
/// - Creates `root` when it does not exist.
/// - Writes the annotated `mdt.toml` unless a config file already exists.
/// - Writes a sample `greeting` provider to [`SAMPLE_TEMPLATE_PATH`] unless
///   a sample template or any provider already exists. When the project has
///   no README, also writes `readme.md` with the sample consumer already
///   synced; an existing README is never modified.
/// - Adds `.mdt/` to `.gitignore` in git repositories so the local cache is
///   not committed.
///
/// # Errors
///
/// Returns an error when a file cannot be written or the existing project
/// cannot be scanned (for example, an invalid existing `mdt.toml`).
pub fn init_project(root: &Path) -> MdtResult<InitReport> {
	let created_root = !root.exists();
	std::fs::create_dir_all(root)?;

	let config = if let Some(existing) = MdtConfig::resolve_path(root) {
		ConfigOutcome::Exists(existing)
	} else {
		let path = root.join("mdt.toml");
		std::fs::write(&path, DEFAULT_MDT_TOML)?;
		ConfigOutcome::Created(path)
	};

	let sample = init_sample(root)?;
	let gitignore = ignore_cache_directory(root)?;

	Ok(InitReport {
		root: root.to_path_buf(),
		created_root,
		sample,
		config,
		gitignore,
	})
}

fn init_sample(root: &Path) -> MdtResult<SampleOutcome> {
	let existing_template = std::iter::once(SAMPLE_TEMPLATE_PATH)
		.chain(LEGACY_TEMPLATE_PATHS)
		.map(|relative| root.join(relative))
		.find(|path| path.exists());
	if let Some(template) = existing_template {
		return Ok(SampleOutcome::TemplateExists { template });
	}

	let provider_count = scan_project_with_config(root)?.project.providers.len();
	if provider_count > 0 {
		return Ok(SampleOutcome::ProvidersExist {
			count: provider_count,
		});
	}

	let template = root.join(SAMPLE_TEMPLATE_PATH);
	if let Some(parent) = template.parent() {
		std::fs::create_dir_all(parent)?;
	}
	std::fs::write(&template, sample_provider())?;

	if let Some(readme) = find_readme(root)? {
		return Ok(SampleOutcome::CreatedWithoutConsumer { template, readme });
	}

	let readme = root.join("readme.md");
	std::fs::write(&readme, sample_readme())?;
	sync_file(root, &readme)?;
	Ok(SampleOutcome::CreatedWithReadme { template, readme })
}

/// Find an existing README of any case or extension (`README.md`,
/// `Readme.markdown`, `README.rst`, ...).
fn find_readme(root: &Path) -> MdtResult<Option<PathBuf>> {
	for entry in std::fs::read_dir(root)? {
		let entry = entry?;
		let is_readme = entry
			.file_name()
			.to_str()
			.is_some_and(|name| name.to_ascii_lowercase().starts_with("readme"));
		if is_readme && entry.file_type()?.is_file() {
			return Ok(Some(entry.path()));
		}
	}
	Ok(None)
}

/// Sync the consumers in `file` only, so initialization never rewrites
/// other files in an existing project.
fn sync_file(root: &Path, file: &Path) -> MdtResult<()> {
	let ctx = scan_project_with_config(root)?;
	let mut updates = compute_updates(&ctx)?;
	updates.updated_files.retain(|path, _| path == file);
	write_updates(&updates)
}

fn ignore_cache_directory(root: &Path) -> MdtResult<GitignoreOutcome> {
	let path = root.join(".gitignore");
	if path.is_file() {
		let content = std::fs::read_to_string(&path)?;
		let already_ignored = content
			.lines()
			.map(str::trim)
			.any(|line| matches!(line, ".mdt" | ".mdt/" | "/.mdt" | "/.mdt/"));
		if already_ignored {
			return Ok(GitignoreOutcome::AlreadyIgnored(path));
		}
		let separator = if content.is_empty() || content.ends_with('\n') {
			""
		} else {
			"\n"
		};
		std::fs::write(
			&path,
			format!("{content}{separator}\n# mdt cache\n{CACHE_IGNORE_ENTRY}\n"),
		)?;
		return Ok(GitignoreOutcome::Updated(path));
	}

	if root.join(".git").exists() {
		std::fs::write(&path, format!("# mdt cache\n{CACHE_IGNORE_ENTRY}\n"))?;
		return Ok(GitignoreOutcome::Created(path));
	}

	Ok(GitignoreOutcome::NotApplicable)
}

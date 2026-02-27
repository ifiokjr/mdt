use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

use globset::Glob;
use globset::GlobSet;
use globset::GlobSetBuilder;
use ignore::gitignore::Gitignore;
use ignore::gitignore::GitignoreBuilder;
use serde::Deserialize;
use serde::Serialize;

use crate::Block;
use crate::BlockType;
use crate::MdtError;
use crate::MdtResult;
use crate::config::CONFIG_FILE_CANDIDATES;
use crate::config::CodeBlockFilter;
use crate::config::DEFAULT_MAX_FILE_SIZE;
use crate::config::MdtConfig;
use crate::config::PaddingConfig;
use crate::engine::validate_transformers;
use crate::index_cache;
use crate::index_cache::FileFingerprint;
use crate::index_cache::ProjectIndexCache;
use crate::parser::parse_with_diagnostics;
use crate::source_scanner::parse_source_with_diagnostics;

/// Options for controlling how a project is scanned.
///
/// Use [`ScanOptions::default()`] for sensible defaults or
/// [`ScanOptions::from_config`] to construct from an [`MdtConfig`].
#[derive(Debug, Clone)]
pub struct ScanOptions {
	/// Gitignore-style patterns to exclude from scanning.
	pub exclude_patterns: Vec<String>,
	/// Glob patterns restricting which files to include.
	pub include_set: GlobSet,
	/// Directories to search for template files.
	pub template_paths: Vec<PathBuf>,
	/// Maximum file size to scan in bytes.
	pub max_file_size: u64,
	/// Whether to disable `.gitignore` integration.
	pub disable_gitignore: bool,
	/// How to handle markdown code blocks.
	pub markdown_codeblocks: CodeBlockFilter,
	/// Block names to exclude from scanning.
	pub excluded_blocks: Vec<String>,
}

impl Default for ScanOptions {
	fn default() -> Self {
		Self {
			exclude_patterns: Vec::new(),
			include_set: GlobSet::empty(),
			template_paths: Vec::new(),
			max_file_size: DEFAULT_MAX_FILE_SIZE,
			disable_gitignore: false,
			markdown_codeblocks: CodeBlockFilter::default(),
			excluded_blocks: Vec::new(),
		}
	}
}

impl ScanOptions {
	/// Construct [`ScanOptions`] from an [`MdtConfig`].
	///
	/// This extracts the relevant scanning parameters from the configuration
	/// and builds the include glob set.
	pub fn from_config(config: Option<&MdtConfig>) -> Self {
		let exclude_patterns = config
			.map(|c| c.exclude.patterns.clone())
			.unwrap_or_default();
		let include_patterns = config.map(|c| &c.include.patterns[..]).unwrap_or_default();
		let template_paths = config
			.map(|c| c.templates.paths.clone())
			.unwrap_or_default();
		let max_file_size = config.map_or(DEFAULT_MAX_FILE_SIZE, |c| c.max_file_size);
		let disable_gitignore = config.is_some_and(|c| c.disable_gitignore);
		let markdown_codeblocks = config
			.map(|c| c.exclude.markdown_codeblocks.clone())
			.unwrap_or_default();
		let excluded_blocks = config.map(|c| c.exclude.blocks.clone()).unwrap_or_default();
		let include_set = build_glob_set(include_patterns);

		Self {
			exclude_patterns,
			include_set,
			template_paths,
			max_file_size,
			disable_gitignore,
			markdown_codeblocks,
			excluded_blocks,
		}
	}
}

/// Options controlling which validations are performed during check/update.
#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct ValidationOptions {
	/// If true, unclosed blocks are ignored (not reported as diagnostics).
	pub ignore_unclosed_blocks: bool,
	/// If true, unused provider blocks (with no consumers) are ignored.
	pub ignore_unused_blocks: bool,
	/// If true, invalid block names are ignored.
	pub ignore_invalid_names: bool,
	/// If true, unknown transformer names and invalid transformer arguments
	/// are ignored.
	pub ignore_invalid_transformers: bool,
}

/// The kind of diagnostic produced during project scanning and validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DiagnosticKind {
	/// A block was opened but never closed.
	UnclosedBlock { name: String },
	/// An unknown transformer name was used.
	UnknownTransformer { name: String },
	/// A transformer received the wrong number of arguments.
	InvalidTransformerArgs {
		name: String,
		expected: String,
		got: usize,
	},
	/// A provider block has no matching consumers.
	UnusedProvider { name: String },
}

/// A diagnostic produced during project scanning and validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDiagnostic {
	/// The file where the diagnostic was found.
	pub file: PathBuf,
	/// The kind of diagnostic.
	pub kind: DiagnosticKind,
	/// 1-indexed line number.
	pub line: usize,
	/// 1-indexed column number.
	pub column: usize,
}

impl ProjectDiagnostic {
	/// Check whether this diagnostic should be treated as an error given the
	/// supplied options.
	pub fn is_error(&self, options: &ValidationOptions) -> bool {
		match &self.kind {
			DiagnosticKind::UnclosedBlock { .. } => !options.ignore_unclosed_blocks,
			DiagnosticKind::UnknownTransformer { .. }
			| DiagnosticKind::InvalidTransformerArgs { .. } => !options.ignore_invalid_transformers,
			DiagnosticKind::UnusedProvider { .. } => !options.ignore_unused_blocks,
		}
	}

	/// Human-readable message for this diagnostic.
	pub fn message(&self) -> String {
		match &self.kind {
			DiagnosticKind::UnclosedBlock { name } => {
				format!("missing closing tag for block `{name}`")
			}
			DiagnosticKind::UnknownTransformer { name } => {
				format!("unknown transformer `{name}`")
			}
			DiagnosticKind::InvalidTransformerArgs {
				name,
				expected,
				got,
			} => format!("transformer `{name}` expects {expected} argument(s), got {got}"),
			DiagnosticKind::UnusedProvider { name } => {
				format!("provider block `{name}` has no consumers")
			}
		}
	}
}

/// A scanned project containing all discovered blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
	/// Provider blocks keyed by block name. Each value is the provider block
	/// and the file path it was found in.
	pub providers: HashMap<String, ProviderEntry>,
	/// Consumer blocks grouped by file path.
	pub consumers: Vec<ConsumerEntry>,
	/// Diagnostics collected during scanning and validation.
	pub diagnostics: Vec<ProjectDiagnostic>,
}

/// A scanned project together with its loaded template data context.
///
/// This is the main entry point returned by [`scan_project_with_config`] and
/// consumed by [`check_project`](crate::check_project) and
/// [`compute_updates`](crate::compute_updates).
#[derive(Debug)]
pub struct ProjectContext {
	/// The scanned project with providers and consumers.
	pub project: Project,
	/// Template data loaded from files referenced in `mdt.toml`.
	pub data: HashMap<String, serde_json::Value>,
	/// Padding configuration controlling blank lines between tags and content.
	/// `None` means no padding is applied.
	pub padding: Option<PaddingConfig>,
}

impl ProjectContext {
	/// Find all provider block names referenced by consumers but missing a
	/// provider definition.
	pub fn find_missing_providers(&self) -> Vec<String> {
		find_missing_providers(&self.project)
	}
}

/// A provider block with its source file and content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEntry {
	pub block: Block,
	pub file: PathBuf,
	/// The raw content between the provider's opening and closing tags.
	pub content: String,
}

/// A consumer block with its source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerEntry {
	pub block: Block,
	pub file: PathBuf,
	/// The current content between the consumer's opening and closing tags.
	pub content: String,
}

/// Scan a directory and discover all provider and consumer blocks.
pub fn scan_project(root: &Path) -> MdtResult<Project> {
	scan_project_with_options(root, &ScanOptions::default())
}

/// Scan a project with config — loads discovered project config, reads data files, and scans.
pub fn scan_project_with_config(root: &Path) -> MdtResult<ProjectContext> {
	let config = MdtConfig::load(root)?;
	let options = ScanOptions::from_config(config.as_ref());
	let project = scan_project_with_options(root, &options)?;
	let padding = config.as_ref().and_then(|c| c.padding.clone());
	let data = match config {
		Some(config) => config.load_data(root)?,
		None => HashMap::new(),
	};

	Ok(ProjectContext {
		project,
		data,
		padding,
	})
}

/// Build a `GlobSet` from a list of glob pattern strings.
fn build_glob_set(patterns: &[String]) -> GlobSet {
	let mut builder = GlobSetBuilder::new();
	for pattern in patterns {
		if let Ok(glob) = Glob::new(pattern) {
			builder.add(glob);
		}
	}
	builder.build().unwrap_or_else(|_| GlobSet::empty())
}

/// Normalize CRLF line endings to LF.
pub fn normalize_line_endings(content: &str) -> String {
	if content.contains('\r') {
		content.replace("\r\n", "\n").replace('\r', "\n")
	} else {
		content.to_string()
	}
}

fn build_project_cache_key(options: &ScanOptions) -> String {
	let mut exclude_patterns = options.exclude_patterns.clone();
	exclude_patterns.sort();

	let mut template_paths: Vec<String> = options
		.template_paths
		.iter()
		.map(|path| path.to_string_lossy().replace('\\', "/"))
		.collect();
	template_paths.sort();

	let mut excluded_blocks = options.excluded_blocks.clone();
	excluded_blocks.sort();

	format!(
		"index-v1|max={}|disable_gitignore={}|markdown={:?\
		 }|exclude={}|templates={}|excluded_blocks={}",
		options.max_file_size,
		options.disable_gitignore,
		options.markdown_codeblocks,
		exclude_patterns.join("\u{1f}"),
		template_paths.join("\u{1f}"),
		excluded_blocks.join("\u{1f}"),
	)
}

fn collect_file_fingerprints(
	root: &Path,
	files: &[PathBuf],
	max_file_size: u64,
) -> MdtResult<BTreeMap<String, FileFingerprint>> {
	let mut fingerprints = BTreeMap::new();

	for file in files {
		let metadata = std::fs::metadata(file)?;
		if metadata.len() > max_file_size {
			return Err(MdtError::FileTooLarge {
				path: file.display().to_string(),
				size: metadata.len(),
				limit: max_file_size,
			});
		}

		fingerprints.insert(
			index_cache::relative_file_key(root, file),
			index_cache::build_file_fingerprint(&metadata),
		);
	}

	Ok(fingerprints)
}

/// Scan a directory with the given [`ScanOptions`].
pub fn scan_project_with_options(root: &Path, options: &ScanOptions) -> MdtResult<Project> {
	let mut providers: HashMap<String, ProviderEntry> = HashMap::new();
	let mut consumers = Vec::new();

	let mut files = collect_files(root, &options.exclude_patterns, options.disable_gitignore)?;

	// Collect files from additional template directories.
	for template_dir in &options.template_paths {
		let abs_dir = root.join(template_dir);
		if abs_dir.is_dir() {
			let extra_files = collect_files(
				&abs_dir,
				&options.exclude_patterns,
				options.disable_gitignore,
			)?;
			for f in extra_files {
				if !files.contains(&f) {
					files.push(f);
				}
			}
		}
	}

	// Build exclude matcher for include filtering.
	let custom_exclude = build_exclude_matcher(root, &options.exclude_patterns)?;

	// Collect files matching include patterns.
	if !options.include_set.is_empty() {
		collect_included_files(
			root,
			root,
			&options.include_set,
			&custom_exclude,
			&mut files,
			true,
		)?;
	}

	let project_key = build_project_cache_key(options);
	let file_fingerprints = collect_file_fingerprints(root, &files, options.max_file_size)?;

	if let Some(cache) = index_cache::load(root, &project_key) {
		if cache.files == file_fingerprints {
			return Ok(cache.project);
		}
	}

	let mut diagnostics: Vec<ProjectDiagnostic> = Vec::new();

	for file in &files {
		let raw_content = std::fs::read_to_string(file)?;
		let content = normalize_line_endings(&raw_content);
		let (blocks, parse_diagnostics) = if is_markdown_file(file) {
			parse_with_diagnostics(&content)?
		} else {
			parse_source_with_diagnostics(&content, &options.markdown_codeblocks)?
		};

		// Convert parse diagnostics to project diagnostics.
		for diag in parse_diagnostics {
			let project_diag = match diag {
				crate::parser::ParseDiagnostic::UnclosedBlock { name, line, column } => {
					ProjectDiagnostic {
						file: file.clone(),
						kind: DiagnosticKind::UnclosedBlock { name },
						line,
						column,
					}
				}
				crate::parser::ParseDiagnostic::UnknownTransformer { name, line, column } => {
					ProjectDiagnostic {
						file: file.clone(),
						kind: DiagnosticKind::UnknownTransformer { name },
						line,
						column,
					}
				}
				crate::parser::ParseDiagnostic::InvalidTransformerArgs {
					name,
					expected,
					got,
					line,
					column,
				} => {
					ProjectDiagnostic {
						file: file.clone(),
						kind: DiagnosticKind::InvalidTransformerArgs {
							name,
							expected,
							got,
						},
						line,
						column,
					}
				}
			};
			diagnostics.push(project_diag);
		}

		let is_template = file
			.file_name()
			.and_then(|name| name.to_str())
			.is_some_and(|name| name.ends_with(".t.md"));

		for block in &blocks {
			// Validate transformer arguments.
			if let Err(MdtError::InvalidTransformerArgs {
				name,
				expected,
				got,
			}) = validate_transformers(&block.transformers)
			{
				diagnostics.push(ProjectDiagnostic {
					file: file.clone(),
					kind: DiagnosticKind::InvalidTransformerArgs {
						name,
						expected,
						got,
					},
					line: block.opening.start.line,
					column: block.opening.start.column,
				});
			}
		}

		for block in blocks {
			// Skip blocks whose names are in the excluded_blocks list.
			if options
				.excluded_blocks
				.iter()
				.any(|name| name == &block.name)
			{
				continue;
			}

			let block_content = extract_content_between_tags(&content, &block);

			match block.r#type {
				BlockType::Provider => {
					if !is_template {
						continue;
					}
					if let Some(existing) = providers.get(&block.name) {
						return Err(MdtError::DuplicateProvider {
							name: block.name.clone(),
							first_file: existing.file.display().to_string(),
							second_file: file.display().to_string(),
						});
					}
					providers.insert(
						block.name.clone(),
						ProviderEntry {
							block,
							file: file.clone(),
							content: block_content,
						},
					);
				}
				BlockType::Consumer => {
					consumers.push(ConsumerEntry {
						block,
						file: file.clone(),
						content: block_content,
					});
				}
			}
		}
	}

	// Check for unused providers.
	let referenced_names: HashSet<&str> = consumers.iter().map(|c| c.block.name.as_str()).collect();
	for (name, entry) in &providers {
		if !referenced_names.contains(name.as_str()) {
			diagnostics.push(ProjectDiagnostic {
				file: entry.file.clone(),
				kind: DiagnosticKind::UnusedProvider { name: name.clone() },
				line: entry.block.opening.start.line,
				column: entry.block.opening.start.column,
			});
		}
	}

	let project = Project {
		providers,
		consumers,
		diagnostics,
	};

	let cache = ProjectIndexCache::new(project_key, file_fingerprints, project.clone());
	index_cache::save(root, &cache);

	Ok(project)
}
/// Extract the text content between a block's opening tag end and closing tag
/// start. The opening position's end marks where the opening comment ends,
/// and the closing position's start marks where the closing comment begins.
pub fn extract_content_between_tags(source: &str, block: &Block) -> String {
	let start_offset = block.opening.end.offset;
	let end_offset = block.closing.start.offset;

	if start_offset >= end_offset || end_offset > source.len() {
		return String::new();
	}

	source[start_offset..end_offset].to_string()
}

/// Build a `Gitignore` matcher from exclude patterns specified in
/// `mdt.toml` `[exclude]`. These follow `.gitignore` syntax and are applied
/// on top of any `.gitignore` rules.
fn build_exclude_matcher(root: &Path, patterns: &[String]) -> MdtResult<Gitignore> {
	let mut builder = GitignoreBuilder::new(root);
	for pattern in patterns {
		builder.add_line(None, pattern).map_err(|e| {
			MdtError::ConfigParse(format!("invalid exclude pattern `{pattern}`: {e}"))
		})?;
	}
	builder
		.build()
		.map_err(|e| MdtError::ConfigParse(format!("failed to build exclude rules: {e}")))
}

/// Build a `Gitignore` matcher from the project's `.gitignore` file (if any).
fn build_gitignore(root: &Path) -> Gitignore {
	let mut builder = GitignoreBuilder::new(root);
	// Add the project root's .gitignore if it exists.
	let gitignore_path = root.join(".gitignore");
	if gitignore_path.exists() {
		let _ = builder.add(gitignore_path);
	}
	builder.build().unwrap_or_else(|_| {
		let empty = GitignoreBuilder::new(root);
		empty.build().unwrap_or_else(|_| {
			// Should never happen — an empty builder always succeeds.
			Gitignore::empty()
		})
	})
}

/// Collect all markdown and relevant source files from a directory tree.
///
/// When `disable_gitignore` is false (the default), files matched by the
/// project's `.gitignore` are skipped. Exclude patterns from `[exclude]` in
/// `mdt.toml` follow gitignore syntax and are always applied on top.
fn collect_files(
	root: &Path,
	exclude_patterns: &[String],
	disable_gitignore: bool,
) -> MdtResult<Vec<PathBuf>> {
	let mut files = Vec::new();
	let mut visited_dirs = HashSet::new();

	// Build gitignore matcher (respects .gitignore unless disabled).
	let gitignore = if disable_gitignore {
		Gitignore::empty()
	} else {
		build_gitignore(root)
	};

	// Build exclude matcher from mdt.toml [exclude] patterns.
	let custom_exclude = build_exclude_matcher(root, exclude_patterns)?;

	walk_dir(
		root,
		root,
		&mut files,
		true,
		&gitignore,
		&custom_exclude,
		&mut visited_dirs,
	)?;
	// Sort for deterministic ordering.
	files.sort();
	Ok(files)
}

fn is_ignored_directory_name(name: &str) -> bool {
	(name.starts_with('.') && name != ".templates") || name == "node_modules" || name == "target"
}

fn has_project_config(dir: &Path) -> bool {
	CONFIG_FILE_CANDIDATES
		.iter()
		.any(|candidate| dir.join(candidate).is_file())
}

#[allow(clippy::only_used_in_recursion)]
fn walk_dir(
	root: &Path,
	dir: &Path,
	files: &mut Vec<PathBuf>,
	is_root: bool,
	gitignore: &Gitignore,
	custom_exclude: &Gitignore,
	visited_dirs: &mut HashSet<PathBuf>,
) -> MdtResult<()> {
	if !dir.is_dir() {
		return Ok(());
	}

	// Detect symlink cycles by tracking canonical paths.
	let canonical = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
	if !visited_dirs.insert(canonical.clone()) {
		return Err(MdtError::SymlinkCycle {
			path: dir.display().to_string(),
		});
	}

	let entries = std::fs::read_dir(dir)?;

	for entry in entries {
		let entry = entry?;
		let path = entry.path();

		// Skip hidden directories and common non-source directories.
		if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
			if is_ignored_directory_name(name) {
				continue;
			}
		}

		let is_dir = path.is_dir();

		// Check against gitignore patterns.
		if gitignore.matched(&path, is_dir).is_ignore() {
			continue;
		}

		// Check against exclude patterns from mdt.toml [exclude].
		if custom_exclude.matched(&path, is_dir).is_ignore() {
			continue;
		}

		if is_dir {
			// Skip subdirectories that have their own mdt config file (separate
			// project scope).
			if !is_root && has_project_config(&path) {
				continue;
			}
			walk_dir(
				root,
				&path,
				files,
				false,
				gitignore,
				custom_exclude,
				visited_dirs,
			)?;
		} else if is_scannable_file(&path) {
			files.push(path);
		}
	}

	Ok(())
}

/// Recursively collect files matching include patterns.
fn collect_included_files(
	root: &Path,
	dir: &Path,
	include_set: &GlobSet,
	exclude_matcher: &Gitignore,
	files: &mut Vec<PathBuf>,
	is_root: bool,
) -> MdtResult<()> {
	if !dir.is_dir() {
		return Ok(());
	}

	let entries = std::fs::read_dir(dir)?;

	for entry in entries {
		let entry = entry?;
		let path = entry.path();

		if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
			if is_ignored_directory_name(name) {
				continue;
			}
		}

		let is_dir = path.is_dir();

		// Check against exclude patterns.
		if exclude_matcher.matched(&path, is_dir).is_ignore() {
			continue;
		}

		if let Ok(rel_path) = path.strip_prefix(root) {
			if path.is_file() && include_set.is_match(rel_path) && !files.contains(&path) {
				files.push(path.clone());
			}
		}

		if is_dir {
			if !is_root && has_project_config(&path) {
				continue;
			}
			collect_included_files(root, &path, include_set, exclude_matcher, files, false)?;
		}
	}

	Ok(())
}

/// Check if a file should be scanned for mdt blocks.
fn is_scannable_file(path: &Path) -> bool {
	let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
		return false;
	};

	matches!(
		ext,
		"md" | "mdx"
			| "markdown"
			| "rs" | "ts"
			| "tsx" | "js"
			| "jsx" | "py"
			| "go" | "java"
			| "kt" | "swift"
			| "c" | "cpp"
			| "h" | "cs"
	)
}

/// Check if a file is a markdown file (parsed via the markdown AST).
fn is_markdown_file(path: &Path) -> bool {
	let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
		return false;
	};

	matches!(ext, "md" | "mdx" | "markdown")
}

/// Check if a specific file is a template definition file.
pub fn is_template_file(path: &Path) -> bool {
	path.file_name()
		.and_then(|name| name.to_str())
		.is_some_and(|name| name.ends_with(".t.md"))
}

/// Find all provider block names that are referenced by consumers but have no
/// matching provider.
pub fn find_missing_providers(project: &Project) -> Vec<String> {
	let mut missing = Vec::new();
	for consumer in &project.consumers {
		if !project.providers.contains_key(&consumer.block.name)
			&& !missing.contains(&consumer.block.name)
		{
			missing.push(consumer.block.name.clone());
		}
	}
	missing
}

/// Validate that all consumer blocks have matching providers.
pub fn validate_project(project: &Project) -> MdtResult<()> {
	let missing = find_missing_providers(project);
	if let Some(name) = missing.into_iter().next() {
		return Err(MdtError::MissingProvider(name));
	}
	Ok(())
}

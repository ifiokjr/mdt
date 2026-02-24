use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

use globset::Glob;
use globset::GlobSet;
use globset::GlobSetBuilder;

use crate::Block;
use crate::BlockType;
use crate::MdtError;
use crate::MdtResult;
use crate::config::DEFAULT_MAX_FILE_SIZE;
use crate::config::MdtConfig;
use crate::engine::validate_transformers;
use crate::parser::parse_with_diagnostics;
use crate::source_scanner::parse_source_with_diagnostics;

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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
#[derive(Debug)]
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
	/// When true, ensure a newline always separates the opening tag from the
	/// content and the content from the closing tag.
	pub pad_blocks: bool,
}

impl ProjectContext {
	/// Find all provider block names referenced by consumers but missing a
	/// provider definition.
	pub fn find_missing_providers(&self) -> Vec<String> {
		find_missing_providers(&self.project)
	}
}

/// A provider block with its source file and content.
#[derive(Debug, Clone)]
pub struct ProviderEntry {
	pub block: Block,
	pub file: PathBuf,
	/// The raw content between the provider's opening and closing tags.
	pub content: String,
}

/// A consumer block with its source file.
#[derive(Debug, Clone)]
pub struct ConsumerEntry {
	pub block: Block,
	pub file: PathBuf,
	/// The current content between the consumer's opening and closing tags.
	pub content: String,
}

/// Scan a directory and discover all provider and consumer blocks.
pub fn scan_project(root: &Path) -> MdtResult<Project> {
	scan_project_with_options(
		root,
		&GlobSet::empty(),
		&GlobSet::empty(),
		&[],
		DEFAULT_MAX_FILE_SIZE,
	)
}

/// Scan a project with config — loads `mdt.toml`, reads data files, and scans.
pub fn scan_project_with_config(root: &Path) -> MdtResult<ProjectContext> {
	let config = MdtConfig::load(root)?;
	let exclude_patterns = config
		.as_ref()
		.map(|c| &c.exclude.patterns[..])
		.unwrap_or_default();
	let include_patterns = config
		.as_ref()
		.map(|c| &c.include.patterns[..])
		.unwrap_or_default();
	let template_paths = config
		.as_ref()
		.map(|c| &c.templates.paths[..])
		.unwrap_or_default();
	let max_file_size = config
		.as_ref()
		.map_or(DEFAULT_MAX_FILE_SIZE, |c| c.max_file_size);
	let exclude_set = build_glob_set(exclude_patterns);
	let include_set = build_glob_set(include_patterns);
	let project = scan_project_with_options(
		root,
		&exclude_set,
		&include_set,
		template_paths,
		max_file_size,
	)?;
	let pad_blocks = config.as_ref().is_some_and(|c| c.pad_blocks);
	let data = match config {
		Some(config) => config.load_data(root)?,
		None => HashMap::new(),
	};

	Ok(ProjectContext {
		project,
		data,
		pad_blocks,
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

/// Scan a directory with exclude/include patterns and extra template paths.
pub(crate) fn scan_project_with_options(
	root: &Path,
	exclude_set: &GlobSet,
	include_set: &GlobSet,
	template_paths: &[PathBuf],
	max_file_size: u64,
) -> MdtResult<Project> {
	let mut providers: HashMap<String, ProviderEntry> = HashMap::new();
	let mut consumers = Vec::new();

	let mut files = collect_files(root, exclude_set)?;

	// Collect files from additional template directories.
	for template_dir in template_paths {
		let abs_dir = root.join(template_dir);
		if abs_dir.is_dir() {
			let extra_files = collect_files(&abs_dir, exclude_set)?;
			for f in extra_files {
				if !files.contains(&f) {
					files.push(f);
				}
			}
		}
	}

	// Collect files matching include patterns.
	if !include_set.is_empty() {
		collect_included_files(root, root, include_set, exclude_set, &mut files)?;
	}

	let mut diagnostics: Vec<ProjectDiagnostic> = Vec::new();

	for file in &files {
		// Check file size before reading.
		let metadata = std::fs::metadata(file)?;
		if metadata.len() > max_file_size {
			return Err(MdtError::FileTooLarge {
				path: file.display().to_string(),
				size: metadata.len(),
				limit: max_file_size,
			});
		}

		let raw_content = std::fs::read_to_string(file)?;
		let content = normalize_line_endings(&raw_content);
		let (blocks, parse_diagnostics) = if is_markdown_file(file) {
			parse_with_diagnostics(&content)?
		} else {
			parse_source_with_diagnostics(&content)?
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

	Ok(Project {
		providers,
		consumers,
		diagnostics,
	})
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

/// Collect all markdown and relevant source files from a directory tree.
fn collect_files(root: &Path, exclude_set: &GlobSet) -> MdtResult<Vec<PathBuf>> {
	let mut files = Vec::new();
	let mut visited_dirs = HashSet::new();
	walk_dir(root, root, &mut files, true, exclude_set, &mut visited_dirs)?;
	// Sort for deterministic ordering
	files.sort();
	Ok(files)
}

fn walk_dir(
	root: &Path,
	dir: &Path,
	files: &mut Vec<PathBuf>,
	is_root: bool,
	exclude_set: &GlobSet,
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

		// Skip hidden directories and common non-source directories
		if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
			if name.starts_with('.') || name == "node_modules" || name == "target" {
				continue;
			}
		}

		// Check against exclude patterns using relative path
		if let Ok(rel_path) = path.strip_prefix(root) {
			if !exclude_set.is_empty() && exclude_set.is_match(rel_path) {
				continue;
			}
		}

		if path.is_dir() {
			// Skip subdirectories that have their own mdt.toml (separate
			// project scope).
			if !is_root && path.join("mdt.toml").exists() {
				continue;
			}
			walk_dir(root, &path, files, false, exclude_set, visited_dirs)?;
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
	exclude_set: &GlobSet,
	files: &mut Vec<PathBuf>,
) -> MdtResult<()> {
	if !dir.is_dir() {
		return Ok(());
	}

	let entries = std::fs::read_dir(dir)?;

	for entry in entries {
		let entry = entry?;
		let path = entry.path();

		if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
			if name.starts_with('.') || name == "node_modules" || name == "target" {
				continue;
			}
		}

		if let Ok(rel_path) = path.strip_prefix(root) {
			if !exclude_set.is_empty() && exclude_set.is_match(rel_path) {
				continue;
			}

			if path.is_file() && include_set.is_match(rel_path) && !files.contains(&path) {
				files.push(path.clone());
			}
		}

		if path.is_dir() {
			collect_included_files(root, &path, include_set, exclude_set, files)?;
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

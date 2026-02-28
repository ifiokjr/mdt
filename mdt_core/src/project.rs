use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
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
use crate::parser::ParseDiagnostic;
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
	/// Whether to include content hashes in file fingerprints for cache validation.
	pub cache_verify_hash: bool,
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
			cache_verify_hash: false,
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
		let cache_verify_hash = std::env::var_os("MDT_CACHE_VERIFY_HASH").is_some();
		let include_set = build_glob_set(include_patterns);

		Self {
			exclude_patterns,
			include_set,
			template_paths,
			max_file_size,
			disable_gitignore,
			markdown_codeblocks,
			excluded_blocks,
			cache_verify_hash,
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

/// Metrics for the most recent cache-assisted project scan.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectCacheLastScan {
	/// Unix timestamp in milliseconds when the scan completed.
	pub timestamp_unix_ms: u64,
	/// Whether the scan reused the entire cached project without reparsing.
	pub full_project_hit: bool,
	/// Number of files reused from cache.
	pub reused_files: u64,
	/// Number of files reparsed from disk.
	pub reparsed_files: u64,
	/// Total files considered by this scan.
	pub total_files: u64,
}

/// Cumulative cache telemetry persisted in the project index cache artifact.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectCacheTelemetry {
	/// Number of scans recorded in this cache artifact lineage.
	pub scan_count: u64,
	/// Number of scans that were full cache hits.
	pub full_project_hit_count: u64,
	/// Total number of file entries reused from cache across scans.
	pub reused_file_count_total: u64,
	/// Total number of file entries reparsed from disk across scans.
	pub reparsed_file_count_total: u64,
	/// Metrics for the most recent scan, if available.
	pub last_scan: Option<ProjectCacheLastScan>,
}

/// Read-only inspection of the on-disk project index cache artifact.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectCacheInspection {
	/// Absolute path to the cache artifact.
	pub path: PathBuf,
	/// Whether a cache artifact file exists at the expected path.
	pub exists: bool,
	/// Whether the artifact could be read from disk.
	pub readable: bool,
	/// Whether the artifact parsed and matched the supported schema.
	pub valid: bool,
	/// Schema version read from the artifact, if present.
	pub schema_version: Option<u32>,
	/// Whether the artifact schema matches the current implementation.
	pub schema_supported: bool,
	/// Whether the artifact key matches current scan options.
	pub project_key_matches: bool,
	/// Whether content-hash cache verification is enabled for this scan mode.
	pub hash_verification_enabled: bool,
	/// Persisted telemetry metrics if the artifact parsed successfully.
	pub telemetry: Option<ProjectCacheTelemetry>,
}

impl From<index_cache::LastScanTelemetry> for ProjectCacheLastScan {
	fn from(value: index_cache::LastScanTelemetry) -> Self {
		Self {
			timestamp_unix_ms: value.timestamp_unix_ms,
			full_project_hit: value.full_project_hit,
			reused_files: value.reused_files,
			reparsed_files: value.reparsed_files,
			total_files: value.total_files,
		}
	}
}

impl From<index_cache::CacheTelemetry> for ProjectCacheTelemetry {
	fn from(value: index_cache::CacheTelemetry) -> Self {
		Self {
			scan_count: value.scan_count,
			full_project_hit_count: value.full_project_hit_count,
			reused_file_count_total: value.reused_file_count_total,
			reparsed_file_count_total: value.reparsed_file_count_total,
			last_scan: value.last_scan.map(Into::into),
		}
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
		"index-v2|max={}|disable_gitignore={}|markdown={:?\
		 }|exclude={}|templates={}|excluded_blocks={}|cache_verify_hash={}",
		options.max_file_size,
		options.disable_gitignore,
		options.markdown_codeblocks,
		exclude_patterns.join("\u{1f}"),
		template_paths.join("\u{1f}"),
		excluded_blocks.join("\u{1f}"),
		options.cache_verify_hash,
	)
}

/// Return the absolute path to the current project's cache artifact.
pub fn project_cache_path(root: &Path) -> PathBuf {
	index_cache::cache_path(root)
}

/// Inspect the project's cache artifact without mutating it.
///
/// This is intended for diagnostics surfaces (`mdt info`, `mdt doctor`) that
/// need to report cache health and telemetry details.
pub fn inspect_project_cache(root: &Path, options: &ScanOptions) -> ProjectCacheInspection {
	let path = project_cache_path(root);
	let mut inspection = ProjectCacheInspection {
		path: path.clone(),
		exists: path.is_file(),
		readable: false,
		valid: false,
		schema_version: None,
		schema_supported: false,
		project_key_matches: false,
		hash_verification_enabled: options.cache_verify_hash,
		telemetry: None,
	};

	if !inspection.exists {
		return inspection;
	}

	let Ok(bytes) = std::fs::read(&path) else {
		return inspection;
	};
	inspection.readable = true;

	let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
		return inspection;
	};

	let schema_version = value
		.get("schema_version")
		.and_then(serde_json::Value::as_u64)
		.and_then(|version| u32::try_from(version).ok());
	inspection.schema_version = schema_version;
	inspection.schema_supported = schema_version == Some(index_cache::CACHE_SCHEMA_VERSION);

	let expected_project_key = build_project_cache_key(options);
	inspection.project_key_matches = value
		.get("project_key")
		.and_then(serde_json::Value::as_str)
		.is_some_and(|key| key == expected_project_key);

	let Ok(cache) = serde_json::from_value::<ProjectIndexCache>(value) else {
		return inspection;
	};

	inspection.valid = inspection.schema_supported;
	inspection.telemetry = Some(cache.telemetry.into());
	inspection
}

fn collect_file_fingerprints(
	root: &Path,
	files: &[PathBuf],
	max_file_size: u64,
	verify_hash: bool,
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

		let content_hash = if verify_hash {
			Some(hash_file_contents(file)?)
		} else {
			None
		};

		fingerprints.insert(
			index_cache::relative_file_key(root, file),
			index_cache::build_file_fingerprint(&metadata, content_hash),
		);
	}

	Ok(fingerprints)
}

fn hash_file_contents(path: &Path) -> MdtResult<u64> {
	let bytes = std::fs::read(path)?;
	let mut hasher = DefaultHasher::new();
	bytes.hash(&mut hasher);
	Ok(hasher.finish())
}

fn parse_diagnostic_to_project(file: &Path, diag: ParseDiagnostic) -> ProjectDiagnostic {
	match diag {
		ParseDiagnostic::UnclosedBlock { name, line, column } => ProjectDiagnostic {
			file: file.to_path_buf(),
			kind: DiagnosticKind::UnclosedBlock { name },
			line,
			column,
		},
		ParseDiagnostic::UnknownTransformer { name, line, column } => ProjectDiagnostic {
			file: file.to_path_buf(),
			kind: DiagnosticKind::UnknownTransformer { name },
			line,
			column,
		},
		ParseDiagnostic::InvalidTransformerArgs {
			name,
			expected,
			got,
			line,
			column,
		} => ProjectDiagnostic {
			file: file.to_path_buf(),
			kind: DiagnosticKind::InvalidTransformerArgs {
				name,
				expected,
				got,
			},
			line,
			column,
		},
	}
}

fn parse_file_for_scan(
	file: &Path,
	options: &ScanOptions,
) -> MdtResult<index_cache::CachedFileData> {
	let raw_content = std::fs::read_to_string(file)?;
	let content = normalize_line_endings(&raw_content);
	let (blocks, parse_diagnostics) = if is_markdown_file(file) {
		parse_with_diagnostics(&content)?
	} else {
		parse_source_with_diagnostics(&content, &options.markdown_codeblocks)?
	};

	let mut diagnostics: Vec<ProjectDiagnostic> = parse_diagnostics
		.into_iter()
		.map(|diag| parse_diagnostic_to_project(file, diag))
		.collect();
	let mut providers = Vec::new();
	let mut consumers = Vec::new();

	let is_template = file
		.file_name()
		.and_then(|name| name.to_str())
		.is_some_and(|name| name.ends_with(".t.md"));

	for block in &blocks {
		if let Err(MdtError::InvalidTransformerArgs {
			name,
			expected,
			got,
		}) = validate_transformers(&block.transformers)
		{
			diagnostics.push(ProjectDiagnostic {
				file: file.to_path_buf(),
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
				providers.push(ProviderEntry {
					block,
					file: file.to_path_buf(),
					content: block_content,
				});
			}
			BlockType::Consumer | BlockType::Inline => {
				consumers.push(ConsumerEntry {
					block,
					file: file.to_path_buf(),
					content: block_content,
				});
			}
		}
	}

	Ok(index_cache::CachedFileData {
		providers,
		consumers,
		diagnostics,
	})
}

fn build_project_from_file_data(
	root: &Path,
	files: &[PathBuf],
	file_data: &BTreeMap<String, index_cache::CachedFileData>,
) -> MdtResult<Project> {
	let mut providers: HashMap<String, ProviderEntry> = HashMap::new();
	let mut consumers = Vec::new();
	let mut diagnostics = Vec::new();

	for file in files {
		let file_key = index_cache::relative_file_key(root, file);
		let Some(entry) = file_data.get(&file_key) else {
			continue;
		};

		diagnostics.extend(entry.diagnostics.iter().cloned());
		for provider in &entry.providers {
			if let Some(existing) = providers.get(&provider.block.name) {
				return Err(MdtError::DuplicateProvider {
					name: provider.block.name.clone(),
					first_file: existing.file.display().to_string(),
					second_file: provider.file.display().to_string(),
				});
			}

			providers.insert(provider.block.name.clone(), provider.clone());
		}
		consumers.extend(entry.consumers.iter().cloned());
	}

	let referenced_names: HashSet<&str> = consumers
		.iter()
		.filter(|consumer| consumer.block.r#type == BlockType::Consumer)
		.map(|consumer| consumer.block.name.as_str())
		.collect();
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

/// Scan a directory with the given [`ScanOptions`].
pub fn scan_project_with_options(root: &Path, options: &ScanOptions) -> MdtResult<Project> {
	let mut files = collect_files(root, &options.exclude_patterns, options.disable_gitignore)?;

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

	let custom_exclude = build_exclude_matcher(root, &options.exclude_patterns)?;

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
	let file_fingerprints = collect_file_fingerprints(
		root,
		&files,
		options.max_file_size,
		options.cache_verify_hash,
	)?;
	let mut cache = index_cache::load(root, &project_key);

	if let Some(cached) = &mut cache {
		if cached.files == file_fingerprints {
			cached
				.telemetry
				.record_scan(true, files.len(), 0, files.len());
			index_cache::save(root, cached);
			return Ok(cached.project.clone());
		}
	}

	let mut merged_file_data = BTreeMap::new();
	let mut reused_file_count = 0usize;
	let mut reparsed_file_count = 0usize;
	for file in &files {
		let file_key = index_cache::relative_file_key(root, file);
		let fingerprint = file_fingerprints.get(&file_key);
		let cached_entry = cache.as_ref().and_then(|cached| {
			if cached.files.get(&file_key) == fingerprint {
				return cached.file_data.get(&file_key).cloned();
			}

			None
		});

		let entry = if let Some(entry) = cached_entry {
			reused_file_count = reused_file_count.saturating_add(1);
			entry
		} else {
			reparsed_file_count = reparsed_file_count.saturating_add(1);
			parse_file_for_scan(file, options)?
		};

		merged_file_data.insert(file_key, entry);
	}

	let project = build_project_from_file_data(root, &files, &merged_file_data)?;
	let mut next_cache = ProjectIndexCache::new(
		project_key,
		file_fingerprints,
		merged_file_data,
		project.clone(),
	);
	if let Some(previous_cache) = cache {
		next_cache.telemetry = previous_cache.telemetry;
	}
	next_cache
		.telemetry
		.record_scan(false, reused_file_count, reparsed_file_count, files.len());
	index_cache::save(root, &next_cache);

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
		if consumer.block.r#type != BlockType::Consumer {
			continue;
		}
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

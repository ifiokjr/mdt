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
use tracing::debug;
use tracing::instrument;

use crate::Block;
use crate::BlockType;
use crate::MdtError;
use crate::MdtResult;
use crate::config::CONFIG_FILE_CANDIDATES;
use crate::config::CodeBlockFilter;
use crate::config::DEFAULT_MAX_FILE_SIZE;
use crate::config::FormatterConfig;
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
	/// A closing tag has no open block with the same name.
	UnmatchedClosingTag { name: String },
	/// A comment that looks like an mdt tag but does not parse, so mdt
	/// ignores it.
	InvalidTag { tag: String },
	/// A block opens inside another block. Blocks cannot be nested: inside a
	/// consumer `mdt update` would overwrite the inner block, and inside a
	/// provider its tags would be copied into every consumer.
	NestedBlock { outer: String, inner: String },
	/// A provider tag outside a `*.t.md` template file, which mdt ignores.
	ProviderOutsideTemplate { name: String },
}

impl DiagnosticKind {
	/// The stable machine-readable code for this kind, such as
	/// `mdt::unclosed_block`, shared by the CLI, MCP server, and docs.
	pub fn code(&self) -> &'static str {
		match self {
			Self::UnclosedBlock { .. } => "mdt::unclosed_block",
			Self::UnknownTransformer { .. } => "mdt::unknown_transformer",
			Self::InvalidTransformerArgs { .. } => "mdt::invalid_transformer_args",
			Self::UnusedProvider { .. } => "mdt::unused_provider",
			Self::UnmatchedClosingTag { .. } => "mdt::unmatched_closing_tag",
			Self::InvalidTag { .. } => "mdt::invalid_tag",
			Self::NestedBlock { .. } => "mdt::nested_block",
			Self::ProviderOutsideTemplate { .. } => "mdt::provider_outside_template",
		}
	}
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
	/// supplied options. Errors stop `check`, `update`, and `list`.
	pub fn is_error(&self, options: &ValidationOptions) -> bool {
		match &self.kind {
			// An unmatched closing tag is the other half of a misspelled or
			// malformed opening tag, so that block silently stops syncing.
			DiagnosticKind::UnclosedBlock { .. } | DiagnosticKind::UnmatchedClosingTag { .. } => {
				!options.ignore_unclosed_blocks
			}
			DiagnosticKind::UnknownTransformer { .. }
			| DiagnosticKind::InvalidTransformerArgs { .. } => !options.ignore_invalid_transformers,
			DiagnosticKind::InvalidTag { .. } => !options.ignore_invalid_names,
			DiagnosticKind::NestedBlock { .. } => true,
			DiagnosticKind::UnusedProvider { .. }
			| DiagnosticKind::ProviderOutsideTemplate { .. } => false,
		}
	}

	/// Check whether the supplied options silence this diagnostic entirely.
	/// Diagnostics that are neither errors nor ignored are warnings.
	pub fn is_ignored(&self, options: &ValidationOptions) -> bool {
		match &self.kind {
			DiagnosticKind::UnclosedBlock { .. } | DiagnosticKind::UnmatchedClosingTag { .. } => {
				options.ignore_unclosed_blocks
			}
			DiagnosticKind::UnknownTransformer { .. }
			| DiagnosticKind::InvalidTransformerArgs { .. } => options.ignore_invalid_transformers,
			DiagnosticKind::UnusedProvider { .. } => options.ignore_unused_blocks,
			DiagnosticKind::InvalidTag { .. } => options.ignore_invalid_names,
			DiagnosticKind::NestedBlock { .. } | DiagnosticKind::ProviderOutsideTemplate { .. } => {
				false
			}
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
			DiagnosticKind::UnmatchedClosingTag { name } => {
				format!("closing tag `{{/{name}}}` has no matching opening tag")
			}
			DiagnosticKind::InvalidTag { tag } => {
				format!("`{tag}` looks like an mdt tag but cannot be parsed, so it is ignored")
			}
			DiagnosticKind::NestedBlock { outer, inner } => {
				format!("block `{inner}` is nested inside block `{outer}`; blocks cannot be nested")
			}
			DiagnosticKind::ProviderOutsideTemplate { name } => {
				format!(
					"provider block `{name}` is ignored because providers are only read from \
					 `*.t.md` files"
				)
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
	/// Project root used for config resolution and formatter execution.
	pub root: PathBuf,
	/// The scanned project with providers and consumers.
	pub project: Project,
	/// Template data loaded from files referenced in `mdt.toml`.
	pub data: HashMap<String, serde_json::Value>,
	/// Padding configuration controlling blank lines between tags and content.
	/// `None` means no padding is applied.
	pub padding: Option<PaddingConfig>,
	/// Ordered formatter pipeline entries used to normalize full-file output.
	pub formatters: Vec<FormatterConfig>,
	/// Source scanning code block filtering needed when reparsing source files.
	pub markdown_codeblocks: CodeBlockFilter,
	/// Check comparison mode (`strict` or `lenient`).
	pub comparison: crate::config::ComparisonMode,
}

impl ProjectContext {
	/// Find all provider block names referenced by consumers but missing a
	/// provider definition.
	pub fn find_missing_providers(&self) -> Vec<String> {
		find_missing_providers(&self.project)
	}
}

/// Resolve an optional project root path to an absolute path.
///
/// If `path` is `None`, falls back to the current working directory and uses
/// `.` if the current directory cannot be determined. Relative paths are
/// made absolute and `.`/`..` components are resolved lexically, so every
/// file path derived from the root — including paths stored in the scan
/// cache — stays valid no matter which directory mdt runs from.
pub fn resolve_root(path: Option<&Path>) -> PathBuf {
	let root = path.map_or_else(
		|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
		PathBuf::from,
	);
	let absolute = std::path::absolute(&root).unwrap_or(root);
	normalize_lexically(&absolute)
}

/// Remove `.` components and resolve `..` against the preceding component
/// without touching the filesystem.
pub fn normalize_lexically(path: &Path) -> PathBuf {
	let mut components: Vec<std::path::Component<'_>> = Vec::new();

	for component in path.components() {
		match component {
			std::path::Component::CurDir => {}
			std::path::Component::ParentDir => {
				let parent_is_normal =
					matches!(components.last(), Some(std::path::Component::Normal(_)));
				if parent_is_normal {
					components.pop();
				} else if !matches!(
					components.last(),
					Some(std::path::Component::RootDir | std::path::Component::Prefix(_))
				) {
					components.push(component);
				}
			}
			other => components.push(other),
		}
	}

	components.iter().collect()
}

/// Render a path relative to the project root for user-facing display.
///
/// If `path` is outside `root`, the full path is returned unchanged.
/// Separators are normalized to forward slashes on every platform so
/// diagnostics render identically across operating systems.
pub fn relative_display_path(path: &Path, root: &Path) -> String {
	let rendered = path
		.strip_prefix(root)
		.unwrap_or(path)
		.display()
		.to_string();

	if rendered.contains('\\') {
		rendered.replace('\\', "/")
	} else {
		rendered
	}
}

/// Returns true if the path uses a markdown-style extension supported by mdt.
pub fn is_markdown_path(path: &Path) -> bool {
	path.extension()
		.and_then(|ext| ext.to_str())
		.is_some_and(|ext| matches!(ext, "md" | "mdx" | "markdown"))
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

/// Readability and validity of the cache artifact itself.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectCacheArtifactState {
	/// Whether a cache artifact file exists at the expected path.
	pub exists: bool,
	/// Whether the artifact could be read from disk.
	pub readable: bool,
	/// Whether the artifact parsed and matched the supported schema.
	pub valid: bool,
}

/// Compatibility details for the current scan mode vs cached metadata.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectCacheCompatibilityState {
	/// Whether the artifact schema matches the current implementation.
	pub schema_supported: bool,
	/// Whether the artifact key matches current scan options.
	pub project_key_matches: bool,
	/// Whether content-hash cache verification is enabled for this scan mode.
	pub hash_verification_enabled: bool,
}

/// Read-only inspection of the on-disk project index cache artifact.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectCacheInspection {
	/// Absolute path to the cache artifact.
	pub path: PathBuf,
	/// Readability and validity details for the artifact.
	pub artifact: ProjectCacheArtifactState,
	/// Schema version read from the artifact, if present.
	pub schema_version: Option<u32>,
	/// Compatibility details for current scan options.
	pub compatibility: ProjectCacheCompatibilityState,
	/// Persisted telemetry metrics if the artifact parsed successfully.
	pub telemetry: Option<ProjectCacheTelemetry>,
}

impl ProjectCacheInspection {
	/// True when a cache artifact exists.
	#[must_use]
	#[inline]
	pub fn exists(&self) -> bool {
		self.artifact.exists
	}

	/// True when the cache artifact can be read from disk.
	#[must_use]
	#[inline]
	pub fn readable(&self) -> bool {
		self.artifact.readable
	}

	/// True when the cache artifact parsed and matched supported schema.
	#[must_use]
	#[inline]
	pub fn valid(&self) -> bool {
		self.artifact.valid
	}

	/// True when cache schema version is supported.
	#[must_use]
	#[inline]
	pub fn schema_supported(&self) -> bool {
		self.compatibility.schema_supported
	}

	/// True when cache key matches current scan options.
	#[must_use]
	#[inline]
	pub fn project_key_matches(&self) -> bool {
		self.compatibility.project_key_matches
	}

	/// True when content-hash verification mode is enabled.
	#[must_use]
	#[inline]
	pub fn hash_verification_enabled(&self) -> bool {
		self.compatibility.hash_verification_enabled
	}
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
///
/// # Errors
///
/// Returns [`MdtError`] if I/O operations fail or if config parsing fails.
#[instrument]
pub fn scan_project(root: &Path) -> MdtResult<Project> {
	scan_project_with_options(root, &ScanOptions::default())
}

/// Scan a project with config — loads discovered project config, reads data files, and scans.
///
/// # Errors
///
/// Returns [`MdtError`] if config loading, data file reading, or I/O operations fail.
#[instrument]
pub fn scan_project_with_config(root: &Path) -> MdtResult<ProjectContext> {
	let config = MdtConfig::load(root)?;
	debug!(config_found = config.is_some(), "loaded project config");
	let options = ScanOptions::from_config(config.as_ref());
	let project = scan_project_with_options(root, &options)?;
	debug!(
		providers = project.providers.len(),
		consumers = project.consumers.len(),
		diagnostics = project.diagnostics.len(),
		"project scan complete",
	);
	let padding = config.as_ref().and_then(|c| c.padding.clone());
	let formatters = config
		.as_ref()
		.map_or_else(Vec::new, |c| c.formatters.clone());
	let comparison = config
		.as_ref()
		.map_or_else(Default::default, |c| c.check.comparison.clone());

	let markdown_codeblocks = options.markdown_codeblocks.clone();

	let data = match config {
		Some(config) => config.load_data(root)?,
		None => HashMap::new(),
	};

	debug!(data_namespaces = data.len(), "loaded data sources");

	Ok(ProjectContext {
		root: root.to_path_buf(),
		project,
		data,
		padding,
		formatters,
		markdown_codeblocks,
		comparison,
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

/// Convert LF-normalized content back to the line-ending style of `raw`.
///
/// All block offsets are computed against LF-normalized text, so read paths
/// must normalize before splicing. This restores the original file's line
/// endings so writing an update does not churn the whole file's EOL style.
pub fn restore_line_endings(normalized: &str, raw: &str) -> String {
	if raw.contains("\r\n") {
		normalized.replace('\n', "\r\n")
	} else if raw.contains('\r') {
		normalized.replace('\n', "\r")
	} else {
		normalized.to_string()
	}
}

/// The cache stores absolute file paths, so the key includes the root: a
/// project that moved, or is reached through another path, rescans instead
/// of reusing paths that point at the old location.
fn build_project_cache_key(root: &Path, options: &ScanOptions) -> String {
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
		"index-v3|root={}|max={}|disable_gitignore={}|markdown={:?\
		 }|exclude={}|templates={}|excluded_blocks={}|cache_verify_hash={}",
		root.display(),
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
		artifact: ProjectCacheArtifactState {
			exists: path.is_file(),
			readable: false,
			valid: false,
		},
		schema_version: None,
		compatibility: ProjectCacheCompatibilityState {
			schema_supported: false,
			project_key_matches: false,
			hash_verification_enabled: options.cache_verify_hash,
		},
		telemetry: None,
	};

	if !inspection.artifact.exists {
		return inspection;
	}

	let Ok(bytes) = std::fs::read(&path) else {
		return inspection;
	};
	inspection.artifact.readable = true;

	let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
		return inspection;
	};

	let schema_version = value
		.get("schema_version")
		.and_then(serde_json::Value::as_u64)
		.and_then(|version| u32::try_from(version).ok());
	inspection.schema_version = schema_version;
	inspection.compatibility.schema_supported =
		schema_version == Some(index_cache::CACHE_SCHEMA_VERSION);

	let expected_project_key = build_project_cache_key(root, options);
	inspection.compatibility.project_key_matches = value
		.get("project_key")
		.and_then(serde_json::Value::as_str)
		.is_some_and(|key| key == expected_project_key);

	let Ok(cache) = serde_json::from_value::<ProjectIndexCache>(value) else {
		return inspection;
	};

	inspection.artifact.valid = inspection.compatibility.schema_supported;
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
		ParseDiagnostic::UnclosedBlock { name, line, column } => {
			ProjectDiagnostic {
				file: file.to_path_buf(),
				kind: DiagnosticKind::UnclosedBlock { name },
				line,
				column,
			}
		}
		ParseDiagnostic::UnknownTransformer { name, line, column } => {
			ProjectDiagnostic {
				file: file.to_path_buf(),
				kind: DiagnosticKind::UnknownTransformer { name },
				line,
				column,
			}
		}
		ParseDiagnostic::InvalidTransformerArgs {
			name,
			expected,
			got,
			line,
			column,
		} => {
			ProjectDiagnostic {
				file: file.to_path_buf(),
				kind: DiagnosticKind::InvalidTransformerArgs {
					name,
					expected,
					got,
				},
				line,
				column,
			}
		}
		ParseDiagnostic::UnmatchedClosingTag { name, line, column } => {
			ProjectDiagnostic {
				file: file.to_path_buf(),
				kind: DiagnosticKind::UnmatchedClosingTag { name },
				line,
				column,
			}
		}
		ParseDiagnostic::InvalidTag { tag, line, column } => {
			ProjectDiagnostic {
				file: file.to_path_buf(),
				kind: DiagnosticKind::InvalidTag { tag },
				line,
				column,
			}
		}
		ParseDiagnostic::NestedBlock {
			outer,
			inner,
			line,
			column,
		} => {
			ProjectDiagnostic {
				file: file.to_path_buf(),
				kind: DiagnosticKind::NestedBlock { outer, inner },
				line,
				column,
			}
		}
	}
}

fn parse_file_for_scan(
	file: &Path,
	options: &ScanOptions,
) -> MdtResult<index_cache::CachedFileData> {
	let read_error = |reason: String| {
		MdtError::ReadFile {
			path: file.display().to_string(),
			reason,
		}
	};
	let bytes = std::fs::read(file).map_err(|error| read_error(error.to_string()))?;
	// Every tag is an HTML comment, so a file without one has nothing to
	// parse. Checking the bytes first skips large tag-free docs cheaply and
	// never fails on tag-free files in other encodings (legacy C sources).
	if crate::lexer::memstr(&bytes, b"<!--").is_none() {
		return Ok(index_cache::CachedFileData {
			providers: Vec::new(),
			consumers: Vec::new(),
			diagnostics: Vec::new(),
		});
	}

	let raw_content = String::from_utf8(bytes).map_err(|error| read_error(error.to_string()))?;
	let content = normalize_line_endings(&raw_content);
	let (blocks, parse_diagnostics) = if is_markdown_file(file) {
		parse_with_diagnostics(&content)?
	} else {
		parse_source_with_diagnostics(&content, &options.markdown_codeblocks)?
	};

	let is_excluded = |name: &str| {
		options
			.excluded_blocks
			.iter()
			.any(|excluded| excluded == name)
	};
	let mut diagnostics: Vec<ProjectDiagnostic> = parse_diagnostics
		.into_iter()
		.map(|diag| parse_diagnostic_to_project(file, diag))
		.filter(|diagnostic| {
			// `[exclude] blocks` removes a block from every check, including
			// the structural ones.
			match &diagnostic.kind {
				DiagnosticKind::UnclosedBlock { name }
				| DiagnosticKind::UnmatchedClosingTag { name }
				| DiagnosticKind::ProviderOutsideTemplate { name } => !is_excluded(name),
				DiagnosticKind::NestedBlock { outer, inner } => {
					!is_excluded(outer) && !is_excluded(inner)
				}
				_ => true,
			}
		})
		.collect();
	let mut providers = Vec::with_capacity(blocks.len());
	let mut consumers = Vec::with_capacity(blocks.len());

	let is_template = is_template_file(file);

	for block in blocks.iter().filter(|block| !is_excluded(&block.name)) {
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
		if is_excluded(&block.name) {
			continue;
		}

		let block_content = extract_content_between_tags(&content, &block);

		match block.r#type {
			BlockType::Provider => {
				if !is_template {
					diagnostics.push(ProjectDiagnostic {
						file: file.to_path_buf(),
						kind: DiagnosticKind::ProviderOutsideTemplate {
							name: block.name.clone(),
						},
						line: block.opening.start.line,
						column: block.opening.start.column,
					});
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
	let mut consumers = Vec::with_capacity(files.len());
	let mut diagnostics = Vec::new();

	for file in files {
		let file_key = index_cache::relative_file_key(root, file);
		let Some(entry) = file_data.get(&file_key) else {
			continue;
		};

		diagnostics.extend(entry.diagnostics.iter().cloned());

		for provider in &entry.providers {
			if let Some(existing) = providers.get(&provider.block.name) {
				let location = |entry: &ProviderEntry| {
					format!(
						"{}:{}",
						relative_display_path(&entry.file, root),
						entry.block.opening.start.line
					)
				};
				return Err(MdtError::DuplicateProvider {
					name: provider.block.name.clone(),
					first_file: location(existing),
					second_file: location(provider),
				});
			}

			providers.insert(provider.block.name.clone(), provider.clone());
		}

		// Only the project's own files are consumers: a shared `*.t.md` read
		// through `[templates] paths` may contain consumers of its own, and
		// updating them would write outside the project.
		consumers.extend(
			entry
				.consumers
				.iter()
				.filter(|consumer| consumer.file.starts_with(root))
				.cloned(),
		);
	}

	let referenced_names: HashSet<&str> = consumers
		.iter()
		.filter(|consumer| consumer.block.r#type == BlockType::Consumer)
		.map(|consumer| consumer.block.name.as_str())
		.collect();

	for (name, entry) in &providers {
		// Providers shared from outside the project (a `[templates] paths`
		// entry such as `../../.templates`) are a library: each project uses
		// only some of them.
		let shared = !entry.file.starts_with(root);

		if !shared && !referenced_names.contains(name.as_str()) {
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
///
/// # Errors
///
/// Returns [`MdtError`] if I/O operations fail, file reading fails, or duplicate providers are found.
#[instrument(skip(options), fields(
	exclude_count = options.exclude_patterns.len(),
	template_paths = options.template_paths.len(),
	max_file_size = options.max_file_size,
	disable_gitignore = options.disable_gitignore,
))]
pub fn scan_project_with_options(root: &Path, options: &ScanOptions) -> MdtResult<Project> {
	let files = collect_project_files(root, options)?;

	debug!(files = files.len(), "collected files for scanning");

	let project_key = build_project_cache_key(root, options);
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

/// Collect every file the scan should parse: markdown and supported source
/// files, `*.t.md` files from `[templates] paths`, and files matching
/// `[include] patterns`. Each file appears once, even when reachable through
/// several symlinks.
fn collect_project_files(root: &Path, options: &ScanOptions) -> MdtResult<Vec<PathBuf>> {
	let exclude = build_exclude_matcher(root, &options.exclude_patterns)?;
	let use_gitignore = !options.disable_gitignore;
	let mut files = Vec::new();
	let mut saw_symlink = false;
	let mut walk = |dir: &Path, accept: &dyn Fn(&Path) -> bool, files: &mut Vec<PathBuf>| {
		let mut walker = ProjectWalker::new(root, &exclude, use_gitignore);
		walker.walk(dir, accept, files)?;
		saw_symlink |= walker.saw_symlink;
		MdtResult::Ok(())
	};

	walk(root, &is_scannable_file, &mut files)?;

	for template_dir in &options.template_paths {
		// Normalized so shared directories outside the project are
		// recognizably outside `root`.
		let dir = normalize_lexically(&root.join(template_dir));

		if !dir.is_dir() {
			return Err(MdtError::TemplatesPath {
				path: template_dir.display().to_string(),
			});
		}

		walk(&dir, &is_template_file, &mut files)?;
	}

	if !options.include_set.is_empty() {
		let is_included = |path: &Path| {
			path.strip_prefix(root)
				.is_ok_and(|relative| options.include_set.is_match(relative))
		};
		walk(root, &is_included, &mut files)?;
	}

	// Overlapping walks (`[templates] paths` inside the project, `[include]`)
	// find the same paths again. Only symlinks can make two different paths
	// name one file, and resolving every path is costly, so compare canonical
	// paths only when a walk met a symlink.
	files.sort();
	files.dedup();

	if saw_symlink {
		let mut canonical_files = HashSet::with_capacity(files.len());
		files.retain(|file| {
			canonical_files.insert(file.canonicalize().unwrap_or_else(|_| file.clone()))
		});
	}

	Ok(files)
}

/// Git ignore rules in effect during a walk, from outermost to innermost:
/// the repository's `.git/info/exclude`, the `.gitignore` files of the
/// project root and its ancestors up to the repository root, and the
/// `.gitignore` files of the directories the walk has descended into.
/// Outside a git repository only the root's own `.gitignore` applies.
struct IgnoreRules {
	enabled: bool,
	/// Whether nested `.gitignore` files apply: only inside a repository,
	/// as in git.
	nested: bool,
	stack: Vec<Gitignore>,
}

impl IgnoreRules {
	fn for_root(root: &Path, enabled: bool) -> Self {
		let mut rules = Self {
			enabled,
			nested: false,
			stack: Vec::new(),
		};

		if !enabled {
			return rules;
		}

		let repository_root = root.ancestors().find(|dir| dir.join(".git").exists());
		let Some(repository_root) = repository_root else {
			rules.push_file(root, &root.join(".gitignore"));

			return rules;
		};
		rules.nested = true;

		rules.push_file(repository_root, &repository_root.join(".git/info/exclude"));
		let ancestors: Vec<&Path> = root
			.ancestors()
			.take_while(|dir| dir.starts_with(repository_root))
			.collect();

		for dir in ancestors.into_iter().rev() {
			rules.enter(dir);
		}

		rules
	}

	/// Push the `.gitignore` of `dir`, if any. Returns whether a matcher was
	/// pushed so the caller can [`leave`](Self::leave) symmetrically.
	fn enter(&mut self, dir: &Path) -> bool {
		self.enabled && self.nested && self.push_file(dir, &dir.join(".gitignore"))
	}

	fn leave(&mut self, pushed: bool) {
		if pushed {
			self.stack.pop();
		}
	}

	fn push_file(&mut self, dir: &Path, file: &Path) -> bool {
		if !file.is_file() {
			return false;
		}

		let mut builder = GitignoreBuilder::new(dir);
		// A malformed line only disables that line, as in git.
		let _ = builder.add(file);

		match builder.build() {
			Ok(matcher) => {
				self.stack.push(matcher);
				true
			}
			Err(_) => false,
		}
	}

	/// Deeper rules win, and a `!` whitelist re-includes a path that an
	/// outer file ignores — the same precedence git uses.
	fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
		for matcher in self.stack.iter().rev() {
			match matcher.matched(path, is_dir) {
				ignore::Match::Ignore(_) => return true,
				ignore::Match::Whitelist(_) => return false,
				ignore::Match::None => {}
			}
		}

		false
	}
}

/// Walks a project tree with mdt's scanning rules: hidden directories
/// (except `.templates`), `node_modules`, and `target` are skipped; git
/// ignore rules and `[exclude]` patterns apply; directories with their own
/// mdt config are separate projects; and each directory is visited once,
/// even through symlink aliases or cycles.
struct ProjectWalker<'a> {
	exclude: &'a Gitignore,
	ignore_rules: IgnoreRules,
	visited_dirs: HashSet<PathBuf>,
	/// Whether any walked entry was a symlink, so the same file may have
	/// been collected under two paths.
	saw_symlink: bool,
}

impl<'a> ProjectWalker<'a> {
	fn new(root: &Path, exclude: &'a Gitignore, use_gitignore: bool) -> Self {
		Self {
			exclude,
			ignore_rules: IgnoreRules::for_root(root, use_gitignore),
			visited_dirs: HashSet::new(),
			saw_symlink: false,
		}
	}

	fn walk(
		&mut self,
		dir: &Path,
		accept: &dyn Fn(&Path) -> bool,
		files: &mut Vec<PathBuf>,
	) -> MdtResult<()> {
		if !dir.is_dir() || !first_visit(dir, &mut self.visited_dirs) {
			return Ok(());
		}

		for entry in std::fs::read_dir(dir)? {
			let entry = entry?;
			let path = entry.path();
			let skipped_name = path
				.file_name()
				.and_then(|name| name.to_str())
				.is_some_and(is_ignored_directory_name);

			if skipped_name {
				continue;
			}

			// The directory listing already knows whether an entry is a
			// symlink; `metadata` follows it, and fails for a dangling link.
			self.saw_symlink |= entry.file_type().is_ok_and(|kind| kind.is_symlink());
			let Ok(metadata) = std::fs::metadata(&path) else {
				continue;
			};
			let is_dir = metadata.is_dir();

			if self.ignore_rules.is_ignored(&path, is_dir)
				|| self.exclude.matched(&path, is_dir).is_ignore()
			{
				continue;
			}

			if is_dir {
				// A directory with its own mdt config is a separate project.
				if has_project_config(&path) {
					continue;
				}

				let pushed = self.ignore_rules.enter(&path);
				let walked = self.walk(&path, accept, files);
				self.ignore_rules.leave(pushed);
				walked?;
			} else if metadata.is_file() && accept(&path) {
				files.push(path);
			}
		}

		Ok(())
	}
}

fn is_ignored_directory_name(name: &str) -> bool {
	(name.starts_with('.') && name != ".templates") || name == "node_modules" || name == "target"
}

fn has_project_config(dir: &Path) -> bool {
	CONFIG_FILE_CANDIDATES
		.iter()
		.any(|candidate| dir.join(candidate).is_file())
}

/// Record `dir` as visited and report whether it still needs scanning.
///
/// Directories are tracked by canonical path, so a directory reached a second
/// time — through a symlink alias or a symlink cycle — is scanned only once.
fn first_visit(dir: &Path, visited_dirs: &mut HashSet<PathBuf>) -> bool {
	let canonical = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
	visited_dirs.insert(canonical)
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
			| "rs"
			| "ts"
			| "tsx"
			| "mts"
			| "cts"
			| "js"
			| "jsx"
			| "mjs"
			| "cjs"
			| "py"
			| "go"
			| "java"
			| "kt"
			| "swift"
			| "c"
			| "cc"
			| "cpp"
			| "cxx"
			| "h"
			| "hh"
			| "hpp"
			| "cs"
			| "dart"
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
#[instrument(skip(project), fields(
	providers = project.providers.len(),
	consumers = project.consumers.len(),
))]
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

	debug!(missing = missing.len(), "found missing providers");
	missing
}

/// Compute the Levenshtein edit distance between two strings.
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
	let a_len = a.len();
	let b_len = b.len();

	if a_len == 0 {
		return b_len;
	}

	if b_len == 0 {
		return a_len;
	}

	let mut prev_row: Vec<usize> = (0..=b_len).collect();
	let mut curr_row = vec![0; b_len + 1];

	for (i, a_char) in a.chars().enumerate() {
		curr_row[0] = i + 1;

		for (j, b_char) in b.chars().enumerate() {
			let cost = usize::from(a_char != b_char);
			curr_row[j + 1] = (prev_row[j + 1] + 1)
				.min(curr_row[j] + 1)
				.min(prev_row[j] + cost);
		}

		std::mem::swap(&mut prev_row, &mut curr_row);
	}

	prev_row[b_len]
}

/// Suggest up to three similar provider names for a missing consumer reference.
pub fn suggest_similar_provider_names<'a>(
	name: &str,
	provider_names: impl IntoIterator<Item = &'a str>,
) -> Vec<&'a str> {
	let max_distance = (name.len() / 2).max(2);
	let mut candidates: Vec<(&'a str, usize)> = provider_names
		.into_iter()
		.map(|provider_name| (provider_name, levenshtein_distance(name, provider_name)))
		.filter(|(_, distance)| *distance <= max_distance && *distance > 0)
		.collect();
	candidates.sort_by_key(|(_, distance)| *distance);
	candidates.truncate(3);
	candidates
		.into_iter()
		.map(|(provider_name, _)| provider_name)
		.collect()
}

/// Validate that all consumer blocks have matching providers.
#[instrument(skip(project), fields(
	providers = project.providers.len(),
	consumers = project.consumers.len(),
))]
pub fn validate_project(project: &Project) -> MdtResult<()> {
	let missing = find_missing_providers(project);

	if let Some(name) = missing.into_iter().next() {
		return Err(MdtError::MissingProvider(name));
	}

	debug!("project validation passed");
	Ok(())
}

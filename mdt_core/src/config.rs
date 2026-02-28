use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::time::UNIX_EPOCH;

use serde::Deserialize;
use serde::Serialize;

use crate::MdtError;
use crate::MdtResult;

/// Default maximum file size in bytes (10 MB).
pub const DEFAULT_MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Supported config file locations in discovery order (highest precedence
/// first).
pub const CONFIG_FILE_CANDIDATES: [&str; 3] = ["mdt.toml", ".mdt.toml", ".config/mdt.toml"];

/// Data source entry for a `[data]` namespace.
///
/// Backward-compatible string entries are supported:
///
/// ```toml
/// [data]
/// pkg = "package.json"
/// ```
///
/// Typed entries can provide an explicit format:
///
/// ```toml
/// [data]
/// release = { path = "release-info", format = "json" }
/// ```
///
/// Script-backed entries can execute commands and optionally cache output
/// based on watched files:
///
/// ```toml
/// [data]
/// version = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
/// ```
#[derive(Debug, Clone, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
#[non_exhaustive]
pub enum DataSource {
	Path(PathBuf),
	Typed(TypedDataSource),
	Script(ScriptDataSource),
}

impl DataSource {
	/// Returns the configured relative path for file-backed sources.
	pub fn path(&self) -> Option<&Path> {
		match self {
			Self::Path(path) => Some(path.as_path()),
			Self::Typed(typed) => Some(typed.path.as_path()),
			Self::Script(_) => None,
		}
	}

	/// Returns the explicit format override (if configured) for typed or script
	/// entries.
	pub fn format(&self) -> Option<&str> {
		match self {
			Self::Path(_) => None,
			Self::Typed(typed) => Some(typed.format.as_str()),
			Self::Script(script) => script.format.as_deref(),
		}
	}

	/// Returns the configured command for script-backed sources.
	pub fn command(&self) -> Option<&str> {
		match self {
			Self::Script(script) => Some(script.command.as_str()),
			_ => None,
		}
	}

	/// Returns watched files for script-backed sources.
	pub fn watch(&self) -> Option<&[PathBuf]> {
		match self {
			Self::Script(script) => Some(script.watch.as_slice()),
			_ => None,
		}
	}
}

/// Typed data source configuration for `[data]` entries.
#[derive(Debug, Clone, Deserialize, Eq, PartialEq)]
pub struct TypedDataSource {
	pub path: PathBuf,
	pub format: String,
}

/// Script-backed data source configuration for `[data]` entries.
#[derive(Debug, Clone, Deserialize, Eq, PartialEq)]
pub struct ScriptDataSource {
	pub command: String,
	#[serde(default)]
	pub format: Option<String>,
	#[serde(default)]
	pub watch: Vec<PathBuf>,
}

/// Configuration loaded from an `mdt.toml` file.
///
/// ```toml
/// [data]
/// package = "package.json"
/// cargo = "Cargo.toml"
///
/// [exclude]
/// patterns = ["vendor/", "generated/", "*.generated.md"]
/// markdown_codeblocks = true
/// blocks = ["internalOnly"]
///
/// [include]
/// patterns = ["docs/**/*.rs"]
///
/// [templates]
/// paths = ["shared/templates"]
///
/// [padding]
/// before = 1
/// after = 1
///
/// disable_gitignore = false
/// ```
#[derive(Debug, Deserialize)]
pub struct MdtConfig {
	/// Map of namespace name to relative file path for data sources.
	#[serde(default)]
	pub data: HashMap<String, DataSource>,
	/// Exclusion configuration using gitignore-style patterns.
	#[serde(default)]
	pub exclude: ExcludeConfig,
	/// Inclusion configuration — additional glob patterns to scan.
	#[serde(default)]
	pub include: IncludeConfig,
	/// Template paths — additional directories to search for `*.t.md` files.
	#[serde(default)]
	pub templates: TemplatesConfig,
	/// Maximum file size in bytes to scan. Files larger than this are skipped.
	/// Defaults to 10 MB.
	#[serde(default = "default_max_file_size")]
	pub max_file_size: u64,
	/// Padding configuration controlling blank lines between tags and content.
	/// When absent, no padding is applied. When present, `before` and `after`
	/// control how many blank lines separate tags from content.
	#[serde(default)]
	pub padding: Option<PaddingConfig>,
	/// When true, `.gitignore` files are not used for filtering. By default
	/// (`false`), mdt respects `.gitignore` patterns and skips files that
	/// would be ignored by git. Set to `true` when working outside a git
	/// repository or when you want full control over which files are
	/// scanned — in that case, use `[exclude]` patterns instead.
	#[serde(default)]
	pub disable_gitignore: bool,
}

/// Controls the number of blank lines between a tag and its content.
///
/// - `false` — Content appears inline with the tag (no newline separator).
/// - `0` — Content starts on the very next line (one newline, no blank lines).
/// - `1` — One blank line between the tag and content.
/// - `2` — Two blank lines, and so on.
///
/// When used in source code files with comment prefixes (e.g., `//!`, `///`),
/// blank lines include the comment prefix to maintain valid syntax.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
#[allow(variant_size_differences)]
pub enum PaddingValue {
	/// `false` disables padding (inline). `true` is treated as 1 blank line.
	Bool(bool),
	/// Number of blank lines (0 = next line, 1 = one blank line, etc.).
	Lines(u32),
}

impl PaddingValue {
	/// Returns the number of blank lines to add, or `None` if padding is
	/// disabled (`false`).
	pub fn line_count(&self) -> Option<u32> {
		match self {
			Self::Bool(false) => None,
			Self::Bool(true) => Some(1),
			Self::Lines(n) => Some(*n),
		}
	}
}

impl Default for PaddingValue {
	fn default() -> Self {
		Self::Lines(1)
	}
}

/// Configuration for padding between block tags and their content.
///
/// ```toml
/// [padding]
/// before = 1
/// after = 1
/// ```
///
/// When the `[padding]` section is present, `before` and `after` default to
/// `1` (one blank line). Set values to `0` for content on the next line with
/// no blank lines, or `false` for content inline with the tag.
#[derive(Debug, Clone, Deserialize)]
pub struct PaddingConfig {
	/// Blank lines between the opening tag and the content.
	#[serde(default)]
	pub before: PaddingValue,
	/// Blank lines between the content and the closing tag.
	#[serde(default)]
	pub after: PaddingValue,
}

fn default_max_file_size() -> u64 {
	DEFAULT_MAX_FILE_SIZE
}

const DATA_CACHE_SCHEMA_VERSION: u32 = 1;
const DATA_CACHE_FILE_NAME: &str = "data-v1.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DataScriptCache {
	schema_version: u32,
	entries: BTreeMap<String, ScriptCacheEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScriptCacheEntry {
	command: String,
	format: String,
	watch: Vec<String>,
	watch_fingerprints: BTreeMap<String, WatchFingerprint>,
	value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct WatchFingerprint {
	exists: bool,
	size: u64,
	modified_unix_ms: u64,
}

fn data_cache_path(root: &Path) -> PathBuf {
	root.join(".mdt").join("cache").join(DATA_CACHE_FILE_NAME)
}

fn load_script_cache(root: &Path) -> DataScriptCache {
	let cache_path = data_cache_path(root);
	let Ok(bytes) = std::fs::read(cache_path) else {
		return DataScriptCache {
			schema_version: DATA_CACHE_SCHEMA_VERSION,
			entries: BTreeMap::new(),
		};
	};

	let Ok(cache) = serde_json::from_slice::<DataScriptCache>(&bytes) else {
		return DataScriptCache {
			schema_version: DATA_CACHE_SCHEMA_VERSION,
			entries: BTreeMap::new(),
		};
	};

	if cache.schema_version != DATA_CACHE_SCHEMA_VERSION {
		return DataScriptCache {
			schema_version: DATA_CACHE_SCHEMA_VERSION,
			entries: BTreeMap::new(),
		};
	}

	cache
}

fn save_script_cache(root: &Path, cache: &DataScriptCache) {
	let cache_path = data_cache_path(root);
	let Some(cache_dir) = cache_path.parent() else {
		return;
	};

	if std::fs::create_dir_all(cache_dir).is_err() {
		return;
	}

	let Ok(payload) = serde_json::to_vec_pretty(cache) else {
		return;
	};

	let temp_path = cache_path.with_extension(format!(
		"json.tmp-{}-{}",
		std::process::id(),
		std::time::SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.map_or(0, |duration| duration.as_nanos())
	));

	if std::fs::write(&temp_path, payload).is_err() {
		return;
	}

	if std::fs::rename(&temp_path, &cache_path).is_err() {
		let _ = std::fs::remove_file(temp_path);
	}
}

fn normalize_path_key(path: &Path) -> String {
	path.to_string_lossy().replace('\\', "/")
}

fn watch_fingerprint(path: &Path) -> WatchFingerprint {
	match std::fs::metadata(path) {
		Ok(metadata) => WatchFingerprint {
			exists: true,
			size: metadata.len(),
			modified_unix_ms: metadata
				.modified()
				.ok()
				.and_then(|time| time.duration_since(UNIX_EPOCH).ok())
				.and_then(|duration| duration.as_millis().try_into().ok())
				.unwrap_or(0),
		},
		Err(_) => WatchFingerprint {
			exists: false,
			size: 0,
			modified_unix_ms: 0,
		},
	}
}

/// Controls filtering of mdt tags inside fenced code blocks in source files.
///
/// Can be:
/// - `false` (default): process tags in all code blocks normally.
/// - `true`: skip mdt tags inside **all** fenced code blocks.
/// - A string (e.g., `"ignore"`): skip tags in code blocks whose info string
///   contains the given string (e.g., `` ```rust,ignore ``).
/// - An array of strings: skip tags in code blocks whose info string contains
///   **any** of the given strings.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum CodeBlockFilter {
	/// `true` to skip all code blocks, `false` to process normally.
	Bool(bool),
	/// Skip code blocks whose info string contains any of these strings.
	InfoStrings(Vec<String>),
	/// Skip code blocks whose info string contains this string.
	InfoString(String),
}

impl Default for CodeBlockFilter {
	fn default() -> Self {
		Self::Bool(false)
	}
}

impl CodeBlockFilter {
	/// Returns `true` if code block filtering is enabled in any form.
	pub fn is_enabled(&self) -> bool {
		match self {
			Self::Bool(b) => *b,
			Self::InfoString(_) => true,
			Self::InfoStrings(v) => !v.is_empty(),
		}
	}

	/// Returns `true` if a code block with the given info string should have
	/// its mdt tags skipped.
	pub fn should_skip(&self, info_string: &str) -> bool {
		match self {
			Self::Bool(b) => *b,
			Self::InfoString(s) => info_string.contains(s.as_str()),
			Self::InfoStrings(v) => v.iter().any(|s| info_string.contains(s.as_str())),
		}
	}
}

/// Configuration for excluding files and directories from scanning.
///
/// Patterns follow gitignore syntax and are applied on top of any `.gitignore`
/// rules (unless `disable_gitignore` is set). Supports negation (`!pattern`),
/// directory markers (trailing `/`), and all standard gitignore wildcards.
#[derive(Debug, Default, Deserialize)]
pub struct ExcludeConfig {
	/// Gitignore-style patterns for files and directories to skip during
	/// scanning. These are relative to the project root.
	///
	/// Examples: `"build/"`, `"*.generated.md"`, `"!important.md"`.
	#[serde(default)]
	pub patterns: Vec<String>,
	/// Controls whether mdt tags inside fenced code blocks are processed.
	///
	/// - `false` (default): tags in code blocks are processed normally.
	/// - `true`: tags inside **all** fenced code blocks are skipped.
	/// - A string (e.g., `"ignore"`): tags in code blocks whose info string
	///   contains the given string are skipped.
	/// - An array of strings: tags in code blocks whose info string contains
	///   **any** of the given strings are skipped.
	#[serde(default)]
	pub markdown_codeblocks: CodeBlockFilter,
	/// Block names to exclude from processing. Any provider or consumer
	/// block whose name appears in this list is completely ignored — it
	/// won't be scanned, matched, or updated.
	#[serde(default)]
	pub blocks: Vec<String>,
}

/// Configuration for including additional files in scanning.
#[derive(Debug, Default, Deserialize)]
pub struct IncludeConfig {
	/// Additional glob patterns for files to scan.
	/// These are relative to the project root.
	#[serde(default)]
	pub patterns: Vec<String>,
}

/// Configuration for additional template search paths.
#[derive(Debug, Default, Deserialize)]
pub struct TemplatesConfig {
	/// Additional directories to search for `*.t.md` template files.
	/// These are relative to the project root.
	#[serde(default)]
	pub paths: Vec<PathBuf>,
}

impl MdtConfig {
	/// Resolve the config path from known discovery candidates.
	#[must_use]
	pub fn resolve_path(root: &Path) -> Option<PathBuf> {
		CONFIG_FILE_CANDIDATES
			.iter()
			.map(|candidate| root.join(candidate))
			.find(|path| path.is_file())
	}

	/// Load the config from the first discovered config file at `root`.
	/// Returns `None` if the file does not exist.
	pub fn load(root: &Path) -> MdtResult<Option<MdtConfig>> {
		let Some(config_path) = Self::resolve_path(root) else {
			return Ok(None);
		};

		let content = std::fs::read_to_string(&config_path)?;
		let config: MdtConfig =
			toml::from_str(&content).map_err(|e| MdtError::ConfigParse(e.to_string()))?;

		Ok(Some(config))
	}

	/// Read each data file and parse it into a `serde_json::Value` keyed by
	/// namespace.
	pub fn load_data(&self, root: &Path) -> MdtResult<HashMap<String, serde_json::Value>> {
		let mut data = HashMap::new();
		let mut script_cache = load_script_cache(root);
		script_cache.schema_version = DATA_CACHE_SCHEMA_VERSION;
		let mut touched_script_cache = false;

		let mut namespaces: Vec<_> = self.data.keys().cloned().collect();
		namespaces.sort();

		for namespace in namespaces {
			let source = self
				.data
				.get(&namespace)
				.unwrap_or_else(|| panic!("missing namespace `{namespace}`"));
			let value = match source {
				DataSource::Path(rel_path) => {
					let abs_path = root.join(rel_path);
					let content =
						std::fs::read_to_string(&abs_path).map_err(|e| MdtError::DataFile {
							path: rel_path.display().to_string(),
							reason: e.to_string(),
						})?;
					let format = abs_path
						.extension()
						.and_then(|e| e.to_str())
						.unwrap_or("")
						.to_ascii_lowercase();
					parse_data_file(&content, format.as_str(), &rel_path.display().to_string())?
				}
				DataSource::Typed(typed) => {
					let rel_path = typed.path.as_path();
					let abs_path = root.join(rel_path);
					let content =
						std::fs::read_to_string(&abs_path).map_err(|e| MdtError::DataFile {
							path: rel_path.display().to_string(),
							reason: e.to_string(),
						})?;
					let format = typed.format.trim().to_ascii_lowercase();
					parse_data_file(&content, format.as_str(), &rel_path.display().to_string())?
				}
				DataSource::Script(script) => {
					touched_script_cache = true;
					load_script_data_source(root, &namespace, script, &mut script_cache)?
				}
			};

			data.insert(namespace, value);
		}

		if touched_script_cache {
			save_script_cache(root, &script_cache);
		}

		Ok(data)
	}
}

fn load_script_data_source(
	root: &Path,
	namespace: &str,
	script: &ScriptDataSource,
	cache: &mut DataScriptCache,
) -> MdtResult<serde_json::Value> {
	let format = script
		.format
		.as_deref()
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(str::to_ascii_lowercase)
		.unwrap_or_else(|| "text".to_string());

	let mut watch: Vec<String> = script
		.watch
		.iter()
		.map(|path| normalize_path_key(path))
		.collect();
	watch.sort();
	watch.dedup();

	let watch_fingerprints: BTreeMap<String, WatchFingerprint> = watch
		.iter()
		.map(|watch_path| {
			let abs_watch = root.join(watch_path);
			(watch_path.clone(), watch_fingerprint(&abs_watch))
		})
		.collect();

	// Only use cache when explicit watch files are configured.
	if !watch.is_empty() {
		if let Some(cached) = cache.entries.get(namespace) {
			if cached.command == script.command
				&& cached.format == format
				&& cached.watch == watch
				&& cached.watch_fingerprints == watch_fingerprints
			{
				return Ok(cached.value.clone());
			}
		}
	}

	let stdout = execute_script(root, namespace, &script.command)?;
	let value = parse_data_file(&stdout, &format, namespace)?;

	cache.entries.insert(
		namespace.to_string(),
		ScriptCacheEntry {
			command: script.command.clone(),
			format,
			watch,
			watch_fingerprints,
			value: value.clone(),
		},
	);

	Ok(value)
}

fn execute_script(root: &Path, namespace: &str, command: &str) -> MdtResult<String> {
	let output = if cfg!(windows) {
		Command::new("cmd")
			.arg("/C")
			.arg(command)
			.current_dir(root)
			.output()?
	} else {
		Command::new("sh")
			.arg("-c")
			.arg(command)
			.current_dir(root)
			.output()?
	};

	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
		let reason = if stderr.is_empty() {
			format!(
				"command exited with status {}",
				output
					.status
					.code()
					.map_or_else(|| "unknown".to_string(), |code| code.to_string())
			)
		} else {
			stderr
		};

		return Err(MdtError::DataScript {
			namespace: namespace.to_string(),
			reason,
		});
	}

	Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Parse a data file's content into a `serde_json::Value` based on its
/// format.
fn parse_data_file(
	content: &str,
	format: &str,
	path_display: &str,
) -> MdtResult<serde_json::Value> {
	match format {
		"text" | "string" | "raw" | "txt" => Ok(serde_json::Value::String(content.to_string())),
		"json" => serde_json::from_str(content).map_err(|e| MdtError::DataFile {
			path: path_display.to_string(),
			reason: e.to_string(),
		}),
		"toml" => {
			let toml_value: toml::Value =
				toml::from_str(content).map_err(|e| MdtError::DataFile {
					path: path_display.to_string(),
					reason: e.to_string(),
				})?;
			toml_to_json(toml_value, path_display)
		}
		"yaml" | "yml" => serde_yaml_ng::from_str(content).map_err(|e| MdtError::DataFile {
			path: path_display.to_string(),
			reason: e.to_string(),
		}),
		"kdl" => {
			let doc: kdl::KdlDocument =
				content
					.parse()
					.map_err(|e: kdl::KdlError| MdtError::DataFile {
						path: path_display.to_string(),
						reason: e.to_string(),
					})?;
			kdl_document_to_value(&doc, path_display)
		}
		"ini" => serde_ini::from_str(content).map_err(|e| MdtError::DataFile {
			path: path_display.to_string(),
			reason: e.to_string(),
		}),
		other => Err(MdtError::UnsupportedDataFormat(other.to_string())),
	}
}

/// Convert a `toml::Value` to a `serde_json::Value`.
fn toml_to_json(value: toml::Value, path_display: &str) -> MdtResult<serde_json::Value> {
	let json = match value {
		toml::Value::String(s) => serde_json::Value::String(s),
		toml::Value::Integer(i) => {
			serde_json::Value::Number(serde_json::Number::from_f64(i as f64).ok_or_else(|| {
				MdtError::UnconvertibleFloat {
					path: path_display.to_string(),
					value: i.to_string(),
				}
			})?)
		}
		toml::Value::Float(f) => {
			serde_json::Value::Number(serde_json::Number::from_f64(f).ok_or_else(|| {
				MdtError::UnconvertibleFloat {
					path: path_display.to_string(),
					value: f.to_string(),
				}
			})?)
		}
		toml::Value::Boolean(b) => serde_json::Value::Bool(b),
		toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
		toml::Value::Array(arr) => {
			let items: MdtResult<Vec<serde_json::Value>> = arr
				.into_iter()
				.map(|v| toml_to_json(v, path_display))
				.collect();
			serde_json::Value::Array(items?)
		}
		toml::Value::Table(table) => {
			let mut map = serde_json::Map::new();
			for (k, v) in table {
				map.insert(k, toml_to_json(v, path_display)?);
			}
			serde_json::Value::Object(map)
		}
	};

	Ok(json)
}

/// Convert a KDL document to a `serde_json::Value`.
fn kdl_document_to_value(
	doc: &kdl::KdlDocument,
	path_display: &str,
) -> MdtResult<serde_json::Value> {
	let mut map = serde_json::Map::new();

	for node in doc.nodes() {
		let name = node.name().to_string();
		let value = kdl_node_to_value(node, path_display)?;
		map.insert(name, value);
	}

	Ok(serde_json::Value::Object(map))
}

/// Convert a KDL node to a `serde_json::Value`.
fn kdl_node_to_value(node: &kdl::KdlNode, path_display: &str) -> MdtResult<serde_json::Value> {
	// If the node has children, treat it as an object
	if let Some(children) = node.children() {
		return kdl_document_to_value(children, path_display);
	}

	// If the node has entries, collect them
	let entries: Vec<&kdl::KdlEntry> = node.entries().iter().collect();

	if entries.is_empty() {
		return Ok(serde_json::Value::Null);
	}

	// If there's exactly one positional argument with no name, return it directly
	if entries.len() == 1 && entries[0].name().is_none() {
		return kdl_entry_value_to_json(entries[0].value(), path_display);
	}

	// If all entries are named, create an object
	let all_named = entries.iter().all(|e| e.name().is_some());
	if all_named {
		let mut map = serde_json::Map::new();
		for entry in &entries {
			if let Some(name) = entry.name() {
				map.insert(
					name.to_string(),
					kdl_entry_value_to_json(entry.value(), path_display)?,
				);
			}
		}
		return Ok(serde_json::Value::Object(map));
	}

	// Mixed or multiple positional: return an array
	let values: MdtResult<Vec<serde_json::Value>> = entries
		.iter()
		.map(|e| kdl_entry_value_to_json(e.value(), path_display))
		.collect();
	Ok(serde_json::Value::Array(values?))
}

/// Convert a KDL entry value to a `serde_json::Value`.
fn kdl_entry_value_to_json(
	value: &kdl::KdlValue,
	path_display: &str,
) -> MdtResult<serde_json::Value> {
	match value {
		kdl::KdlValue::String(s) => Ok(serde_json::Value::String(s.clone())),
		kdl::KdlValue::Integer(i) => Ok(serde_json::Value::Number(
			serde_json::Number::from_f64(*i as f64).ok_or_else(|| {
				MdtError::UnconvertibleFloat {
					path: path_display.to_string(),
					value: i.to_string(),
				}
			})?,
		)),
		kdl::KdlValue::Float(f) => Ok(serde_json::Value::Number(
			serde_json::Number::from_f64(*f).ok_or_else(|| MdtError::UnconvertibleFloat {
				path: path_display.to_string(),
				value: f.to_string(),
			})?,
		)),
		kdl::KdlValue::Bool(b) => Ok(serde_json::Value::Bool(*b)),
		kdl::KdlValue::Null => Ok(serde_json::Value::Null),
	}
}

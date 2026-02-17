use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;

use crate::MdtError;
use crate::MdtResult;

/// Configuration loaded from an `mdt.toml` file.
///
/// ```toml
/// [data]
/// package = "package.json"
/// cargo = "Cargo.toml"
///
/// [exclude]
/// patterns = ["vendor/**", "generated/**"]
///
/// [include]
/// patterns = ["docs/**/*.rs"]
///
/// [templates]
/// paths = ["shared/templates"]
/// ```
#[derive(Debug, Deserialize)]
pub struct MdtConfig {
	/// Map of namespace name to relative file path for data sources.
	#[serde(default)]
	pub data: HashMap<String, PathBuf>,
	/// Exclusion configuration.
	#[serde(default)]
	pub exclude: ExcludeConfig,
	/// Inclusion configuration — additional glob patterns to scan.
	#[serde(default)]
	pub include: IncludeConfig,
	/// Template paths — additional directories to search for `*.t.md` files.
	#[serde(default)]
	pub templates: TemplatesConfig,
}

/// Configuration for excluding files and directories from scanning.
#[derive(Debug, Default, Deserialize)]
pub struct ExcludeConfig {
	/// Glob patterns for directories or files to skip during scanning.
	/// These are relative to the project root.
	#[serde(default)]
	pub patterns: Vec<String>,
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
	/// Load the config from `mdt.toml` at the given root directory.
	/// Returns `None` if the file does not exist.
	pub fn load(root: &Path) -> MdtResult<Option<MdtConfig>> {
		let config_path = root.join("mdt.toml");

		if !config_path.exists() {
			return Ok(None);
		}

		let content = std::fs::read_to_string(&config_path)?;
		let config: MdtConfig =
			toml::from_str(&content).map_err(|e| MdtError::ConfigParse(e.to_string()))?;

		Ok(Some(config))
	}

	/// Read each data file and parse it into a `serde_json::Value` keyed by
	/// namespace.
	pub fn load_data(&self, root: &Path) -> MdtResult<HashMap<String, serde_json::Value>> {
		let mut data = HashMap::new();

		for (namespace, rel_path) in &self.data {
			let abs_path = root.join(rel_path);
			let content = std::fs::read_to_string(&abs_path).map_err(|e| {
				MdtError::DataFile {
					path: rel_path.display().to_string(),
					reason: e.to_string(),
				}
			})?;

			let ext = abs_path.extension().and_then(|e| e.to_str()).unwrap_or("");

			let value = parse_data_file(&content, ext, &rel_path.display().to_string())?;
			data.insert(namespace.clone(), value);
		}

		Ok(data)
	}
}

/// Parse a data file's content into a `serde_json::Value` based on its
/// extension.
fn parse_data_file(
	content: &str,
	extension: &str,
	path_display: &str,
) -> MdtResult<serde_json::Value> {
	match extension {
		"json" => {
			serde_json::from_str(content).map_err(|e| {
				MdtError::DataFile {
					path: path_display.to_string(),
					reason: e.to_string(),
				}
			})
		}
		"toml" => {
			let toml_value: toml::Value = toml::from_str(content).map_err(|e| {
				MdtError::DataFile {
					path: path_display.to_string(),
					reason: e.to_string(),
				}
			})?;
			toml_to_json(toml_value)
		}
		"yaml" | "yml" => {
			serde_yml::from_str(content).map_err(|e| {
				MdtError::DataFile {
					path: path_display.to_string(),
					reason: e.to_string(),
				}
			})
		}
		"kdl" => {
			let doc: kdl::KdlDocument = content.parse().map_err(|e: kdl::KdlError| {
				MdtError::DataFile {
					path: path_display.to_string(),
					reason: e.to_string(),
				}
			})?;
			Ok(kdl_document_to_value(&doc))
		}
		other => Err(MdtError::UnsupportedDataFormat(other.to_string())),
	}
}

/// Convert a `toml::Value` to a `serde_json::Value`.
fn toml_to_json(value: toml::Value) -> MdtResult<serde_json::Value> {
	let json = match value {
		toml::Value::String(s) => serde_json::Value::String(s),
		toml::Value::Integer(i) => {
			serde_json::Value::Number(
				serde_json::Number::from_f64(i as f64)
					.unwrap_or_else(|| serde_json::Number::from(0)),
			)
		}
		toml::Value::Float(f) => {
			serde_json::Value::Number(
				serde_json::Number::from_f64(f).unwrap_or_else(|| serde_json::Number::from(0)),
			)
		}
		toml::Value::Boolean(b) => serde_json::Value::Bool(b),
		toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
		toml::Value::Array(arr) => {
			let items: MdtResult<Vec<serde_json::Value>> =
				arr.into_iter().map(toml_to_json).collect();
			serde_json::Value::Array(items?)
		}
		toml::Value::Table(table) => {
			let mut map = serde_json::Map::new();
			for (k, v) in table {
				map.insert(k, toml_to_json(v)?);
			}
			serde_json::Value::Object(map)
		}
	};

	Ok(json)
}

/// Convert a KDL document to a `serde_json::Value`.
fn kdl_document_to_value(doc: &kdl::KdlDocument) -> serde_json::Value {
	let mut map = serde_json::Map::new();

	for node in doc.nodes() {
		let name = node.name().to_string();
		let value = kdl_node_to_value(node);
		map.insert(name, value);
	}

	serde_json::Value::Object(map)
}

/// Convert a KDL node to a `serde_json::Value`.
fn kdl_node_to_value(node: &kdl::KdlNode) -> serde_json::Value {
	// If the node has children, treat it as an object
	if let Some(children) = node.children() {
		return kdl_document_to_value(children);
	}

	// If the node has entries, collect them
	let entries: Vec<&kdl::KdlEntry> = node.entries().iter().collect();

	if entries.is_empty() {
		return serde_json::Value::Null;
	}

	// If there's exactly one positional argument with no name, return it directly
	if entries.len() == 1 && entries[0].name().is_none() {
		return kdl_entry_value_to_json(entries[0].value());
	}

	// If all entries are named, create an object
	let all_named = entries.iter().all(|e| e.name().is_some());
	if all_named {
		let mut map = serde_json::Map::new();
		for entry in &entries {
			if let Some(name) = entry.name() {
				map.insert(name.to_string(), kdl_entry_value_to_json(entry.value()));
			}
		}
		return serde_json::Value::Object(map);
	}

	// Mixed or multiple positional: return an array
	let values: Vec<serde_json::Value> = entries
		.iter()
		.map(|e| kdl_entry_value_to_json(e.value()))
		.collect();
	serde_json::Value::Array(values)
}

/// Convert a KDL entry value to a `serde_json::Value`.
fn kdl_entry_value_to_json(value: &kdl::KdlValue) -> serde_json::Value {
	match value {
		kdl::KdlValue::String(s) => serde_json::Value::String(s.clone()),
		kdl::KdlValue::Integer(i) => {
			serde_json::Value::Number(
				serde_json::Number::from_f64(*i as f64)
					.unwrap_or_else(|| serde_json::Number::from(0)),
			)
		}
		kdl::KdlValue::Float(f) => {
			serde_json::Value::Number(
				serde_json::Number::from_f64(*f).unwrap_or_else(|| serde_json::Number::from(0)),
			)
		}
		kdl::KdlValue::Bool(b) => serde_json::Value::Bool(*b),
		kdl::KdlValue::Null => serde_json::Value::Null,
	}
}

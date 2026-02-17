use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use globset::Glob;
use globset::GlobSet;
use globset::GlobSetBuilder;

use crate::Block;
use crate::BlockType;
use crate::MdtError;
use crate::MdtResult;
use crate::config::MdtConfig;
use crate::parser::parse;
use crate::source_scanner::parse_source;

/// A scanned project containing all discovered blocks.
#[derive(Debug)]
pub struct Project {
	/// Provider blocks keyed by block name. Each value is the provider block
	/// and the file path it was found in.
	pub providers: HashMap<String, ProviderEntry>,
	/// Consumer blocks grouped by file path.
	pub consumers: Vec<ConsumerEntry>,
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
	scan_project_with_excludes(root, &GlobSet::empty())
}

/// Scan a project with config — loads `mdt.toml`, reads data files, and scans.
pub fn scan_project_with_config(
	root: &Path,
) -> MdtResult<(Project, HashMap<String, serde_json::Value>)> {
	let config = MdtConfig::load(root)?;
	let exclude_patterns = config
		.as_ref()
		.map(|c| &c.exclude.patterns[..])
		.unwrap_or_default();
	let exclude_set = build_glob_set(exclude_patterns);
	let project = scan_project_with_excludes(root, &exclude_set)?;
	let data = match config {
		Some(config) => config.load_data(root)?,
		None => HashMap::new(),
	};

	Ok((project, data))
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

/// Scan a directory with exclude patterns applied.
fn scan_project_with_excludes(root: &Path, exclude_set: &GlobSet) -> MdtResult<Project> {
	let mut providers = HashMap::new();
	let mut consumers = Vec::new();

	let files = collect_files(root, exclude_set)?;

	for file in &files {
		let content = std::fs::read_to_string(file)?;
		let blocks = if is_markdown_file(file) {
			parse(&content)?
		} else {
			parse_source(&content)?
		};
		let is_template = file
			.file_name()
			.and_then(|name| name.to_str())
			.is_some_and(|name| name.ends_with(".t.md"));

		for block in blocks {
			let block_content = extract_content_between_tags(&content, &block);

			match block.r#type {
				BlockType::Provider => {
					if !is_template {
						continue;
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

	Ok(Project {
		providers,
		consumers,
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
	walk_dir(root, root, &mut files, true, exclude_set)?;
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
) -> MdtResult<()> {
	if !dir.is_dir() {
		return Ok(());
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
			walk_dir(root, &path, files, false, exclude_set)?;
		} else if is_scannable_file(&path) {
			files.push(path);
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

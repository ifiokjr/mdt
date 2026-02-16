use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use crate::Block;
use crate::BlockType;
use crate::MdtError;
use crate::MdtResult;
use crate::parser::parse;

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
	let mut providers = HashMap::new();
	let mut consumers = Vec::new();

	let files = collect_files(root)?;

	for file in &files {
		let content = std::fs::read_to_string(file)?;
		let blocks = parse(&content)?;
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
fn collect_files(root: &Path) -> MdtResult<Vec<PathBuf>> {
	let mut files = Vec::new();
	walk_dir(root, &mut files)?;
	// Sort for deterministic ordering
	files.sort();
	Ok(files)
}

fn walk_dir(dir: &Path, files: &mut Vec<PathBuf>) -> MdtResult<()> {
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

		if path.is_dir() {
			walk_dir(&path, files)?;
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

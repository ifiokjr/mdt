use std::collections::HashMap;
use std::path::PathBuf;

use crate::Argument;
use crate::MdtError;
use crate::MdtResult;
use crate::Transformer;
use crate::TransformerType;
use crate::project::ConsumerEntry;
use crate::project::Project;

/// Result of checking a project for stale consumers.
#[derive(Debug)]
pub struct CheckResult {
	/// Consumer entries that are out of date.
	pub stale: Vec<StaleEntry>,
}

impl CheckResult {
	/// Returns true if all consumers are up to date.
	pub fn is_ok(&self) -> bool {
		self.stale.is_empty()
	}
}

/// A consumer entry that is out of date.
#[derive(Debug)]
pub struct StaleEntry {
	pub file: PathBuf,
	pub block_name: String,
	pub current_content: String,
	pub expected_content: String,
}

/// Result of updating a project.
#[derive(Debug)]
pub struct UpdateResult {
	/// Files that were modified and their new content.
	pub updated_files: HashMap<PathBuf, String>,
	/// Number of consumer blocks that were updated.
	pub updated_count: usize,
}

/// Render provider content through minijinja using the given data context.
/// If data is empty or the content has no template syntax, returns the
/// content unchanged.
#[allow(clippy::implicit_hasher)]
pub fn render_template(
	content: &str,
	data: &HashMap<String, serde_json::Value>,
) -> MdtResult<String> {
	if data.is_empty() || !has_template_syntax(content) {
		return Ok(content.to_string());
	}

	let mut env = minijinja::Environment::new();
	env.set_undefined_behavior(minijinja::UndefinedBehavior::Chainable);
	env.add_template("__inline__", content)
		.map_err(|e| MdtError::TemplateRender(e.to_string()))?;

	let template = env
		.get_template("__inline__")
		.map_err(|e| MdtError::TemplateRender(e.to_string()))?;

	let ctx = minijinja::Value::from_serialize(data);
	template
		.render(ctx)
		.map_err(|e| MdtError::TemplateRender(e.to_string()))
}

/// Check whether content contains minijinja template syntax.
fn has_template_syntax(content: &str) -> bool {
	content.contains("{{") || content.contains("{%") || content.contains("{#")
}

/// Check whether all consumer blocks in the project are up to date.
/// Consumer blocks that reference non-existent providers are silently skipped.
#[allow(clippy::implicit_hasher)]
pub fn check_project(
	project: &Project,
	data: &HashMap<String, serde_json::Value>,
) -> MdtResult<CheckResult> {
	let mut stale = Vec::new();

	for consumer in &project.consumers {
		let Some(provider) = project.providers.get(&consumer.block.name) else {
			continue;
		};

		let rendered = render_template(&provider.content, data)?;
		let expected = apply_transformers(&rendered, &consumer.block.transformers);

		if consumer.content != expected {
			stale.push(StaleEntry {
				file: consumer.file.clone(),
				block_name: consumer.block.name.clone(),
				current_content: consumer.content.clone(),
				expected_content: expected,
			});
		}
	}

	Ok(CheckResult { stale })
}

/// Compute the updated file contents for all consumer blocks.
#[allow(clippy::implicit_hasher)]
pub fn compute_updates(
	project: &Project,
	data: &HashMap<String, serde_json::Value>,
) -> MdtResult<UpdateResult> {
	let mut file_contents: HashMap<PathBuf, String> = HashMap::new();
	let mut updated_count = 0;

	// Group consumers by file
	let mut consumers_by_file: HashMap<PathBuf, Vec<&ConsumerEntry>> = HashMap::new();
	for consumer in &project.consumers {
		consumers_by_file
			.entry(consumer.file.clone())
			.or_default()
			.push(consumer);
	}

	for (file, consumers) in &consumers_by_file {
		let original = if let Some(content) = file_contents.get(file) {
			content.clone()
		} else {
			std::fs::read_to_string(file)?
		};

		let mut result = original.clone();
		// Process consumers in reverse offset order so earlier replacements
		// don't shift the positions of later ones.
		let mut sorted_consumers: Vec<&&ConsumerEntry> = consumers.iter().collect();
		sorted_consumers
			.sort_by(|a, b| b.block.opening.end.offset.cmp(&a.block.opening.end.offset));

		for consumer in sorted_consumers {
			let Some(provider) = project.providers.get(&consumer.block.name) else {
				continue;
			};

			let rendered = render_template(&provider.content, data)?;
			let new_content = apply_transformers(&rendered, &consumer.block.transformers);

			if consumer.content != new_content {
				let start = consumer.block.opening.end.offset;
				let end = consumer.block.closing.start.offset;

				if start <= end && end <= result.len() {
					result = format!("{}{}{}", &result[..start], new_content, &result[end..]);
					updated_count += 1;
				}
			}
		}

		if result != original {
			file_contents.insert(file.clone(), result);
		}
	}

	Ok(UpdateResult {
		updated_files: file_contents,
		updated_count,
	})
}

/// Write the updated contents back to disk.
pub fn write_updates(updates: &UpdateResult) -> MdtResult<()> {
	for (path, content) in &updates.updated_files {
		std::fs::write(path, content)?;
	}
	Ok(())
}

/// Apply a sequence of transformers to content.
pub fn apply_transformers(content: &str, transformers: &[Transformer]) -> String {
	let mut result = content.to_string();

	for transformer in transformers {
		result = apply_transformer(&result, transformer);
	}

	result
}

fn apply_transformer(content: &str, transformer: &Transformer) -> String {
	match transformer.r#type {
		TransformerType::Trim => content.trim().to_string(),
		TransformerType::TrimStart => content.trim_start().to_string(),
		TransformerType::TrimEnd => content.trim_end().to_string(),
		TransformerType::Indent => {
			let indent_str = get_string_arg(&transformer.args, 0).unwrap_or_default();
			content
				.lines()
				.map(|line| {
					if line.is_empty() {
						String::new()
					} else {
						format!("{indent_str}{line}")
					}
				})
				.collect::<Vec<_>>()
				.join("\n")
		}
		TransformerType::Prefix => {
			let prefix = get_string_arg(&transformer.args, 0).unwrap_or_default();
			format!("{prefix}{content}")
		}
		TransformerType::Wrap => {
			let wrapper = get_string_arg(&transformer.args, 0).unwrap_or_default();
			format!("{wrapper}{content}{wrapper}")
		}
		TransformerType::CodeBlock => {
			let lang = get_string_arg(&transformer.args, 0).unwrap_or_default();
			format!("```{lang}\n{content}\n```")
		}
		TransformerType::Code => {
			format!("`{content}`")
		}
		TransformerType::Replace => {
			let search = get_string_arg(&transformer.args, 0).unwrap_or_default();
			let replacement = get_string_arg(&transformer.args, 1).unwrap_or_default();
			content.replace(&search, &replacement)
		}
	}
}

fn get_string_arg(args: &[Argument], index: usize) -> Option<String> {
	args.get(index).map(|arg| {
		match arg {
			Argument::String(s) => s.clone(),
			Argument::Number(n) => n.to_string(),
			Argument::Boolean(b) => b.to_string(),
		}
	})
}

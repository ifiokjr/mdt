use std::collections::HashMap;
use std::path::PathBuf;

use crate::Argument;
use crate::MdtError;
use crate::MdtResult;
use crate::Transformer;
use crate::TransformerType;
use crate::project::ConsumerEntry;
use crate::project::ProjectContext;

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
	/// Path to the file containing the stale consumer.
	pub file: PathBuf,
	/// Name of the block that is out of date.
	pub block_name: String,
	/// The current content between the consumer's tags.
	pub current_content: String,
	/// The expected content after applying provider content and transformers.
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
pub fn check_project(ctx: &ProjectContext) -> MdtResult<CheckResult> {
	let mut stale = Vec::new();

	for consumer in &ctx.project.consumers {
		let Some(provider) = ctx.project.providers.get(&consumer.block.name) else {
			continue;
		};

		let rendered = render_template(&provider.content, &ctx.data)?;
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
pub fn compute_updates(ctx: &ProjectContext) -> MdtResult<UpdateResult> {
	let mut file_contents: HashMap<PathBuf, String> = HashMap::new();
	let mut updated_count = 0;

	// Group consumers by file
	let mut consumers_by_file: HashMap<PathBuf, Vec<&ConsumerEntry>> = HashMap::new();
	for consumer in &ctx.project.consumers {
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
		let mut had_update = false;
		// Process consumers in reverse offset order so earlier replacements
		// don't shift the positions of later ones.
		let mut sorted_consumers: Vec<&&ConsumerEntry> = consumers.iter().collect();
		sorted_consumers
			.sort_by(|a, b| b.block.opening.end.offset.cmp(&a.block.opening.end.offset));

		for consumer in sorted_consumers {
			let Some(provider) = ctx.project.providers.get(&consumer.block.name) else {
				continue;
			};

			let rendered = render_template(&provider.content, &ctx.data)?;
			let new_content = apply_transformers(&rendered, &consumer.block.transformers);

			if consumer.content != new_content {
				let start = consumer.block.opening.end.offset;
				let end = consumer.block.closing.start.offset;

				if start <= end && end <= result.len() {
					let mut buf =
						String::with_capacity(result.len() - (end - start) + new_content.len());
					buf.push_str(&result[..start]);
					buf.push_str(&new_content);
					buf.push_str(&result[end..]);
					result = buf;
					had_update = true;
					updated_count += 1;
				}
			}
		}

		if had_update {
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
			let include_empty = get_bool_arg(&transformer.args, 1).unwrap_or(false);
			content
				.lines()
				.map(|line| {
					if line.is_empty() && !include_empty {
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
		TransformerType::Suffix => {
			let suffix = get_string_arg(&transformer.args, 0).unwrap_or_default();
			format!("{content}{suffix}")
		}
		TransformerType::LinePrefix => {
			let prefix = get_string_arg(&transformer.args, 0).unwrap_or_default();
			let include_empty = get_bool_arg(&transformer.args, 1).unwrap_or(false);
			content
				.lines()
				.map(|line| {
					if line.is_empty() && !include_empty {
						String::new()
					} else {
						format!("{prefix}{line}")
					}
				})
				.collect::<Vec<_>>()
				.join("\n")
		}
		TransformerType::LineSuffix => {
			let suffix = get_string_arg(&transformer.args, 0).unwrap_or_default();
			let include_empty = get_bool_arg(&transformer.args, 1).unwrap_or(false);
			content
				.lines()
				.map(|line| {
					if line.is_empty() && !include_empty {
						String::new()
					} else {
						format!("{line}{suffix}")
					}
				})
				.collect::<Vec<_>>()
				.join("\n")
		}
	}
}

/// Validate that all transformer arguments are well-formed. Returns an error
/// for the first invalid transformer found.
pub fn validate_transformers(transformers: &[Transformer]) -> MdtResult<()> {
	for t in transformers {
		let (min, max) = match t.r#type {
			TransformerType::Trim
			| TransformerType::TrimStart
			| TransformerType::TrimEnd
			| TransformerType::Code => (0, 0),
			TransformerType::Prefix
			| TransformerType::Suffix
			| TransformerType::Wrap
			| TransformerType::CodeBlock => (0, 1),
			TransformerType::Indent | TransformerType::LinePrefix | TransformerType::LineSuffix => {
				(0, 2)
			}
			TransformerType::Replace => (2, 2),
		};

		if t.args.len() < min || t.args.len() > max {
			let expected = if min == max {
				format!("{min}")
			} else {
				format!("{min}-{max}")
			};
			return Err(MdtError::InvalidTransformerArgs {
				name: t.r#type.to_string(),
				expected,
				got: t.args.len(),
			});
		}
	}
	Ok(())
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

fn get_bool_arg(args: &[Argument], index: usize) -> Option<bool> {
	args.get(index).map(|arg| {
		match arg {
			Argument::Boolean(b) => *b,
			Argument::String(s) => s == "true",
			Argument::Number(n) => n.0 != 0.0,
		}
	})
}

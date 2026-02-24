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
	/// Errors encountered while rendering templates. These are collected
	/// instead of aborting so that the check reports all problems at once.
	pub render_errors: Vec<RenderError>,
}

impl CheckResult {
	/// Returns true if all consumers are up to date and no errors occurred.
	pub fn is_ok(&self) -> bool {
		self.stale.is_empty() && self.render_errors.is_empty()
	}

	/// Returns true if there are template render errors.
	pub fn has_errors(&self) -> bool {
		!self.render_errors.is_empty()
	}
}

/// A template render error associated with a specific consumer block.
#[derive(Debug)]
pub struct RenderError {
	/// Path to the file containing the consumer block.
	pub file: PathBuf,
	/// Name of the block whose template failed to render.
	pub block_name: String,
	/// The error message from the template engine.
	pub message: String,
	/// 1-indexed line number of the consumer's opening tag.
	pub line: usize,
	/// 1-indexed column number of the consumer's opening tag.
	pub column: usize,
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
	/// 1-indexed line number of the consumer's opening tag.
	pub line: usize,
	/// 1-indexed column number of the consumer's opening tag.
	pub column: usize,
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
/// Template render errors are collected rather than aborting, so the check
/// reports all problems in a single pass.
pub fn check_project(ctx: &ProjectContext) -> MdtResult<CheckResult> {
	let mut stale = Vec::new();
	let mut render_errors = Vec::new();

	for consumer in &ctx.project.consumers {
		let Some(provider) = ctx.project.providers.get(&consumer.block.name) else {
			continue;
		};

		let rendered = match render_template(&provider.content, &ctx.data) {
			Ok(r) => r,
			Err(e) => {
				render_errors.push(RenderError {
					file: consumer.file.clone(),
					block_name: consumer.block.name.clone(),
					message: e.to_string(),
					line: consumer.block.opening.start.line,
					column: consumer.block.opening.start.column,
				});
				continue;
			}
		};
		let mut expected = apply_transformers(&rendered, &consumer.block.transformers);
		if ctx.pad_blocks {
			expected = pad_content_preserving_suffix(&expected, &consumer.content);
		}

		if consumer.content != expected {
			stale.push(StaleEntry {
				file: consumer.file.clone(),
				block_name: consumer.block.name.clone(),
				current_content: consumer.content.clone(),
				expected_content: expected,
				line: consumer.block.opening.start.line,
				column: consumer.block.opening.start.column,
			});
		}
	}

	Ok(CheckResult {
		stale,
		render_errors,
	})
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
			let mut new_content = apply_transformers(&rendered, &consumer.block.transformers);
			if ctx.pad_blocks {
				new_content = pad_content_preserving_suffix(&new_content, &consumer.content);
			}

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

/// Ensure content has a leading newline and a trailing newline so it never runs
/// directly into the opening or closing tag.
pub fn pad_content(content: &str) -> String {
	let mut result = String::with_capacity(content.len() + 2);
	if !content.starts_with('\n') {
		result.push('\n');
	}
	result.push_str(content);
	if !content.ends_with('\n') {
		result.push('\n');
	}
	result
}

/// Pad content while preserving the trailing line prefix from the original
/// consumer content. When the closing tag is preceded by a comment prefix
/// (e.g., `//! ` or `/// `) that prefix is part of the content range and must
/// be preserved after replacement.
///
/// Adds an extra blank line (using the trailing prefix) between the opening
/// tag and the content and between the content and the closing tag. This
/// ensures the content is visually separated from the surrounding tags.
fn pad_content_preserving_suffix(new_content: &str, original_content: &str) -> String {
	// Extract the trailing prefix from the original content — everything after
	// the last newline. For example, in "\n//! old\n//! " the trailing prefix
	// is "//! ".
	let trailing_prefix = original_content
		.rfind('\n')
		.map(|idx| &original_content[idx + 1..])
		.unwrap_or("");

	let mut result = String::with_capacity(new_content.len() + trailing_prefix.len() * 3 + 4);
	// Leading newline
	if !new_content.starts_with('\n') {
		result.push('\n');
	}
	// Extra blank line between opening tag and content, using the comment prefix
	if !trailing_prefix.is_empty() {
		result.push_str(trailing_prefix);
		result.push('\n');
	}
	result.push_str(new_content);
	// Trailing newline
	if !new_content.ends_with('\n') {
		result.push('\n');
	}
	// Extra blank line between content and closing tag, using the comment prefix
	if !trailing_prefix.is_empty() {
		result.push_str(trailing_prefix);
		result.push('\n');
	}
	result.push_str(trailing_prefix);
	result
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

use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::BuildHasher;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use crate::Argument;
use crate::BlockType;
use crate::MdtError;
use crate::MdtResult;
use crate::Transformer;
use crate::TransformerType;
use crate::config::PaddingConfig;
use crate::parser::parse_with_diagnostics;
use crate::project::ConsumerEntry;
use crate::project::ProjectContext;
use crate::project::ProviderEntry;
use crate::project::extract_content_between_tags;
use crate::project::is_markdown_path;
use crate::project::normalize_line_endings;
use crate::source_scanner::parse_source_with_diagnostics;

/// A warning about undefined template variables in a provider block.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TemplateWarning {
	/// Path to the file containing the provider block that uses the undefined
	/// variables.
	pub provider_file: PathBuf,
	/// Name of the provider block.
	pub block_name: String,
	/// The undefined variable references found in the template (e.g.,
	/// `["pkgg.version", "typo"]`).
	pub undefined_variables: Vec<String>,
}

/// Result of checking a project for stale consumers.
#[derive(Debug)]
#[non_exhaustive]
pub struct CheckResult {
	/// Consumer entries that are out of date.
	pub stale: Vec<StaleEntry>,
	/// Files whose formatter-normalized full-file output differs even though no
	/// individual consumer content changed.
	pub stale_files: Vec<StaleFileEntry>,
	/// Errors encountered while rendering templates. These are collected
	/// instead of aborting so that the check reports all problems at once.
	pub render_errors: Vec<RenderError>,
	/// Warnings about undefined template variables in provider blocks.
	pub warnings: Vec<TemplateWarning>,
}

impl CheckResult {
	/// Returns true if all consumers are up to date and no errors occurred.
	pub fn is_ok(&self) -> bool {
		self.stale.is_empty() && self.stale_files.is_empty() && self.render_errors.is_empty()
	}

	/// Returns true if there are template render errors.
	pub fn has_errors(&self) -> bool {
		!self.render_errors.is_empty()
	}

	/// Returns true if there are warnings about undefined template variables.
	pub fn has_warnings(&self) -> bool {
		!self.warnings.is_empty()
	}
}

/// A template render error associated with a specific consumer block.
#[derive(Debug)]
#[non_exhaustive]
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
#[non_exhaustive]
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

/// <!-- {=mdtFormatterOnlyStaleDocs|trim|linePrefix:"/// ":true} -->
/// Formatter-aware checking can also report **formatter-only** drift. This happens when the formatter would rewrite the full file, but no individual managed block body is stale.
///
/// In that case mdt reports the file in `stale_files` so automation can distinguish surrounding-formatting drift from block-content drift. The CLI JSON output and MCP responses include `stale_files` for this reason.
/// <!-- {/mdtFormatterOnlyStaleDocs} -->
#[derive(Debug)]
#[non_exhaustive]
pub struct StaleFileEntry {
	/// Path to the stale file.
	pub file: PathBuf,
	/// The current full file content.
	pub current_content: String,
	/// The expected full file content after formatter normalization.
	pub expected_content: String,
}

/// Result of updating a project.
#[derive(Debug)]
#[non_exhaustive]
pub struct UpdateResult {
	/// Files that were modified and their new content.
	pub updated_files: HashMap<PathBuf, String>,
	/// Number of consumer blocks that were updated.
	pub updated_count: usize,
	/// Warnings about undefined template variables in provider blocks.
	pub warnings: Vec<TemplateWarning>,
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
	env.set_keep_trailing_newline(true);
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

/// Find template variables referenced in `content` that are not defined in
/// `data`. Returns the list of undefined variable names (with nested
/// attribute access like `"pkgg.version"`). This uses minijinja's static
/// analysis to detect undeclared variables, so it does not depend on
/// runtime control flow.
///
/// Returns an empty `Vec` when `data` is empty (no data configured means
/// template rendering is a no-op) or when the content has no template
/// syntax.
#[allow(clippy::implicit_hasher)]
pub fn find_undefined_variables(
	content: &str,
	data: &HashMap<String, serde_json::Value>,
) -> Vec<String> {
	if data.is_empty() || !has_template_syntax(content) {
		return Vec::new();
	}

	let mut env = minijinja::Environment::new();
	env.set_keep_trailing_newline(true);
	// We only need the template for static analysis, undefined behavior
	// doesn't affect undeclared_variables.
	let Ok(()) = env.add_template("__inline__", content) else {
		return Vec::new();
	};
	let Ok(template) = env.get_template("__inline__") else {
		return Vec::new();
	};

	// Get all undeclared variables with nested access (e.g., "pkg.version").
	let undeclared: HashSet<String> = template.undeclared_variables(true);

	// Also get top-level names so we can check both "pkg.version" (nested)
	// and "pkg" (top-level).
	let top_level_names: HashSet<String> = data.keys().cloned().collect();

	let mut undefined: Vec<String> = undeclared
		.into_iter()
		.filter(|var| {
			// Extract the top-level namespace from the variable reference.
			let top_level = var.split('.').next().unwrap_or(var);
			// A variable is truly undefined if its top-level namespace is
			// not present in the data context. Variables like "loop" or
			// "range" are minijinja builtins that we should not warn about.
			!top_level_names.contains(top_level) && !is_builtin_variable(top_level)
		})
		.collect();

	undefined.sort();
	undefined
}

/// Check whether a variable name is a minijinja builtin that should not
/// trigger an "undefined variable" warning.
fn is_builtin_variable(name: &str) -> bool {
	matches!(
		name,
		"loop" | "self" | "super" | "true" | "false" | "none" | "namespace" | "range" | "dict"
	)
}

/// Check whether content contains minijinja template syntax.
fn has_template_syntax(content: &str) -> bool {
	content.contains("{{") || content.contains("{%") || content.contains("{#")
}

/// Build a data context that merges base project data with block-specific
/// positional arguments. Consumer argument values are bound to the provider's
/// declared parameter names, with block args taking precedence over data
/// variables.
/// Build a data context that merges base project data with block-specific
/// positional arguments. Returns `None` if the argument count doesn't match.
pub fn build_render_context<S: BuildHasher + Clone>(
	base_data: &HashMap<String, serde_json::Value, S>,
	provider: &ProviderEntry,
	consumer: &ConsumerEntry,
) -> Option<HashMap<String, serde_json::Value, S>> {
	let param_count = provider.block.arguments.len();
	let arg_count = consumer.block.arguments.len();

	if param_count != arg_count && (param_count > 0 || arg_count > 0) {
		return None;
	}

	if provider.block.arguments.is_empty() {
		return Some(base_data.clone());
	}

	let mut data = base_data.clone();
	for (name, value) in provider
		.block
		.arguments
		.iter()
		.zip(consumer.block.arguments.iter())
	{
		data.insert(name.clone(), serde_json::Value::String(value.clone()));
	}
	Some(data)
}

/// Check whether all consumer blocks in the project are up to date.
/// Consumer blocks that reference non-existent providers are silently skipped.
/// Template render errors are collected rather than aborting, so the check
/// reports all problems in a single pass.
pub fn check_project(ctx: &ProjectContext) -> MdtResult<CheckResult> {
	if ctx.formatters.is_empty() {
		return check_project_without_formatters(ctx);
	}

	let mut stale = Vec::new();
	let mut stale_files = Vec::new();
	let mut render_errors = Vec::new();
	let warnings = collect_template_warnings(ctx);
	let consumers_by_file = group_consumers_by_file(&ctx.project.consumers);

	for (file, consumers) in consumers_by_file {
		let original = std::fs::read_to_string(&file)?;
		let ordered_consumers = sort_consumers_in_file(consumers);
		let mut candidate = original.clone();
		let mut eligible = vec![false; ordered_consumers.len()];
		let mut raw_expected: Vec<Option<String>> = vec![None; ordered_consumers.len()];

		for (index, consumer) in ordered_consumers.iter().enumerate().rev() {
			match consumer.block.r#type {
				BlockType::Consumer => {
					let Some(provider) = ctx.project.providers.get(&consumer.block.name) else {
						continue;
					};

					let Some(render_data) = build_render_context(&ctx.data, provider, consumer)
					else {
						render_errors.push(RenderError {
							file: consumer.file.clone(),
							block_name: consumer.block.name.clone(),
							message: format!(
								"argument count mismatch: provider `{}` declares {} parameter(s), \
								 but consumer passes {}",
								consumer.block.name,
								provider.block.arguments.len(),
								consumer.block.arguments.len(),
							),
							line: consumer.block.opening.start.line,
							column: consumer.block.opening.start.column,
						});
						continue;
					};
					let rendered = match render_template(&provider.content, &render_data) {
						Ok(rendered) => rendered,
						Err(error) => {
							render_errors.push(RenderError {
								file: consumer.file.clone(),
								block_name: consumer.block.name.clone(),
								message: error.to_string(),
								line: consumer.block.opening.start.line,
								column: consumer.block.opening.start.column,
							});
							continue;
						}
					};
					let mut expected = apply_transformers_with_data(
						&rendered,
						&consumer.block.transformers,
						Some(&render_data),
					);
					if let Some(padding) = &ctx.padding {
						expected = pad_content_with_config(&expected, &consumer.content, padding);
					}
					eligible[index] = true;
					raw_expected[index] = Some(expected.clone());
					if consumer.content != expected {
						replace_consumer_content(&mut candidate, consumer, &expected);
					}
				}
				BlockType::Inline => {
					let Some(template) = consumer.block.arguments.first() else {
						render_errors.push(RenderError {
							file: consumer.file.clone(),
							block_name: consumer.block.name.clone(),
							message: "inline block requires one template argument, e.g. <!-- \
							          {~name:\"{{ pkg.version }}\"} -->"
								.to_string(),
							line: consumer.block.opening.start.line,
							column: consumer.block.opening.start.column,
						});
						continue;
					};
					let rendered = match render_template(template, &ctx.data) {
						Ok(rendered) => rendered,
						Err(error) => {
							render_errors.push(RenderError {
								file: consumer.file.clone(),
								block_name: consumer.block.name.clone(),
								message: error.to_string(),
								line: consumer.block.opening.start.line,
								column: consumer.block.opening.start.column,
							});
							continue;
						}
					};
					let expected = apply_transformers_with_data(
						&rendered,
						&consumer.block.transformers,
						Some(&ctx.data),
					);
					eligible[index] = true;
					raw_expected[index] = Some(expected.clone());
					if consumer.content != expected {
						replace_consumer_content(&mut candidate, consumer, &expected);
					}
				}
				BlockType::Provider => {}
			}
		}

		let (candidate, formatter_commands) = apply_formatter_pipeline(ctx, &file, &candidate)?;
		if formatter_commands.is_empty() {
			for (index, consumer) in ordered_consumers.iter().enumerate() {
				let Some(expected) = raw_expected[index].clone() else {
					continue;
				};
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
			continue;
		}

		if candidate == original {
			continue;
		}

		let final_contents = parse_candidate_consumer_contents(
			ctx,
			&file,
			&candidate,
			ordered_consumers.len(),
			&formatter_commands,
		)?;
		let mut file_stale_count = 0;
		for (index, consumer) in ordered_consumers.iter().enumerate() {
			if !eligible[index] {
				continue;
			}
			let expected = final_contents[index].clone();
			if consumer.content != expected {
				file_stale_count += 1;
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

		if file_stale_count == 0 {
			stale_files.push(StaleFileEntry {
				file: file.clone(),
				current_content: original,
				expected_content: candidate,
			});
		}
	}

	Ok(CheckResult {
		stale,
		stale_files,
		render_errors,
		warnings,
	})
}

/// Compute the updated file contents for all consumer blocks.
pub fn compute_updates(ctx: &ProjectContext) -> MdtResult<UpdateResult> {
	if ctx.formatters.is_empty() {
		return compute_updates_without_formatters(ctx);
	}

	let mut file_contents: HashMap<PathBuf, String> = HashMap::new();
	let mut updated_count = 0;
	let warnings = collect_template_warnings(ctx);
	let consumers_by_file = group_consumers_by_file(&ctx.project.consumers);

	for (file, consumers) in consumers_by_file {
		let original = std::fs::read_to_string(&file)?;
		let ordered_consumers = sort_consumers_in_file(consumers);
		let mut candidate = original.clone();
		let mut eligible = vec![false; ordered_consumers.len()];
		let mut raw_expected: Vec<Option<String>> = vec![None; ordered_consumers.len()];

		for (index, consumer) in ordered_consumers.iter().enumerate().rev() {
			let new_content = match consumer.block.r#type {
				BlockType::Consumer => {
					let Some(provider) = ctx.project.providers.get(&consumer.block.name) else {
						continue;
					};
					let Some(render_data) = build_render_context(&ctx.data, provider, consumer)
					else {
						continue;
					};
					let rendered = render_template(&provider.content, &render_data)?;
					let mut new_content = apply_transformers_with_data(
						&rendered,
						&consumer.block.transformers,
						Some(&render_data),
					);
					if let Some(padding) = &ctx.padding {
						new_content =
							pad_content_with_config(&new_content, &consumer.content, padding);
					}
					new_content
				}
				BlockType::Inline => {
					let Some(template) = consumer.block.arguments.first() else {
						continue;
					};
					let rendered = render_template(template, &ctx.data)?;
					apply_transformers_with_data(
						&rendered,
						&consumer.block.transformers,
						Some(&ctx.data),
					)
				}
				BlockType::Provider => continue,
			};

			eligible[index] = true;
			raw_expected[index] = Some(new_content.clone());
			if consumer.content != new_content {
				replace_consumer_content(&mut candidate, consumer, &new_content);
			}
		}

		let (candidate, formatter_commands) = apply_formatter_pipeline(ctx, &file, &candidate)?;
		if candidate == original {
			continue;
		}

		if formatter_commands.is_empty() {
			updated_count += ordered_consumers
				.iter()
				.enumerate()
				.filter(|(index, consumer)| {
					raw_expected[*index]
						.as_ref()
						.is_some_and(|expected| consumer.content != *expected)
				})
				.count();
		} else {
			let final_contents = parse_candidate_consumer_contents(
				ctx,
				&file,
				&candidate,
				ordered_consumers.len(),
				&formatter_commands,
			)?;
			updated_count += ordered_consumers
				.iter()
				.enumerate()
				.filter(|(index, consumer)| {
					eligible[*index] && consumer.content != final_contents[*index]
				})
				.count();
		}

		file_contents.insert(file.clone(), candidate);
	}

	Ok(UpdateResult {
		updated_files: file_contents,
		updated_count,
		warnings,
	})
}

fn check_project_without_formatters(ctx: &ProjectContext) -> MdtResult<CheckResult> {
	let mut stale = Vec::new();
	let mut render_errors = Vec::new();
	let warnings = collect_template_warnings(ctx);

	for consumer in &ctx.project.consumers {
		match consumer.block.r#type {
			BlockType::Consumer => {
				let Some(provider) = ctx.project.providers.get(&consumer.block.name) else {
					continue;
				};

				let Some(render_data) = build_render_context(&ctx.data, provider, consumer) else {
					render_errors.push(RenderError {
						file: consumer.file.clone(),
						block_name: consumer.block.name.clone(),
						message: format!(
							"argument count mismatch: provider `{}` declares {} parameter(s), but \
							 consumer passes {}",
							consumer.block.name,
							provider.block.arguments.len(),
							consumer.block.arguments.len(),
						),
						line: consumer.block.opening.start.line,
						column: consumer.block.opening.start.column,
					});
					continue;
				};
				let rendered = match render_template(&provider.content, &render_data) {
					Ok(rendered) => rendered,
					Err(error) => {
						render_errors.push(RenderError {
							file: consumer.file.clone(),
							block_name: consumer.block.name.clone(),
							message: error.to_string(),
							line: consumer.block.opening.start.line,
							column: consumer.block.opening.start.column,
						});
						continue;
					}
				};
				let mut expected = apply_transformers_with_data(
					&rendered,
					&consumer.block.transformers,
					Some(&render_data),
				);
				if let Some(padding) = &ctx.padding {
					expected = pad_content_with_config(&expected, &consumer.content, padding);
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
			BlockType::Inline => {
				let Some(template) = consumer.block.arguments.first() else {
					render_errors.push(RenderError {
						file: consumer.file.clone(),
						block_name: consumer.block.name.clone(),
						message: "inline block requires one template argument, e.g. <!-- \
						          {~name:\"{{ pkg.version }}\"} -->"
							.to_string(),
						line: consumer.block.opening.start.line,
						column: consumer.block.opening.start.column,
					});
					continue;
				};
				let rendered = match render_template(template, &ctx.data) {
					Ok(rendered) => rendered,
					Err(error) => {
						render_errors.push(RenderError {
							file: consumer.file.clone(),
							block_name: consumer.block.name.clone(),
							message: error.to_string(),
							line: consumer.block.opening.start.line,
							column: consumer.block.opening.start.column,
						});
						continue;
					}
				};
				let expected = apply_transformers_with_data(
					&rendered,
					&consumer.block.transformers,
					Some(&ctx.data),
				);

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
			BlockType::Provider => {}
		}
	}

	Ok(CheckResult {
		stale,
		stale_files: Vec::new(),
		render_errors,
		warnings,
	})
}

fn compute_updates_without_formatters(ctx: &ProjectContext) -> MdtResult<UpdateResult> {
	let mut file_contents: HashMap<PathBuf, String> = HashMap::new();
	let mut updated_count = 0;
	let warnings = collect_template_warnings(ctx);
	let consumers_by_file = group_consumers_by_file(&ctx.project.consumers);

	for (file, consumers) in &consumers_by_file {
		let original = if let Some(content) = file_contents.get(file) {
			content.clone()
		} else {
			std::fs::read_to_string(file)?
		};

		let mut result = original.clone();
		let mut had_update = false;
		let mut sorted_consumers: Vec<&&ConsumerEntry> = consumers.iter().collect();
		sorted_consumers
			.sort_by(|a, b| b.block.opening.end.offset.cmp(&a.block.opening.end.offset));

		for consumer in sorted_consumers {
			let new_content = match consumer.block.r#type {
				BlockType::Consumer => {
					let Some(provider) = ctx.project.providers.get(&consumer.block.name) else {
						continue;
					};

					let Some(render_data) = build_render_context(&ctx.data, provider, consumer)
					else {
						continue;
					};
					let rendered = render_template(&provider.content, &render_data)?;
					let mut new_content = apply_transformers_with_data(
						&rendered,
						&consumer.block.transformers,
						Some(&render_data),
					);
					if let Some(padding) = &ctx.padding {
						new_content =
							pad_content_with_config(&new_content, &consumer.content, padding);
					}
					new_content
				}
				BlockType::Inline => {
					let Some(template) = consumer.block.arguments.first() else {
						continue;
					};
					let rendered = render_template(template, &ctx.data)?;
					apply_transformers_with_data(
						&rendered,
						&consumer.block.transformers,
						Some(&ctx.data),
					)
				}
				BlockType::Provider => continue,
			};

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
		warnings,
	})
}

fn group_consumers_by_file(consumers: &[ConsumerEntry]) -> HashMap<PathBuf, Vec<&ConsumerEntry>> {
	let mut grouped: HashMap<PathBuf, Vec<&ConsumerEntry>> = HashMap::new();
	for consumer in consumers {
		grouped
			.entry(consumer.file.clone())
			.or_default()
			.push(consumer);
	}
	grouped
}

fn sort_consumers_in_file(mut consumers: Vec<&ConsumerEntry>) -> Vec<&ConsumerEntry> {
	consumers.sort_by(|a, b| {
		a.block
			.opening
			.start
			.offset
			.cmp(&b.block.opening.start.offset)
	});
	consumers
}

fn replace_consumer_content(result: &mut String, consumer: &ConsumerEntry, new_content: &str) {
	let start = consumer.block.opening.end.offset;
	let end = consumer.block.closing.start.offset;
	if start > end || end > result.len() {
		return;
	}

	let mut buf = String::with_capacity(result.len() - (end - start) + new_content.len());
	buf.push_str(&result[..start]);
	buf.push_str(new_content);
	buf.push_str(&result[end..]);
	*result = buf;
}

fn apply_formatter_pipeline(
	ctx: &ProjectContext,
	file: &Path,
	content: &str,
) -> MdtResult<(String, Vec<String>)> {
	let matching_commands: Vec<String> = ctx
		.formatters
		.iter()
		.filter(|formatter| formatter.matches_file(&ctx.root, file))
		.map(|formatter| formatter.command.clone())
		.collect();

	let mut current = content.to_string();
	for command in &matching_commands {
		current = run_formatter_command(ctx, file, command, &current)?;
	}

	Ok((current, matching_commands))
}

fn run_formatter_command(
	ctx: &ProjectContext,
	file: &Path,
	command: &str,
	input: &str,
) -> MdtResult<String> {
	let relative_file = file.strip_prefix(&ctx.root).unwrap_or(file);
	let interpolated = interpolate_formatter_command(command, file, relative_file, &ctx.root)
		.map_err(|reason| {
			MdtError::Formatter {
				file: relative_file.display().to_string(),
				command: command.to_string(),
				reason,
			}
		})?;
	let mut command_builder = if cfg!(windows) {
		let mut command_builder = Command::new("cmd");
		command_builder.arg("/C").arg(&interpolated);
		command_builder
	} else {
		let mut command_builder = Command::new("sh");
		command_builder.arg("-c").arg(&interpolated);
		command_builder
	};
	let mut child = command_builder
		.current_dir(&ctx.root)
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()?;

	if let Some(mut stdin) = child.stdin.take() {
		stdin.write_all(input.as_bytes())?;
	}

	let output = child.wait_with_output()?;
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
		return Err(MdtError::Formatter {
			file: relative_file.display().to_string(),
			command: interpolated,
			reason,
		});
	}

	Ok(normalize_line_endings(&String::from_utf8_lossy(
		&output.stdout,
	)))
}

fn interpolate_formatter_command(
	command: &str,
	file: &Path,
	relative_file: &Path,
	root: &Path,
) -> Result<String, String> {
	if !has_template_syntax(command) {
		return Ok(command.to_string());
	}

	let mut env = minijinja::Environment::new();
	env.set_keep_trailing_newline(true);
	env.add_template("__formatter_command__", command)
		.map_err(|error| format!("invalid formatter command template: {error}"))?;

	let template = env
		.get_template("__formatter_command__")
		.map_err(|error| format!("invalid formatter command template: {error}"))?;

	template
		.render(minijinja::context! {
			filePath => file.display().to_string(),
			relativeFilePath => relative_file.display().to_string(),
			rootDirectory => root.display().to_string(),
		})
		.map_err(|error| format!("invalid formatter command template: {error}"))
}

fn parse_candidate_consumer_contents(
	ctx: &ProjectContext,
	file: &Path,
	content: &str,
	expected_consumer_count: usize,
	formatter_commands: &[String],
) -> MdtResult<Vec<String>> {
	let normalized = normalize_line_endings(content);
	let (blocks, _) = if is_markdown_path(file) {
		parse_with_diagnostics(&normalized).map_err(|error| {
			MdtError::Formatter {
				file: file
					.strip_prefix(&ctx.root)
					.unwrap_or(file)
					.display()
					.to_string(),
				command: formatter_commands.join(" && "),
				reason: format!("formatter pipeline produced unparsable markdown: {error}"),
			}
		})?
	} else {
		parse_source_with_diagnostics(&normalized, &ctx.markdown_codeblocks).map_err(|error| {
			MdtError::Formatter {
				file: file
					.strip_prefix(&ctx.root)
					.unwrap_or(file)
					.display()
					.to_string(),
				command: formatter_commands.join(" && "),
				reason: format!("formatter pipeline produced unparsable source comments: {error}"),
			}
		})?
	};
	let consumer_contents: Vec<String> = blocks
		.into_iter()
		.filter(|block| matches!(block.r#type, BlockType::Consumer | BlockType::Inline))
		.map(|block| extract_content_between_tags(&normalized, &block))
		.collect();

	if consumer_contents.len() != expected_consumer_count {
		return Err(MdtError::Formatter {
			file: file
				.strip_prefix(&ctx.root)
				.unwrap_or(file)
				.display()
				.to_string(),
			command: formatter_commands.join(" && "),
			reason: format!(
				"formatter pipeline changed the number of consumer blocks from {} to {}",
				expected_consumer_count,
				consumer_contents.len()
			),
		});
	}

	Ok(consumer_contents)
}

/// Collect warnings about undefined template variables across all provider
/// blocks that have at least one consumer. Each provider is checked at most
/// once even if it has multiple consumers.
fn collect_template_warnings(ctx: &ProjectContext) -> Vec<TemplateWarning> {
	let mut warnings = Vec::new();
	let mut checked_providers: HashSet<String> = HashSet::new();

	// Only check providers that are actually referenced by consumers.
	for consumer in &ctx.project.consumers {
		if consumer.block.r#type != BlockType::Consumer {
			continue;
		}
		let name = &consumer.block.name;
		if checked_providers.contains(name) {
			continue;
		}
		checked_providers.insert(name.clone());

		let Some(provider) = ctx.project.providers.get(name) else {
			continue;
		};

		// Provider params are known variables — add them to the data context
		// so they don't trigger false undefined-variable warnings.
		let data_with_params = if provider.block.arguments.is_empty() {
			std::borrow::Cow::Borrowed(&ctx.data)
		} else {
			let mut data = ctx.data.clone();
			for param in &provider.block.arguments {
				data.entry(param.clone())
					.or_insert(serde_json::Value::String(String::new()));
			}
			std::borrow::Cow::Owned(data)
		};

		let undefined = find_undefined_variables(&provider.content, &data_with_params);
		if !undefined.is_empty() {
			warnings.push(TemplateWarning {
				provider_file: provider.file.clone(),
				block_name: name.clone(),
				undefined_variables: undefined,
			});
		}
	}

	warnings
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
	apply_transformers_with_data(content, transformers, None)
}

/// Apply a sequence of transformers to content with an optional data context.
/// The data context is used by data-dependent transformers like `if`.
#[allow(clippy::implicit_hasher)]
pub fn apply_transformers_with_data(
	content: &str,
	transformers: &[Transformer],
	data: Option<&HashMap<String, serde_json::Value>>,
) -> String {
	let mut result = content.to_string();

	for transformer in transformers {
		result = apply_transformer(&result, transformer, data);
	}

	result
}

fn apply_transformer(
	content: &str,
	transformer: &Transformer,
	data: Option<&HashMap<String, serde_json::Value>>,
) -> String {
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
					} else if line.is_empty() {
						prefix.trim_end().to_string()
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
					} else if line.is_empty() {
						suffix.trim_start().to_string()
					} else {
						format!("{line}{suffix}")
					}
				})
				.collect::<Vec<_>>()
				.join("\n")
		}
		TransformerType::If => {
			let path = get_string_arg(&transformer.args, 0).unwrap_or_default();
			if is_data_path_truthy(data, &path) {
				content.to_string()
			} else {
				String::new()
			}
		}
	}
}

/// Look up a dot-separated path in the data context and return whether the
/// value is "truthy". A value is truthy if it exists and is not `false`,
/// `null`, `""`, or `0`.
fn is_data_path_truthy(data: Option<&HashMap<String, serde_json::Value>>, path: &str) -> bool {
	let Some(data) = data else {
		return false;
	};

	let mut parts = path.split('.');
	let Some(root) = parts.next() else {
		return false;
	};

	let Some(mut current) = data.get(root) else {
		return false;
	};

	for part in parts {
		match current {
			serde_json::Value::Object(map) => {
				let Some(next) = map.get(part) else {
					return false;
				};
				current = next;
			}
			_ => return false,
		}
	}

	is_json_value_truthy(current)
}

/// Check whether a JSON value is truthy.
/// A value is falsy if it is `null`, `false`, `""`, `0`, or `0.0`.
/// Everything else (including non-empty arrays and objects) is truthy.
fn is_json_value_truthy(value: &serde_json::Value) -> bool {
	match value {
		serde_json::Value::Null => false,
		serde_json::Value::Bool(b) => *b,
		serde_json::Value::Number(n) => {
			// 0 and 0.0 are falsy
			if let Some(i) = n.as_i64() {
				i != 0
			} else if let Some(u) = n.as_u64() {
				u != 0
			} else if let Some(f) = n.as_f64() {
				f != 0.0
			} else {
				true
			}
		}
		serde_json::Value::String(s) => !s.is_empty(),
		serde_json::Value::Array(_) | serde_json::Value::Object(_) => true,
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
			TransformerType::If => (1, 1),
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

/// Pad content according to the padding configuration while preserving the
/// trailing line prefix from the original consumer content. When the closing
/// tag is preceded by a comment prefix (e.g., `//! ` or `/// `) that prefix
/// is part of the content range and must be preserved after replacement.
///
/// The `before` value controls blank lines between the opening tag and
/// content, and `after` controls blank lines between content and the closing
/// tag. Each value can be:
///
/// - `false` — No padding; content appears inline with the tag.
/// - `0` — Content on the very next line (one newline, no blank lines).
/// - `1` — One blank line between the tag and content.
/// - `2` — Two blank lines, and so on.
fn pad_content_with_config(
	new_content: &str,
	original_content: &str,
	padding: &PaddingConfig,
) -> String {
	// Extract the trailing prefix from the original content — everything after
	// the last newline. For example, in "\n//! old\n//! " the trailing prefix
	// is "//! ".
	let trailing_prefix = original_content
		.rfind('\n')
		.map_or("", |idx| &original_content[idx + 1..]);
	// Trimmed prefix for blank padding lines — avoids trailing whitespace
	// on empty lines (e.g., "//! " becomes "//!").
	let blank_line_prefix = trailing_prefix.trim_end();

	let mut result = String::with_capacity(new_content.len() + trailing_prefix.len() * 4 + 8);

	// Before padding: lines between opening tag and content
	match padding.before.line_count() {
		None => {
			// false — content inline with opening tag
		}
		Some(0) => {
			// Content on the very next line
			if !new_content.starts_with('\n') {
				result.push('\n');
			}
		}
		Some(n) => {
			// N blank lines between opening tag and content
			if !new_content.starts_with('\n') {
				result.push('\n');
			}
			for _ in 0..n {
				result.push_str(blank_line_prefix);
				result.push('\n');
			}
		}
	}

	result.push_str(new_content);

	// After padding: lines between content and closing tag
	match padding.after.line_count() {
		None => {
			// false — closing tag inline with content
		}
		Some(0) => {
			// Closing tag on the very next line
			if !new_content.ends_with('\n') {
				result.push('\n');
			}
			result.push_str(trailing_prefix);
		}
		Some(n) => {
			if !new_content.ends_with('\n') {
				result.push('\n');
			}
			for _ in 0..n {
				result.push_str(blank_line_prefix);
				result.push('\n');
			}
			result.push_str(trailing_prefix);
		}
	}

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

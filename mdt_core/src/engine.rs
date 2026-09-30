use std::cmp::Reverse;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::BuildHasher;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use tracing::debug;
use tracing::instrument;
use tracing::trace;
use tracing::warn;

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
use crate::project::restore_line_endings;
use crate::project::suggest_similar_provider_names;
use crate::source_scanner::extract_line_comment_prefix;
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
	/// Whether the provider is rendered as a template at all. Providers are
	/// only rendered when `[data]` is configured (or the provider declares
	/// parameters); otherwise `{{ ... }}` is copied to consumers literally.
	pub template_rendered: bool,
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
	/// Consumers whose block name matches no provider. `mdt update` cannot
	/// sync them, so they fail the check like stale consumers do.
	pub orphans: Vec<OrphanConsumer>,
	/// Warnings about undefined template variables in provider blocks.
	pub warnings: Vec<TemplateWarning>,
}

impl CheckResult {
	/// Returns true if every consumer is linked and up to date and no errors
	/// occurred.
	pub fn is_ok(&self) -> bool {
		self.stale.is_empty()
			&& self.stale_files.is_empty()
			&& self.render_errors.is_empty()
			&& self.orphans.is_empty()
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

/// A consumer block whose name matches no provider — usually a misspelled
/// name, or a provider defined outside a `*.t.md` file.
#[derive(Debug)]
#[non_exhaustive]
pub struct OrphanConsumer {
	/// Path to the file containing the consumer.
	pub file: PathBuf,
	/// The block name that matches no provider.
	pub block_name: String,
	/// 1-indexed line number of the consumer's opening tag.
	pub line: usize,
	/// 1-indexed column number of the consumer's opening tag.
	pub column: usize,
	/// Existing provider names close to `block_name`, best match first.
	pub suggestions: Vec<String>,
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
/// In that case mdt reports the file in `stale_files` so automation can tell surrounding-formatting drift from block-content drift. The CLI JSON output and MCP responses include `stale_files` for this reason.
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
	/// Consumers left untouched because their provider failed to render.
	pub render_errors: Vec<RenderError>,
	/// Warnings about undefined template variables in provider blocks.
	pub warnings: Vec<TemplateWarning>,
}

/// Render provider content through minijinja using the given data context.
/// If data is empty or the content has no template syntax, returns the
/// content unchanged.
#[allow(clippy::implicit_hasher)]
#[instrument(skip(content, data), fields(content_len = content.len(), data_keys = data.len()))]
pub fn render_template(
	content: &str,
	data: &HashMap<String, serde_json::Value>,
) -> MdtResult<String> {
	trace!("rendering template");
	if data.is_empty() || !has_template_syntax(content) {
		return Ok(content.to_string());
	}

	let mut env = minijinja::Environment::new();
	env.set_keep_trailing_newline(true);
	env.set_undefined_behavior(minijinja::UndefinedBehavior::Chainable);
	env.add_template("__inline__", content)
		.map_err(template_render_error)?;

	let template = env
		.get_template("__inline__")
		.map_err(template_render_error)?;

	let ctx = minijinja::Value::from_serialize(data);
	template.render(ctx).map_err(template_render_error)
}

/// Describe a minijinja failure by kind, detail, and line within the
/// template, leaving out the internal template name.
#[allow(clippy::needless_pass_by_value)] // Matches `map_err`'s by-value closure.
fn template_render_error(error: minijinja::Error) -> MdtError {
	let detail = error
		.detail()
		.map_or_else(String::new, |detail| format!(": {detail}"));
	let line = error
		.line()
		.map_or_else(String::new, |line| format!(" (template line {line})"));
	MdtError::TemplateRender(format!("{}{detail}{line}", error.kind()))
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
	if data.is_empty() {
		return Vec::new();
	}

	undeclared_variables(content)
		.into_iter()
		// A variable is truly undefined if its top-level namespace is not
		// present in the data context.
		.filter(|var| !data.contains_key(var.split('.').next().unwrap_or(var)))
		.collect()
}

/// Every variable a template reads without defining it, with nested access
/// (`pkg.version`), sorted. Minijinja builtins such as `loop` are left out;
/// unparsable templates yield nothing (rendering reports those).
fn undeclared_variables(content: &str) -> Vec<String> {
	if !has_template_syntax(content) {
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

	let mut undeclared: Vec<String> = template
		.undeclared_variables(true)
		.into_iter()
		.filter(|var| !is_builtin_variable(var.split('.').next().unwrap_or(var)))
		.collect();
	undeclared.sort();
	undeclared
}

/// Check whether a variable name is a minijinja builtin that should not
/// trigger an "undefined variable" warning.
fn is_builtin_variable(name: &str) -> bool {
	matches!(
		name,
		"loop" | "self" | "super" | "true" | "false" | "none" | "namespace" | "range" | "dict"
	)
}

/// Normalize content for lenient comparison by collapsing insignificant
/// whitespace. This makes `mdt check` tolerant of formatter rewrites that
/// only change blank lines, trailing spaces, or indentation alignment.
///
/// The normalization:
/// - trims each line of trailing whitespace
/// - collapses runs of blank lines into a single blank line
/// - trims leading/trailing blank lines from the whole string
pub fn normalize_whitespace(content: &str) -> String {
	let mut result = String::with_capacity(content.len());
	let mut prev_blank = false;

	for line in content.split('\n') {
		let trimmed = line.trim_end();
		let is_blank = trimmed.is_empty();

		if is_blank {
			if !prev_blank && !result.is_empty() {
				result.push('\n');
			}
			prev_blank = true;
		} else {
			if !result.is_empty() {
				result.push('\n');
			}
			result.push_str(trimmed);
			prev_blank = false;
		}
	}

	result
}

/// Whether a consumer's current content passes `mdt check` against the
/// expected content under the configured `[check] comparison` mode.
pub fn content_matches(
	actual: &str,
	expected: &str,
	comparison: &crate::config::ComparisonMode,
) -> bool {
	match comparison {
		crate::config::ComparisonMode::Strict => actual == expected,
		crate::config::ComparisonMode::Lenient => {
			// Most consumers are up to date; skip the normalizing allocations
			// whenever the byte-for-byte comparison already succeeds.
			actual == expected || normalize_whitespace(actual) == normalize_whitespace(expected)
		}
	}
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

/// A provider rendered for one consumer.
struct RenderedForConsumer<'a> {
	/// The rendered content, or the render error message.
	rendered: Result<String, String>,
	/// Data context for data-aware transformers (e.g. `if`).
	data: std::borrow::Cow<'a, HashMap<String, serde_json::Value>>,
}

/// Caches rendered provider content across consumers within a single run.
///
/// Providers without parameters render identically for every consumer, so
/// the minijinja environment, template compile, and render run once per
/// provider instead of once per consumer — and the base data map is no
/// longer deep-cloned per consumer.
struct RenderCache<'a> {
	data: &'a HashMap<String, serde_json::Value>,
	rendered: HashMap<String, Result<String, String>>,
}

impl<'a> RenderCache<'a> {
	fn new(data: &'a HashMap<String, serde_json::Value>) -> Self {
		Self {
			data,
			rendered: HashMap::new(),
		}
	}

	/// Render `provider` content for `consumer`.
	///
	/// Returns `None` when the consumer's argument count does not match the
	/// provider's parameter count. Parameterized providers render per
	/// consumer with their argument values; providers without parameters use
	/// the shared cache.
	fn render(
		&mut self,
		provider: &ProviderEntry,
		consumer: &ConsumerEntry,
	) -> Option<RenderedForConsumer<'_>> {
		let param_count = provider.block.arguments.len();
		let arg_count = consumer.block.arguments.len();

		if param_count != arg_count && (param_count > 0 || arg_count > 0) {
			return None;
		}

		if provider.block.arguments.is_empty() {
			let rendered = self
				.rendered
				.entry(provider.block.name.clone())
				.or_insert_with(|| {
					render_template(&provider.content, self.data).map_err(render_error_message)
				})
				.clone();
			return Some(RenderedForConsumer {
				rendered,
				data: std::borrow::Cow::Borrowed(self.data),
			});
		}

		let render_data = build_render_context(self.data, provider, consumer)?;
		let rendered =
			render_template(&provider.content, &render_data).map_err(render_error_message);
		Some(RenderedForConsumer {
			rendered,
			data: std::borrow::Cow::Owned(render_data),
		})
	}
}

/// The content `mdt update` writes between one consumer's tags, before any
/// `[[formatters]]` run.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExpectedContent {
	/// The rendered, transformed, and padded content.
	Rendered(String),
	/// The block names no provider, so mdt leaves its content untouched.
	NoProvider,
	/// The provider template could not be rendered for this consumer; the
	/// message explains why.
	RenderFailed(String),
}

/// Compute what `mdt update` would write between `consumer`'s tags.
///
/// `source` is the current text of the consumer's file: the comment prefix
/// in front of the closing tag is re-applied after the configured padding.
/// The CLI, MCP server, and language server all report staleness through
/// this function, so they agree on data, transformers, and padding.
pub fn expected_consumer_content(
	ctx: &ProjectContext,
	consumer: &ConsumerEntry,
	source: &str,
) -> ExpectedContent {
	expected_content(ctx, &mut RenderCache::new(&ctx.data), consumer, source)
}

fn expected_content(
	ctx: &ProjectContext,
	render_cache: &mut RenderCache<'_>,
	consumer: &ConsumerEntry,
	source: &str,
) -> ExpectedContent {
	match consumer.block.r#type {
		BlockType::Consumer => {
			let Some(provider) = ctx.project.providers.get(&consumer.block.name) else {
				return ExpectedContent::NoProvider;
			};
			let Some(rendered) = render_cache.render(provider, consumer) else {
				return ExpectedContent::RenderFailed(format!(
					"argument count mismatch: provider `{}` declares {} parameter(s), but \
					 consumer passes {}",
					consumer.block.name,
					provider.block.arguments.len(),
					consumer.block.arguments.len(),
				));
			};
			let rendered_content = match rendered.rendered {
				Ok(rendered_content) => rendered_content,
				Err(message) => return ExpectedContent::RenderFailed(message),
			};
			let transformed = apply_transformers_with_data(
				&rendered_content,
				&consumer.block.transformers,
				Some(&rendered.data),
			);
			ExpectedContent::Rendered(pad_content_with_config(
				&transformed,
				closing_tag_prefix(consumer, source),
				effective_padding(ctx),
			))
		}
		BlockType::Inline => {
			let Some(template) = consumer.block.arguments.first() else {
				return ExpectedContent::RenderFailed(
					"inline block requires one template argument, e.g. <!-- {~name:\"{{ \
					 pkg.version }}\"} -->"
						.to_string(),
				);
			};
			match render_template(template, &ctx.data) {
				Ok(rendered) => {
					ExpectedContent::Rendered(apply_transformers_with_data(
						&rendered,
						&consumer.block.transformers,
						Some(&ctx.data),
					))
				}
				Err(error) => ExpectedContent::RenderFailed(render_error_message(error)),
			}
		}
		// Scanning never records providers as consumers.
		BlockType::Provider => ExpectedContent::NoProvider,
	}
}

/// Whether a `[[formatters]]` entry formats `file`. For such files the
/// content `mdt check` expects is only known after running the formatter.
pub fn formatter_applies(ctx: &ProjectContext, file: &Path) -> bool {
	!ctx.formatters.is_empty()
		&& !FormatterPipeline::compile(&ctx.formatters)
			.commands_for(&ctx.root, file)
			.is_empty()
}

/// The text in front of a consumer's closing tag that padding re-applies.
///
/// In source files that is the comment prefix (`//! `, ` * `). Markdown has
/// no comment prefixes — `#` starts a heading and `*` a list item — so only
/// indentation carries over there.
fn closing_tag_prefix<'a>(consumer: &ConsumerEntry, source: &'a str) -> &'a str {
	let prefix = extract_line_comment_prefix(source, consumer.block.closing.start.offset);
	if is_markdown_path(&consumer.file) && !prefix.trim().is_empty() {
		""
	} else {
		prefix
	}
}

/// The template engine's explanation of a render failure, without the
/// generic "template rendering failed" prefix callers add themselves.
fn render_error_message(error: MdtError) -> String {
	match error {
		MdtError::TemplateRender(message) => message,
		other => other.to_string(),
	}
}

fn render_error(consumer: &ConsumerEntry, message: String) -> RenderError {
	RenderError {
		file: consumer.file.clone(),
		block_name: consumer.block.name.clone(),
		message,
		line: consumer.block.opening.start.line,
		column: consumer.block.opening.start.column,
	}
}

fn orphan_consumer(ctx: &ProjectContext, consumer: &ConsumerEntry) -> OrphanConsumer {
	OrphanConsumer {
		file: consumer.file.clone(),
		block_name: consumer.block.name.clone(),
		line: consumer.block.opening.start.line,
		column: consumer.block.opening.start.column,
		suggestions: suggest_similar_provider_names(
			&consumer.block.name,
			ctx.project.providers.keys().map(String::as_str),
		)
		.into_iter()
		.map(str::to_string)
		.collect(),
	}
}

fn stale_entry(consumer: &ConsumerEntry, expected_content: String) -> StaleEntry {
	StaleEntry {
		file: consumer.file.clone(),
		block_name: consumer.block.name.clone(),
		current_content: consumer.content.clone(),
		expected_content,
		line: consumer.block.opening.start.line,
		column: consumer.block.opening.start.column,
	}
}

/// Check whether all consumer blocks in the project are up to date.
/// Consumer blocks that reference non-existent providers are skipped here;
/// they are reported through [`ProjectContext::find_missing_providers`].
/// Template render errors are collected rather than aborting, so the check
/// reports all problems in a single pass.
#[instrument(skip(ctx), fields(
	root = %ctx.root.display(),
	providers = ctx.project.providers.len(),
	consumers = ctx.project.consumers.len(),
	has_formatters = !ctx.formatters.is_empty(),
))]
pub fn check_project(ctx: &ProjectContext) -> MdtResult<CheckResult> {
	debug!("checking project");
	if ctx.formatters.is_empty() {
		return check_project_without_formatters(ctx);
	}

	let formatter_pipeline = FormatterPipeline::compile(&ctx.formatters);
	let mut render_cache = RenderCache::new(&ctx.data);
	let mut stale = Vec::new();
	let mut stale_files = Vec::new();
	let mut render_errors = Vec::new();
	let mut orphans = Vec::new();
	let warnings = collect_template_warnings(ctx);
	debug!(warnings = warnings.len(), "collected template warnings");
	let consumers_by_file = group_consumers_by_file(&ctx.project.consumers);

	for (file, consumers) in consumers_by_file {
		trace!(file = %file.display(), consumers = consumers.len(), "checking file");
		let original = normalize_line_endings(&std::fs::read_to_string(&file)?);
		let ordered_consumers = sort_consumers_in_file(consumers);
		let mut candidate = original.clone();
		let mut eligible = vec![false; ordered_consumers.len()];
		let mut raw_expected: Vec<Option<String>> = vec![None; ordered_consumers.len()];

		for (index, consumer) in ordered_consumers.iter().enumerate().rev() {
			let expected = match expected_content(ctx, &mut render_cache, consumer, &original) {
				ExpectedContent::Rendered(expected) => expected,
				ExpectedContent::NoProvider => {
					orphans.push(orphan_consumer(ctx, consumer));
					continue;
				}
				ExpectedContent::RenderFailed(message) => {
					warn!(
						file = %consumer.file.display(),
						block = consumer.block.name,
						error = %message,
						"template render failed",
					);
					render_errors.push(render_error(consumer, message));
					continue;
				}
			};
			eligible[index] = true;
			if consumer.content != expected {
				replace_consumer_content(&mut candidate, consumer, &expected);
			}
			raw_expected[index] = Some(expected);
		}

		let (candidate, formatter_commands) =
			apply_formatter_pipeline(&formatter_pipeline, ctx, &file, &candidate)?;
		if formatter_commands.is_empty() {
			for (index, consumer) in ordered_consumers.iter().enumerate() {
				let Some(expected) = raw_expected[index].take() else {
					continue;
				};
				if !content_matches(&consumer.content, &expected, &ctx.comparison) {
					stale.push(stale_entry(consumer, expected));
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
			&ordered_consumers,
			&formatter_commands,
		)?;
		let mut file_stale_count = 0;
		for (index, consumer) in ordered_consumers.iter().enumerate() {
			if !eligible[index] {
				continue;
			}
			let expected = final_contents[index].clone();
			if !content_matches(&consumer.content, &expected, &ctx.comparison) {
				file_stale_count += 1;
				stale.push(stale_entry(consumer, expected));
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

	// Files are visited in hash order; report orphans in source order.
	orphans.sort_by(|a, b| (&a.file, a.line, a.column).cmp(&(&b.file, b.line, b.column)));

	debug!(
		stale = stale.len(),
		stale_files = stale_files.len(),
		render_errors = render_errors.len(),
		orphans = orphans.len(),
		"check complete",
	);

	Ok(CheckResult {
		stale,
		stale_files,
		render_errors,
		orphans,
		warnings,
	})
}

/// Compute the updated file contents for all consumer blocks.
///
/// Consumers whose provider fails to render are left untouched and reported
/// in [`UpdateResult::render_errors`]; every other consumer is still updated.
#[instrument(skip(ctx), fields(
	root = %ctx.root.display(),
	providers = ctx.project.providers.len(),
	consumers = ctx.project.consumers.len(),
	has_formatters = !ctx.formatters.is_empty(),
))]
pub fn compute_updates(ctx: &ProjectContext) -> MdtResult<UpdateResult> {
	debug!("computing updates");
	if ctx.formatters.is_empty() {
		return compute_updates_without_formatters(ctx);
	}

	let formatter_pipeline = FormatterPipeline::compile(&ctx.formatters);
	let mut render_cache = RenderCache::new(&ctx.data);
	let mut file_contents: HashMap<PathBuf, String> = HashMap::new();
	let mut updated_count = 0;
	let mut render_errors = Vec::new();
	let warnings = collect_template_warnings(ctx);
	let consumers_by_file = group_consumers_by_file(&ctx.project.consumers);

	for (file, consumers) in consumers_by_file {
		trace!(file = %file.display(), "processing file for updates");
		let raw = std::fs::read_to_string(&file)?;
		let original = normalize_line_endings(&raw);
		let ordered_consumers = sort_consumers_in_file(consumers);
		let mut candidate = original.clone();
		let mut eligible = vec![false; ordered_consumers.len()];
		let mut raw_expected: Vec<Option<String>> = vec![None; ordered_consumers.len()];

		for (index, consumer) in ordered_consumers.iter().enumerate().rev() {
			let new_content = match expected_content(ctx, &mut render_cache, consumer, &original) {
				ExpectedContent::Rendered(new_content) => new_content,
				ExpectedContent::NoProvider => continue,
				ExpectedContent::RenderFailed(message) => {
					render_errors.push(render_error(consumer, message));
					continue;
				}
			};

			eligible[index] = true;
			if consumer.content != new_content {
				replace_consumer_content(&mut candidate, consumer, &new_content);
			}
			raw_expected[index] = Some(new_content);
		}

		let (candidate, formatter_commands) =
			apply_formatter_pipeline(&formatter_pipeline, ctx, &file, &candidate)?;
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
				&ordered_consumers,
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

		file_contents.insert(file.clone(), restore_line_endings(&candidate, &raw));
	}

	debug!(
		updated_files = file_contents.len(),
		updated_count, "updates computed"
	);

	Ok(UpdateResult {
		updated_files: file_contents,
		updated_count,
		render_errors,
		warnings,
	})
}

fn check_project_without_formatters(ctx: &ProjectContext) -> MdtResult<CheckResult> {
	let mut stale = Vec::new();
	let mut render_errors = Vec::new();
	let mut orphans = Vec::new();
	let warnings = collect_template_warnings(ctx);

	// Cache file contents so the closing-tag comment prefix can be recovered
	// from the source when padding is enabled.
	let mut file_contents: HashMap<PathBuf, String> = HashMap::new();
	let mut render_cache = RenderCache::new(&ctx.data);

	for consumer in &ctx.project.consumers {
		if !file_contents.contains_key(&consumer.file) {
			file_contents.insert(
				consumer.file.clone(),
				normalize_line_endings(&std::fs::read_to_string(&consumer.file)?),
			);
		}
		let source = &file_contents[&consumer.file];
		match expected_content(ctx, &mut render_cache, consumer, source) {
			ExpectedContent::Rendered(expected) => {
				if !content_matches(&consumer.content, &expected, &ctx.comparison) {
					stale.push(stale_entry(consumer, expected));
				}
			}
			ExpectedContent::NoProvider => orphans.push(orphan_consumer(ctx, consumer)),
			ExpectedContent::RenderFailed(message) => {
				render_errors.push(render_error(consumer, message));
			}
		}
	}

	Ok(CheckResult {
		stale,
		stale_files: Vec::new(),
		render_errors,
		orphans,
		warnings,
	})
}

fn compute_updates_without_formatters(ctx: &ProjectContext) -> MdtResult<UpdateResult> {
	let mut file_contents: HashMap<PathBuf, String> = HashMap::new();
	let mut updated_count = 0;
	let mut render_errors = Vec::new();
	let warnings = collect_template_warnings(ctx);
	let consumers_by_file = group_consumers_by_file(&ctx.project.consumers);
	let mut render_cache = RenderCache::new(&ctx.data);

	for (file, consumers) in &consumers_by_file {
		let raw = std::fs::read_to_string(file)?;
		let original = normalize_line_endings(&raw);

		let mut result = original.clone();
		let mut had_update = false;
		let mut sorted_consumers: Vec<&&ConsumerEntry> = consumers.iter().collect();
		sorted_consumers.sort_by_key(|b| Reverse(b.block.opening.end.offset));

		for consumer in sorted_consumers {
			let new_content = match expected_content(ctx, &mut render_cache, consumer, &original) {
				ExpectedContent::Rendered(new_content) => new_content,
				ExpectedContent::NoProvider => continue,
				ExpectedContent::RenderFailed(message) => {
					render_errors.push(render_error(consumer, message));
					continue;
				}
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
			file_contents.insert(file.clone(), restore_line_endings(&result, &raw));
		}
	}

	Ok(UpdateResult {
		updated_files: file_contents,
		updated_count,
		render_errors,
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
	pipeline: &FormatterPipeline,
	ctx: &ProjectContext,
	file: &Path,
	content: &str,
) -> MdtResult<(String, Vec<String>)> {
	let matching_commands: Vec<String> = pipeline
		.commands_for(&ctx.root, file)
		.into_iter()
		.map(str::to_string)
		.collect();

	let mut current = content.to_string();
	for command in &matching_commands {
		current = run_formatter_command(ctx, file, command, &current)?;
	}

	Ok((current, matching_commands))
}

/// Formatter pipeline with precompiled glob matchers, built once per
/// check/update run so per-file routing does not recompile every pattern for
/// every scanned file.
struct FormatterPipeline {
	entries: Vec<FormatterPipelineEntry>,
}

struct FormatterPipelineEntry {
	command: String,
	patterns: crate::config::FormatterRuleSet,
	ignore: crate::config::FormatterRuleSet,
}

impl FormatterPipeline {
	fn compile(formatters: &[crate::config::FormatterConfig]) -> Self {
		Self {
			entries: formatters
				.iter()
				.map(|formatter| {
					FormatterPipelineEntry {
						command: formatter.command.clone(),
						patterns: crate::config::FormatterRuleSet::compile(&formatter.patterns),
						ignore: crate::config::FormatterRuleSet::compile(&formatter.ignore),
					}
				})
				.collect(),
		}
	}

	/// Returns the commands matching `file`, in declaration order.
	fn commands_for<'a>(&'a self, root: &Path, file: &Path) -> Vec<&'a str> {
		let relative = file.strip_prefix(root).unwrap_or(file);
		let key = relative.to_string_lossy().replace('\\', "/");

		self.entries
			.iter()
			.filter(|entry| entry.patterns.is_match(&key) && !entry.ignore.is_match(&key))
			.map(|entry| entry.command.as_str())
			.collect()
	}
}

/// Environment variables that carry the formatter template values.
///
/// `{{ filePath }}`, `{{ relativeFilePath }}`, and `{{ rootDirectory }}` render
/// as references to these variables rather than as the raw paths, so the
/// shell expands them as data. Pasting a path into `sh -c` would let a file
/// named ``a$(cmd).md`` run `cmd`.
const FORMATTER_FILE_PATH_ENV: &str = "MDT_FILE_PATH";
const FORMATTER_RELATIVE_FILE_PATH_ENV: &str = "MDT_RELATIVE_FILE_PATH";
const FORMATTER_ROOT_DIRECTORY_ENV: &str = "MDT_ROOT_DIRECTORY";

/// Reference an environment variable in the shell that runs formatter
/// commands (`sh` on Unix, `cmd` on Windows).
fn shell_variable_reference(name: &str) -> String {
	if cfg!(windows) {
		format!("%{name}%")
	} else {
		format!("${{{name}}}")
	}
}

fn run_formatter_command(
	ctx: &ProjectContext,
	file: &Path,
	command: &str,
	input: &str,
) -> MdtResult<String> {
	let relative_file = file.strip_prefix(&ctx.root).unwrap_or(file);
	let interpolated = interpolate_formatter_command(command).map_err(|reason| {
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
		.env(FORMATTER_FILE_PATH_ENV, file)
		.env(FORMATTER_RELATIVE_FILE_PATH_ENV, relative_file)
		.env(FORMATTER_ROOT_DIRECTORY_ENV, &ctx.root)
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()?;

	// Feed stdin from a dedicated thread while the main thread drains stdout.
	// Writing all input before reading output can deadlock when the formatter
	// fills its stdout pipe while we are still writing a large file.
	let stdin = child.stdin.take();
	let output = std::thread::scope(|scope| {
		let writer = stdin.map(|mut stdin| {
			scope.spawn(move || {
				// A write error here (e.g. formatter exited early) surfaces as
				// a non-zero exit status below; dropping stdin closes the pipe
				// so formatters reading to EOF still terminate.
				let _ = stdin.write_all(input.as_bytes());
			})
		});

		let output = child.wait_with_output();

		if let Some(writer) = writer {
			let _ = writer.join();
		}

		output
	})?;

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

fn interpolate_formatter_command(command: &str) -> Result<String, String> {
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
			filePath => shell_variable_reference(FORMATTER_FILE_PATH_ENV),
			relativeFilePath => shell_variable_reference(FORMATTER_RELATIVE_FILE_PATH_ENV),
			rootDirectory => shell_variable_reference(FORMATTER_ROOT_DIRECTORY_ENV),
		})
		.map_err(|error| format!("failed to render formatter command template: {error}"))
}

fn parse_candidate_consumer_contents(
	ctx: &ProjectContext,
	file: &Path,
	content: &str,
	consumers: &[&ConsumerEntry],
	formatter_commands: &[String],
) -> MdtResult<Vec<String>> {
	let expected_consumer_count = consumers.len();
	// Blocks named in `[exclude] blocks` are never scanned, so keep only the
	// names this file's scan produced when matching blocks back up.
	let managed_names: HashSet<&str> = consumers
		.iter()
		.map(|consumer| consumer.block.name.as_str())
		.collect();
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
		.filter(|block| {
			matches!(block.r#type, BlockType::Consumer | BlockType::Inline)
				&& managed_names.contains(block.name.as_str())
		})
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

		// Without data the provider is copied verbatim. Namespaced
		// variables (`pkg.version`) are almost certainly meant to render —
		// typically a sub-project reusing shared providers without its own
		// `[data]` — so point them out instead of copying them silently.
		let template_rendered = !data_with_params.is_empty();
		let undefined = if template_rendered {
			find_undefined_variables(&provider.content, &data_with_params)
		} else {
			undeclared_variables(&provider.content)
				.into_iter()
				.filter(|variable| variable.contains('.'))
				.collect()
		};
		if !undefined.is_empty() {
			warnings.push(TemplateWarning {
				provider_file: provider.file.clone(),
				block_name: name.clone(),
				undefined_variables: undefined,
				template_rendered,
			});
		}
	}

	warnings
}

/// Write the updated contents back to disk.
#[instrument(skip(updates), fields(file_count = updates.updated_files.len()))]
pub fn write_updates(updates: &UpdateResult) -> MdtResult<()> {
	for (path, content) in &updates.updated_files {
		trace!(path = %path.display(), "writing updated file");
		std::fs::write(path, content)?;
	}
	Ok(())
}

/// Apply a sequence of transformers to content.
#[instrument(skip(content), fields(content_len = content.len(), transformer_count = transformers.len()))]
pub fn apply_transformers(content: &str, transformers: &[Transformer]) -> String {
	apply_transformers_with_data(content, transformers, None)
}

/// Apply a sequence of transformers to content with an optional data context.
/// The data context is used by data-dependent transformers like `if`.
#[allow(clippy::implicit_hasher)]
#[instrument(skip(content, data), fields(content_len = content.len(), transformer_count = transformers.len(), has_data = data.is_some()))]
pub fn apply_transformers_with_data(
	content: &str,
	transformers: &[Transformer],
	data: Option<&HashMap<String, serde_json::Value>>,
) -> String {
	let mut result = content.to_string();

	for transformer in transformers {
		trace!(transformer = ?transformer.r#type, "applying transformer");
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
			// A fence closes at the first backtick run at least as long as the
			// opener, so the fence must outrun any run inside the content.
			let fence = "`".repeat(longest_backtick_run(content).max(2) + 1);
			format!("{fence}{lang}\n{content}\n{fence}")
		}
		TransformerType::Code => {
			// CommonMark code spans close at the first backtick run of the same
			// length, and strip one space of padding when both ends have one.
			let delimiter = "`".repeat(shortest_absent_backtick_run(content));
			if content.starts_with('`') || content.ends_with('`') {
				format!("{delimiter} {content} {delimiter}")
			} else {
				format!("{delimiter}{content}{delimiter}")
			}
		}
		TransformerType::Replace => {
			let search = get_string_arg(&transformer.args, 0).unwrap_or_default();
			if search.is_empty() {
				// `str::replace` with an empty pattern inserts the replacement
				// between every character, which is never what a template means.
				return content.to_string();
			}
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

/// Lengths of every run of consecutive backticks in `content`.
fn backtick_runs(content: &str) -> impl Iterator<Item = usize> + '_ {
	content
		.split(|character| character != '`')
		.map(str::len)
		.filter(|length| *length > 0)
}

fn longest_backtick_run(content: &str) -> usize {
	backtick_runs(content).max().unwrap_or(0)
}

/// The shortest backtick delimiter that does not appear as a run in
/// `content`, so a code span wrapping it cannot close early.
fn shortest_absent_backtick_run(content: &str) -> usize {
	let runs: HashSet<usize> = backtick_runs(content).collect();
	// At most `runs.len()` lengths are taken, so one of the first
	// `runs.len() + 1` is free.
	(1..=runs.len() + 1)
		.find(|length| !runs.contains(length))
		.unwrap_or(1)
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

/// Default padding applied when no `[padding]` section is configured.
///
/// Content starts on the line after the opening tag and the closing tag
/// starts on its own line. Keeping the closing tag off the last content line
/// matters: a trimmed `codeBlock` would otherwise glue the closing tag to its
/// closing fence, which then no longer closes the fence, and a source-file
/// closing tag would lose its comment prefix.
static DEFAULT_PADDING: PaddingConfig = PaddingConfig {
	before: crate::config::PaddingValue::Lines(0),
	after: crate::config::PaddingValue::Lines(0),
};

/// Return the effective padding configuration, defaulting to
/// [`DEFAULT_PADDING`] when no `[padding]` section is configured.
fn effective_padding(ctx: &ProjectContext) -> &PaddingConfig {
	ctx.padding.as_ref().unwrap_or(&DEFAULT_PADDING)
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
///
/// Padding adds to newlines the content already starts or ends with, so
/// untrimmed provider content keeps its own blank lines.
pub(crate) fn pad_content_with_config(
	new_content: &str,
	trailing_prefix: &str,
	padding: &PaddingConfig,
) -> String {
	// The trailing prefix is the comment prefix (e.g., `//! `) that precedes
	// the closing tag in the source file, extracted from the closing tag's
	// line. It is preserved after replacement so the closing tag stays inside
	// the comment. For markdown files it is at most indentation.
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
			// N blank lines between opening tag and content. End the tag line
			// first — with the content's own leading newline when it has one —
			// so the first blank-line prefix never lands on the tag line.
			result.push('\n');
			for _ in 0..n {
				result.push_str(blank_line_prefix);
				result.push('\n');
			}
		}
	}

	let new_content = match padding.before.line_count() {
		Some(n) if n > 0 => new_content.strip_prefix('\n').unwrap_or(new_content),
		_ => new_content,
	};
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

//! Tool implementations.
//!
//! Each function runs on the blocking thread pool against a project root that
//! has already been confined to the server root, and builds the tool's JSON
//! payload. Failures that stop a tool from doing its job are returned as
//! [`ToolError`]s and become `isError` results.

use std::cmp::Reverse;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::path::Path;

use mdt_core::BlockType;
use mdt_core::ExpectedContent;
use mdt_core::MdtError;
use mdt_core::check_project;
use mdt_core::compute_updates;
use mdt_core::expected_consumer_content;
use mdt_core::init::ConfigOutcome;
use mdt_core::init::GitignoreOutcome;
use mdt_core::init::InitReport;
use mdt_core::init::SAMPLE_BLOCK_NAME;
use mdt_core::init::SampleOutcome;
use mdt_core::init::init_project;
use mdt_core::project::ConsumerEntry;
use mdt_core::project::ProjectContext;
use mdt_core::project::ProviderEntry;
use mdt_core::project::ValidationOptions;
use mdt_core::project::is_markdown_path;
use mdt_core::project::normalize_line_endings;
use mdt_core::project::relative_display_path;
use mdt_core::project::scan_project_with_config;
use mdt_core::render_template;
use mdt_core::write_updates;
use rmcp::model::CallToolResult;
use serde::Serialize;
use serde_json::json;

use crate::response::ConsumerInfo;
use crate::response::ConsumerStatus;
use crate::response::ConsumerStatuses;
use crate::response::RenderErrorInfo;
use crate::response::TemplateWarningInfo;
use crate::response::ToolError;
use crate::response::diagnostics;
use crate::response::error_count;
use crate::response::json_result;
use crate::response::render_failure_message;
use crate::response::sort_by_location;
use crate::reuse::MatchKind;
use crate::reuse::NameMatch;
use crate::reuse::content_contains;
use crate::reuse::match_name;

const VALIDATION_FAILED: &str = "mdt::validation";
const BLOCK_NOT_FOUND: &str = "mdt::block_not_found";
const PROVIDER_NOT_FOUND: &str = "mdt::provider_not_found";

// ---------------------------------------------------------------------------
// mdt_check
// ---------------------------------------------------------------------------
/// A consumer whose content differs from what `mdt update` would write.
#[derive(Debug, Serialize)]
struct StaleInfo {
	block_name: String,
	file: String,
	line: usize,
	column: usize,
}

/// A consumer whose name matches no provider.
#[derive(Debug, Serialize)]
struct OrphanInfo {
	block_name: String,
	file: String,
	line: usize,
	column: usize,
	/// Existing provider names close to `block_name`, best match first.
	suggestions: Vec<String>,
}

pub(crate) fn check(root: &Path, options: &ValidationOptions) -> Result<CallToolResult, ToolError> {
	let ctx = scan_project_with_config(root)?;
	let result = check_project(&ctx)?;
	let diagnostics = diagnostics(&ctx, root, options);

	let mut stale: Vec<_> = result
		.stale
		.iter()
		.map(|entry| {
			StaleInfo {
				block_name: entry.block_name.clone(),
				file: relative_display_path(&entry.file, root),
				line: entry.line,
				column: entry.column,
			}
		})
		.collect();
	stale.sort_by(|a, b| (&a.file, a.line, a.column).cmp(&(&b.file, b.line, b.column)));
	let mut stale_files: Vec<_> = result
		.stale_files
		.iter()
		.map(|entry| relative_display_path(&entry.file, root))
		.collect();
	stale_files.sort();
	let orphans: Vec<_> = result
		.orphans
		.iter()
		.map(|orphan| {
			OrphanInfo {
				block_name: orphan.block_name.clone(),
				file: relative_display_path(&orphan.file, root),
				line: orphan.line,
				column: orphan.column,
				suggestions: orphan.suggestions.clone(),
			}
		})
		.collect();
	let render_errors = RenderErrorInfo::list(&result.render_errors, root);
	let mut missing_provider_names = ctx.find_missing_providers();
	missing_provider_names.sort();

	let diagnostic_errors = error_count(&diagnostics);
	let ok = result.is_ok() && diagnostic_errors == 0;
	let summary = if ok {
		"All consumer blocks are up to date.".to_string()
	} else {
		let problems = count_phrases(&[
			(diagnostic_errors, "validation error(s)"),
			(render_errors.len(), "render error(s)"),
			(stale.len(), "stale consumer block(s)"),
			(stale_files.len(), "stale formatter-normalized file(s)"),
			(
				orphans.len(),
				"orphan consumer(s) with no matching provider",
			),
		]);
		format!("Check found {problems}.")
	};

	Ok(json_result(json!({
		"ok": ok,
		"action": "check",
		"summary": summary,
		"stale": stale,
		"stale_files": stale_files,
		"render_errors": render_errors,
		"orphans": orphans,
		"diagnostics": diagnostics,
		"warnings": TemplateWarningInfo::list(&result.warnings, root),
		"missing_provider_names": missing_provider_names,
	})))
}

// ---------------------------------------------------------------------------
// mdt_update
// ---------------------------------------------------------------------------
pub(crate) fn update(
	root: &Path,
	options: &ValidationOptions,
	dry_run: bool,
) -> Result<CallToolResult, ToolError> {
	let ctx = scan_project_with_config(root)?;
	let diagnostics = diagnostics(&ctx, root, options);

	// Like `mdt update`, validation errors stop the update before anything
	// is written.
	let diagnostic_errors = error_count(&diagnostics);

	if diagnostic_errors > 0 {
		return Err(ToolError::new(
			VALIDATION_FAILED,
			format!(
				"Update refused: fix {diagnostic_errors} validation error(s) first (or silence \
				 them with the matching `ignore_*` option). No files were written."
			),
		)
		.with_detail("dry_run", dry_run)
		.with_detail("updated_count", 0)
		.with_detail("updated_files", Vec::<String>::new())
		.with_detail("diagnostics", &diagnostics));
	}

	let updates = compute_updates(&ctx)?;

	if !dry_run && !updates.updated_files.is_empty() {
		write_updates(&updates)?;
	}

	let mut updated_files: Vec<_> = updates
		.updated_files
		.keys()
		.map(|path| relative_display_path(path, root))
		.collect();
	updated_files.sort();
	let render_errors = RenderErrorInfo::list(&updates.render_errors, root);
	let mut missing_provider_names = ctx.find_missing_providers();
	missing_provider_names.sort();

	let outcome = match (updated_files.len(), updates.updated_count, dry_run) {
		(0, ..) => "All consumer blocks are already up to date. No changes needed.".to_string(),
		(files, 0, true) => {
			format!("Dry run: would normalize {files} file(s) via formatter integration.")
		}
		(files, 0, false) => format!("Normalized {files} file(s) via formatter integration."),
		(files, blocks, true) => {
			format!("Dry run: would update {blocks} block(s) in {files} file(s).")
		}
		(files, blocks, false) => format!("Updated {blocks} block(s) in {files} file(s)."),
	};

	let summary = match render_errors.len() {
		0 => outcome,
		failed => {
			format!(
				"{outcome} {failed} consumer block(s) failed to render and were left unchanged."
			)
		}
	};

	Ok(json_result(json!({
		"ok": render_errors.is_empty(),
		"action": "update",
		"dry_run": dry_run,
		"summary": summary,
		"updated_count": updates.updated_count,
		"updated_files": updated_files,
		"render_errors": render_errors,
		"diagnostics": diagnostics,
		"warnings": TemplateWarningInfo::list(&updates.warnings, root),
		"missing_provider_names": missing_provider_names,
	})))
}

// ---------------------------------------------------------------------------
// mdt_list
// ---------------------------------------------------------------------------
/// A provider as `mdt_list` reports it.
#[derive(Debug, Serialize)]
struct ProviderSummary {
	name: String,
	file: String,
	line: usize,
	column: usize,
	consumer_count: usize,
	/// The trimmed provider body, only with `include_content`.
	#[serde(skip_serializing_if = "Option::is_none")]
	content: Option<String>,
}

pub(crate) fn list(
	root: &Path,
	options: &ValidationOptions,
	include_content: bool,
) -> Result<CallToolResult, ToolError> {
	let ctx = scan_project_with_config(root)?;
	let statuses = ConsumerStatuses::new(&check_project(&ctx)?);
	let diagnostics = diagnostics(&ctx, root, options);
	let consumers_by_name = consumers_by_provider_name(&ctx);

	let mut providers: Vec<_> = ctx
		.project
		.providers
		.iter()
		.map(|(name, provider)| {
			ProviderSummary {
				name: name.clone(),
				file: relative_display_path(&provider.file, root),
				line: provider.block.opening.start.line,
				column: provider.block.opening.start.column,
				consumer_count: consumers_by_name.get(name.as_str()).map_or(0, Vec::len),
				content: include_content.then(|| provider.content.trim().to_string()),
			}
		})
		.collect();
	providers.sort_by(|a, b| a.name.cmp(&b.name));

	let mut consumers: Vec<_> = ctx.project.consumers.iter().collect();
	sort_by_location(&mut consumers);
	let consumers: Vec<_> = consumers
		.into_iter()
		.map(|consumer| ConsumerInfo::new(consumer, root, &statuses))
		.collect();

	let stale_count = consumers
		.iter()
		.filter(|consumer| consumer.is_stale)
		.count();
	let diagnostic_errors = error_count(&diagnostics);
	let counts = format!(
		"{} provider(s), {} consumer(s), {stale_count} stale.",
		providers.len(),
		consumers.len()
	);

	let summary = match diagnostic_errors {
		0 => counts,
		errors => format!("{counts} {errors} validation error(s)."),
	};

	Ok(json_result(json!({
		"ok": diagnostic_errors == 0,
		"action": "list",
		"summary": summary,
		"providers": providers,
		"consumers": consumers,
		"diagnostics": diagnostics,
	})))
}

// ---------------------------------------------------------------------------
// mdt_find_reuse
// ---------------------------------------------------------------------------
/// What `mdt_find_reuse` searches for.
pub(crate) struct ReuseQuery<'a> {
	pub block_name: Option<&'a str>,
	pub content: Option<&'a str>,
	pub limit: usize,
}

/// A provider to consider reusing.
#[derive(Debug, Serialize)]
struct ReuseCandidate {
	name: String,
	file: String,
	consumer_count: usize,
	markdown_files: Vec<String>,
	code_files: Vec<String>,
	/// How the provider matched the query; `null` without a query.
	#[serde(rename = "match")]
	match_kind: Option<MatchKind>,
	/// Edit distance between the normalized names, for name matches.
	distance: Option<usize>,
}

pub(crate) fn find_reuse(root: &Path, query: &ReuseQuery<'_>) -> Result<CallToolResult, ToolError> {
	let ctx = scan_project_with_config(root)?;
	let consumers_by_name = consumers_by_provider_name(&ctx);
	let has_query = query.block_name.is_some() || query.content.is_some();

	let mut candidates: Vec<_> = ctx
		.project
		.providers
		.iter()
		.filter_map(|(name, provider)| {
			let name_match = query
				.block_name
				.and_then(|block_name| match_name(block_name, name));
			let content_match = query
				.content
				.is_some_and(|content| content_contains(&provider.content, content));
			let (match_kind, distance) = match (name_match, content_match) {
				(Some(NameMatch { kind, distance }), _) => (Some(kind), Some(distance)),
				(None, true) => (Some(MatchKind::Content), None),
				(None, false) if has_query => return None,
				(None, false) => (None, None),
			};

			let consumers = consumers_by_name
				.get(name.as_str())
				.map_or(&[][..], Vec::as_slice);
			Some(ReuseCandidate {
				name: name.clone(),
				file: relative_display_path(&provider.file, root),
				consumer_count: consumers.len(),
				markdown_files: unique_display_paths(
					consumers
						.iter()
						.copied()
						.filter(|consumer| is_markdown_path(&consumer.file)),
					root,
				),
				code_files: unique_display_paths(
					consumers
						.iter()
						.copied()
						.filter(|consumer| !is_markdown_path(&consumer.file)),
					root,
				),
				match_kind,
				distance,
			})
		})
		.collect();

	candidates.sort_by(|a, b| {
		(a.match_kind, a.distance, Reverse(a.consumer_count), &a.name).cmp(&(
			b.match_kind,
			b.distance,
			Reverse(b.consumer_count),
			&b.name,
		))
	});
	candidates.truncate(query.limit);

	let summary = match (has_query, candidates.len()) {
		(false, count) => format!("{count} provider(s), most consumed first."),
		(true, 0) => "No existing provider matches; create a new provider.".to_string(),
		(true, count) => format!("Found {count} matching provider(s), best match first."),
	};

	Ok(json_result(json!({
		"ok": true,
		"action": "find_reuse",
		"summary": summary,
		"query": query.block_name,
		"content_query": query.content,
		"guidance": "Prefer reusing an existing provider when semantics match. Candidates show \
					 where blocks are already consumed in markdown and source files.",
		"candidates": candidates,
		"next_steps": [
			"If a candidate already matches your intent, reuse that block name in new consumers.",
			"If no candidate fits, create a new provider in .templates/ (or another configured \
			 template path)."
		],
	})))
}

/// The sorted, deduplicated files of `consumers`, relative to `root`.
fn unique_display_paths<'a>(
	consumers: impl Iterator<Item = &'a ConsumerEntry>,
	root: &Path,
) -> Vec<String> {
	let mut paths: Vec<_> = consumers
		.map(|consumer| relative_display_path(&consumer.file, root))
		.collect();
	paths.sort();
	paths.dedup();
	paths
}

// ---------------------------------------------------------------------------
// mdt_get_block and mdt_preview
// ---------------------------------------------------------------------------
/// A provider as `mdt_get_block` and `mdt_preview` report it.
#[derive(Debug, Serialize)]
struct ProviderDetail {
	name: String,
	file: String,
	line: usize,
	column: usize,
	/// Parameter names declared on the provider tag.
	parameters: Vec<String>,
	consumer_count: usize,
	raw_content: String,
	/// The body rendered with the project's `[data]`; `null` when rendering
	/// fails.
	rendered_with_project_data: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	render_error: Option<String>,
}

impl ProviderDetail {
	fn new(ctx: &ProjectContext, provider: &ProviderEntry, root: &Path) -> Self {
		let (rendered_with_project_data, render_error) =
			match render_template(&provider.content, &ctx.data) {
				Ok(rendered) => (Some(rendered), None),
				Err(error) => (None, Some(render_failure_message(error))),
			};
		Self {
			name: provider.block.name.clone(),
			file: relative_display_path(&provider.file, root),
			line: provider.block.opening.start.line,
			column: provider.block.opening.start.column,
			parameters: provider.block.arguments.clone(),
			consumer_count: ctx
				.project
				.consumers
				.iter()
				.filter(|consumer| {
					consumer.block.r#type == BlockType::Consumer
						&& consumer.block.name == provider.block.name
				})
				.count(),
			raw_content: provider.content.clone(),
			rendered_with_project_data,
			render_error,
		}
	}
}

/// A block with its current content, for `mdt_get_block`.
#[derive(Debug, Serialize)]
struct BlockDetail {
	#[serde(flatten)]
	info: ConsumerInfo,
	current_content: String,
}

/// A consumer next to what `mdt update` would write into it, for
/// `mdt_preview`.
#[derive(Debug, Serialize)]
struct ConsumerPreview {
	#[serde(flatten)]
	info: ConsumerInfo,
	current_content: String,
	/// The content after data, arguments, transformers, and `[padding]`,
	/// before any `[[formatters]]` run; `null` when rendering fails.
	rendered_content: Option<String>,
}

pub(crate) fn get_block(root: &Path, block_name: &str) -> Result<CallToolResult, ToolError> {
	let ctx = scan_project_with_config(root)?;
	let provider = ctx.project.providers.get(block_name);
	let mut blocks: Vec<_> = ctx
		.project
		.consumers
		.iter()
		.filter(|consumer| consumer.block.name == block_name)
		.collect();

	if provider.is_none() && blocks.is_empty() {
		return Err(ToolError::new(
			BLOCK_NOT_FOUND,
			format!("No block named `{block_name}` found in the project."),
		)
		.with_detail("block_name", block_name));
	}

	let statuses = ConsumerStatuses::new(&check_project(&ctx)?);
	sort_by_location(&mut blocks);
	let provider = provider.map(|provider| ProviderDetail::new(&ctx, provider, root));
	let consumers: Vec<_> = blocks
		.into_iter()
		.map(|consumer| {
			BlockDetail {
				info: ConsumerInfo::new(consumer, root, &statuses),
				current_content: consumer.content.clone(),
			}
		})
		.collect();

	let infos: Vec<_> = consumers.iter().map(|consumer| &consumer.info).collect();
	let provider_error = provider
		.as_ref()
		.and_then(|provider| provider.render_error.as_deref());
	let (ok, summary) = block_summary(block_name, provider.is_some(), provider_error, &infos);

	Ok(json_result(json!({
		"ok": ok,
		"action": "get_block",
		"summary": summary,
		"block_name": block_name,
		"provider": provider,
		"consumers": consumers,
	})))
}

pub(crate) fn preview(root: &Path, block_name: &str) -> Result<CallToolResult, ToolError> {
	let ctx = scan_project_with_config(root)?;
	let Some(provider) = ctx.project.providers.get(block_name) else {
		return Err(ToolError::new(
			PROVIDER_NOT_FOUND,
			format!("No provider named `{block_name}` found."),
		)
		.with_detail("block_name", block_name));
	};

	let statuses = ConsumerStatuses::new(&check_project(&ctx)?);
	let mut consumers: Vec<_> = ctx
		.project
		.consumers
		.iter()
		.filter(|consumer| {
			consumer.block.r#type == BlockType::Consumer && consumer.block.name == block_name
		})
		.collect();
	sort_by_location(&mut consumers);

	let mut sources: HashMap<&Path, String> = HashMap::new();
	let mut previews = Vec::with_capacity(consumers.len());

	for consumer in consumers {
		let source = match sources.entry(consumer.file.as_path()) {
			Entry::Occupied(entry) => entry.into_mut(),
			Entry::Vacant(entry) => entry.insert(read_source(&consumer.file, root)?),
		};

		let rendered_content = match expected_consumer_content(&ctx, consumer, source) {
			ExpectedContent::Rendered(content) => Some(content),
			_ => None,
		};

		previews.push(ConsumerPreview {
			info: ConsumerInfo::new(consumer, root, &statuses),
			current_content: consumer.content.clone(),
			rendered_content,
		});
	}

	let provider = ProviderDetail::new(&ctx, provider, root);
	let infos: Vec<_> = previews.iter().map(|preview| &preview.info).collect();
	let (ok, summary) = block_summary(block_name, true, provider.render_error.as_deref(), &infos);

	Ok(json_result(json!({
		"ok": ok,
		"action": "preview",
		"summary": summary,
		"block_name": block_name,
		"provider": provider,
		"consumers": previews,
	})))
}

/// Whether a block renders cleanly for every consumer, and a summary
/// describing it.
fn block_summary(
	block_name: &str,
	has_provider: bool,
	provider_error: Option<&str>,
	consumers: &[&ConsumerInfo],
) -> (bool, String) {
	let count = |status: ConsumerStatus| {
		consumers
			.iter()
			.filter(|consumer| consumer.status == status)
			.count()
	};
	let render_errors = count(ConsumerStatus::RenderError);
	let orphans = count(ConsumerStatus::Orphan);

	let mut sentences = vec![if has_provider {
		format!(
			"Provider `{block_name}` has {} consumer(s), {} stale.",
			consumers.len(),
			count(ConsumerStatus::Stale)
		)
	} else {
		format!(
			"No provider named `{block_name}`; {orphans} consumer(s) reference it and cannot be \
			 synced."
		)
	}];

	if let Some(error) = provider_error {
		sentences.push(format!("The provider fails to render: {error}"));
	}

	if render_errors > 0 {
		sentences.push(format!("{render_errors} consumer block(s) fail to render."));
	}

	let ok = provider_error.is_none() && render_errors == 0 && orphans == 0;
	(ok, sentences.join(" "))
}

fn read_source(file: &Path, root: &Path) -> Result<String, ToolError> {
	let content = std::fs::read_to_string(file).map_err(|error| {
		MdtError::ReadFile {
			path: relative_display_path(file, root),
			reason: error.to_string(),
		}
	})?;
	Ok(normalize_line_endings(&content))
}

// ---------------------------------------------------------------------------
// mdt_init
// ---------------------------------------------------------------------------
/// Initialize `root` with [`init_project`], the implementation behind
/// `mdt init`. Paths in the payload are relative to `root`; `root` itself is
/// shown relative to the server root `base`.
pub(crate) fn init(base: &Path, root: &Path) -> Result<CallToolResult, ToolError> {
	let report = init_project(root)?;
	let rel = |path: &Path| relative_display_path(path, root);

	let config = match &report.config {
		ConfigOutcome::Created(file) => json!({ "status": "created", "file": rel(file) }),
		ConfigOutcome::Exists(file) => json!({ "status": "exists", "file": rel(file) }),
		// `ConfigOutcome` is non-exhaustive.
		_ => json!({ "status": "unknown" }),
	};

	let sample = match &report.sample {
		SampleOutcome::CreatedWithReadme { template, readme } => {
			json!({
				"status": "created_with_readme",
				"template": rel(template),
				"readme": rel(readme),
			})
		}
		SampleOutcome::CreatedWithoutConsumer { template, readme } => {
			json!({
				"status": "created_without_consumer",
				"template": rel(template),
				"readme": rel(readme),
			})
		}
		SampleOutcome::TemplateExists { template } => {
			json!({ "status": "template_exists", "template": rel(template) })
		}
		SampleOutcome::ProvidersExist { count } => {
			json!({ "status": "providers_exist", "provider_count": count })
		}
		// `SampleOutcome` is non-exhaustive.
		_ => json!({ "status": "unknown" }),
	};

	let gitignore = match &report.gitignore {
		GitignoreOutcome::Updated(file) => json!({ "status": "updated", "file": rel(file) }),
		GitignoreOutcome::Created(file) => json!({ "status": "created", "file": rel(file) }),
		GitignoreOutcome::AlreadyIgnored(file) => {
			json!({ "status": "already_ignored", "file": rel(file) })
		}
		GitignoreOutcome::NotApplicable => json!({ "status": "not_applicable" }),
		// `GitignoreOutcome` is non-exhaustive.
		_ => json!({ "status": "unknown" }),
	};

	let written_files: Vec<_> = report.written_files().into_iter().map(rel).collect();

	let root_display = match relative_display_path(root, base) {
		relative if relative.is_empty() => ".".to_string(),
		relative => relative,
	};
	let summary = if written_files.is_empty() {
		format!("mdt is already set up in `{root_display}`; nothing was written.")
	} else {
		format!(
			"Initialized mdt in `{root_display}`: wrote {}.",
			written_files.join(", ")
		)
	};

	Ok(json_result(json!({
		"ok": true,
		"action": "init",
		"summary": summary,
		"root": root_display,
		"created_root": report.created_root,
		"config": config,
		"sample": sample,
		"gitignore": gitignore,
		"written_files": written_files,
		"next_steps": init_next_steps(&report, root),
	})))
}

fn init_next_steps(report: &InitReport, root: &Path) -> Vec<String> {
	let rel = |path: &Path| relative_display_path(path, root);

	match &report.sample {
		SampleOutcome::CreatedWithReadme { template, readme } => {
			vec![
				format!("Open {} to see the synced sample block.", rel(readme)),
				format!("Edit {}, then run mdt_update.", rel(template)),
				"Run mdt_check (or `mdt check` in CI) to catch stale docs.".to_string(),
			]
		}
		SampleOutcome::CreatedWithoutConsumer { template, readme } => {
			// The tags are assembled at runtime: literal tag text in this file
			// would be scanned as a live block by the repository's own
			// `mdt check`.
			let name = SAMPLE_BLOCK_NAME;
			vec![
				format!(
					"Add a consumer to {}: <!-- {{={name}}} --> <!-- {{/{name}}} -->",
					rel(readme)
				),
				format!(
					"Run mdt_update to fill it in (until then mdt_check warns that `{name}` has \
					 no consumers)."
				),
				format!(
					"Replace the sample in {} with your own providers.",
					rel(template)
				),
			]
		}
		_ => {
			vec![
				"Run mdt_list to see the existing providers and consumers.".to_string(),
				"Run mdt_check to verify every consumer is in sync.".to_string(),
			]
		}
	}
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------
/// Consumer blocks (not inline blocks) grouped by the provider name they
/// reference.
fn consumers_by_provider_name(ctx: &ProjectContext) -> HashMap<&str, Vec<&ConsumerEntry>> {
	let mut grouped: HashMap<&str, Vec<&ConsumerEntry>> = HashMap::new();

	for consumer in &ctx.project.consumers {
		if consumer.block.r#type == BlockType::Consumer {
			grouped
				.entry(consumer.block.name.as_str())
				.or_default()
				.push(consumer);
		}
	}

	grouped
}

/// Join the non-zero counts as `"2 stale consumer block(s), 1 render
/// error(s)"`.
fn count_phrases(counts: &[(usize, &str)]) -> String {
	counts
		.iter()
		.filter(|(count, _)| *count > 0)
		.map(|(count, label)| format!("{count} {label}"))
		.collect::<Vec<_>>()
		.join(", ")
}

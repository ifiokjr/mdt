//! JSON payloads shared by the tools.
//!
//! Every tool answers with a JSON object carrying `ok`, `action`, and
//! `summary`, sent both as text and as structured content. A tool that could
//! not perform its action answers with an `isError` result whose payload adds
//! `error: { code, message, help? }`, so clients that hide JSON-RPC errors from
//! the model still show it what went wrong.

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use mdt_core::BlockType;
use mdt_core::CheckResult;
use mdt_core::MdtError;
use mdt_core::project::ConsumerEntry;
use mdt_core::project::ProjectContext;
use mdt_core::project::ValidationOptions;
use mdt_core::project::relative_display_path;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

/// A failure that stopped a tool from performing its action.
#[derive(Debug, Serialize)]
pub(crate) struct ToolError {
	/// Stable machine-readable code, such as `mdt::config_parse`.
	pub code: String,
	pub message: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub help: Option<String>,
	/// Extra top-level fields for the error payload, such as the requested
	/// `block_name`.
	#[serde(skip)]
	details: Map<String, Value>,
}

impl ToolError {
	pub(crate) fn new(code: &str, message: impl Into<String>) -> Self {
		Self {
			code: code.to_string(),
			message: message.into(),
			help: None,
			details: Map::new(),
		}
	}

	/// Add a top-level field to the error payload.
	pub(crate) fn with_detail(mut self, key: &str, value: impl Serialize) -> Self {
		self.details.insert(key.to_string(), to_json(&value));
		self
	}

	/// The `isError` tool result reporting this failure for `action`.
	pub(crate) fn into_result(self, action: &str) -> CallToolResult {
		let mut payload = Map::new();
		payload.insert("ok".to_string(), Value::Bool(false));
		payload.insert("action".to_string(), Value::from(action));
		payload.insert("summary".to_string(), Value::from(self.message.clone()));
		payload.insert("error".to_string(), to_json(&self));
		payload.extend(self.details);
		tool_result(Value::Object(payload), true)
	}
}

impl From<MdtError> for ToolError {
	fn from(error: MdtError) -> Self {
		use miette::Diagnostic as _;

		Self {
			code: error
				.code()
				.map_or_else(|| "mdt::error".to_string(), |code| code.to_string()),
			help: error.help().map(|help| help.to_string()),
			message: error.to_string(),
			details: Map::new(),
		}
	}
}

/// A successful tool result whose text and structured content are `payload`.
pub(crate) fn json_result(payload: Value) -> CallToolResult {
	tool_result(payload, false)
}

fn tool_result(payload: Value, is_error: bool) -> CallToolResult {
	let text = serde_json::to_string_pretty(&payload)
		.unwrap_or_else(|error| panic!("a serde_json::Value always serializes: {error}"));
	let content = vec![ContentBlock::text(text)];
	let mut result = if is_error {
		CallToolResult::error(content)
	} else {
		CallToolResult::success(content)
	};

	result.structured_content = Some(payload);
	result
}

fn to_json(value: &impl Serialize) -> Value {
	serde_json::to_value(value)
		.unwrap_or_else(|error| panic!("tool payloads always serialize: {error}"))
}

/// A parser or validation diagnostic, with the severity `mdt` gives it.
#[derive(Debug, Serialize)]
pub(crate) struct DiagnosticInfo {
	pub kind: &'static str,
	/// The same code `mdt check --format json` reports, e.g.
	/// `mdt::unclosed_block`.
	pub code: &'static str,
	pub severity: Severity,
	pub file: String,
	pub line: usize,
	pub column: usize,
	pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Severity {
	/// Stops `mdt check`, `mdt update`, and `mdt list`.
	Error,
	Warning,
}

/// The project's diagnostics that `options` does not silence, in source
/// order.
pub(crate) fn diagnostics(
	ctx: &ProjectContext,
	root: &Path,
	options: &ValidationOptions,
) -> Vec<DiagnosticInfo> {
	let mut diagnostics: Vec<_> = ctx
		.project
		.diagnostics
		.iter()
		.filter(|diagnostic| !diagnostic.is_ignored(options))
		.map(|diagnostic| {
			DiagnosticInfo {
				kind: diagnostic.kind.code().trim_start_matches("mdt::"),
				code: diagnostic.kind.code(),
				severity: if diagnostic.is_error(options) {
					Severity::Error
				} else {
					Severity::Warning
				},

				file: relative_display_path(&diagnostic.file, root),
				line: diagnostic.line,
				column: diagnostic.column,
				message: diagnostic.message(),
			}
		})
		.collect();
	diagnostics.sort_by(|a, b| (&a.file, a.line, a.column).cmp(&(&b.file, b.line, b.column)));
	diagnostics
}

pub(crate) fn error_count(diagnostics: &[DiagnosticInfo]) -> usize {
	diagnostics
		.iter()
		.filter(|diagnostic| diagnostic.severity == Severity::Error)
		.count()
}

/// How a consumer or inline block compares with what `mdt update` would
/// write, as `mdt check` decides it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ConsumerStatus {
	Current,
	Stale,
	/// The provider (or inline template) failed to render for this block.
	RenderError,
	/// No provider has this block's name.
	Orphan,
}

/// Per-block outcome of [`mdt_core::check_project`], so every tool reports
/// the same staleness as `mdt check`, including `[padding]`, lenient
/// comparison, and `[[formatters]]`.
pub(crate) struct ConsumerStatuses {
	/// Keyed by file, then by the opening tag's line and column. Blocks
	/// without an entry are current.
	by_file: HashMap<PathBuf, StatusesByPosition>,
}

/// Status and render-failure reason, keyed by opening tag line and column.
type StatusesByPosition = HashMap<(usize, usize), (ConsumerStatus, Option<String>)>;

impl ConsumerStatuses {
	pub(crate) fn new(result: &CheckResult) -> Self {
		let mut statuses = Self {
			by_file: HashMap::new(),
		};

		for entry in &result.stale {
			statuses.insert(
				&entry.file,
				entry.line,
				entry.column,
				ConsumerStatus::Stale,
				None,
			);
		}

		for error in &result.render_errors {
			statuses.insert(
				&error.file,
				error.line,
				error.column,
				ConsumerStatus::RenderError,
				Some(error.message.clone()),
			);
		}

		for orphan in &result.orphans {
			statuses.insert(
				&orphan.file,
				orphan.line,
				orphan.column,
				ConsumerStatus::Orphan,
				None,
			);
		}

		statuses
	}

	fn insert(
		&mut self,
		file: &Path,
		line: usize,
		column: usize,
		status: ConsumerStatus,
		message: Option<String>,
	) {
		self.by_file
			.entry(file.to_path_buf())
			.or_default()
			.insert((line, column), (status, message));
	}

	/// The status of `consumer` and, for render failures, the reason.
	pub(crate) fn of(&self, consumer: &ConsumerEntry) -> (ConsumerStatus, Option<&str>) {
		let start = &consumer.block.opening.start;
		self.by_file
			.get(consumer.file.as_path())
			.and_then(|by_position| by_position.get(&(start.line, start.column)))
			.map_or((ConsumerStatus::Current, None), |(status, message)| {
				(*status, message.as_deref())
			})
	}
}

/// A consumer or inline block, as every tool reports it.
#[derive(Debug, Serialize)]
pub(crate) struct ConsumerInfo {
	/// `consumer` (`{=name}`) or `inline` (`{~name:"template"}`).
	#[serde(rename = "type")]
	pub kind: &'static str,
	pub name: String,
	pub file: String,
	pub line: usize,
	pub column: usize,
	pub transformers: Vec<String>,
	pub arguments: Vec<String>,
	pub status: ConsumerStatus,
	/// Shorthand for `status == "stale"`.
	pub is_stale: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub render_error: Option<String>,
}

impl ConsumerInfo {
	pub(crate) fn new(consumer: &ConsumerEntry, root: &Path, statuses: &ConsumerStatuses) -> Self {
		let (status, render_error) = statuses.of(consumer);
		Self {
			kind: block_kind(consumer.block.r#type),
			name: consumer.block.name.clone(),
			file: relative_display_path(&consumer.file, root),
			line: consumer.block.opening.start.line,
			column: consumer.block.opening.start.column,
			transformers: consumer
				.block
				.transformers
				.iter()
				.map(|transformer| transformer.r#type.to_string())
				.collect(),
			arguments: consumer.block.arguments.clone(),
			status,
			is_stale: status == ConsumerStatus::Stale,
			render_error: render_error.map(str::to_string),
		}
	}
}

fn block_kind(block_type: BlockType) -> &'static str {
	match block_type {
		BlockType::Consumer => "consumer",
		BlockType::Inline => "inline",
		BlockType::Provider => "provider",
		// `BlockType` is non-exhaustive.
		_ => "unknown",
	}
}

/// Sort consumers into source order: by file, then position.
pub(crate) fn sort_by_location(consumers: &mut [&ConsumerEntry]) {
	consumers.sort_by(|a, b| {
		let a_start = &a.block.opening.start;
		let b_start = &b.block.opening.start;
		(&a.file, a_start.line, a_start.column).cmp(&(&b.file, b_start.line, b_start.column))
	});
}

/// A template render failure reported by `mdt check` or `mdt update`.
#[derive(Debug, Serialize)]
pub(crate) struct RenderErrorInfo {
	pub block_name: String,
	pub file: String,
	pub line: usize,
	pub column: usize,
	pub message: String,
}

impl RenderErrorInfo {
	pub(crate) fn list(errors: &[mdt_core::RenderError], root: &Path) -> Vec<Self> {
		let mut infos: Vec<_> = errors
			.iter()
			.map(|error| {
				Self {
					block_name: error.block_name.clone(),
					file: relative_display_path(&error.file, root),
					line: error.line,
					column: error.column,
					message: error.message.clone(),
				}
			})
			.collect();
		infos.sort_by(|a, b| (&a.file, a.line, a.column).cmp(&(&b.file, b.line, b.column)));
		infos
	}
}

/// Undefined template variables in a provider.
#[derive(Debug, Serialize)]
pub(crate) struct TemplateWarningInfo {
	pub block_name: String,
	pub file: String,
	pub undefined_variables: Vec<String>,
	/// False when the project has no `[data]`, so the provider's
	/// `{{ ... }}` text is copied to consumers without rendering.
	pub template_rendered: bool,
}

impl TemplateWarningInfo {
	pub(crate) fn list(warnings: &[mdt_core::TemplateWarning], root: &Path) -> Vec<Self> {
		let mut infos: Vec<_> = warnings
			.iter()
			.map(|warning| {
				let mut undefined_variables = warning.undefined_variables.clone();
				undefined_variables.sort();
				Self {
					block_name: warning.block_name.clone(),
					file: relative_display_path(&warning.provider_file, root),
					undefined_variables,
					template_rendered: warning.template_rendered,
				}
			})
			.collect();
		infos.sort_by(|a, b| (&a.file, &a.block_name).cmp(&(&b.file, &b.block_name)));
		infos
	}
}

/// The engine's explanation of a render failure, without the "template
/// rendering failed" prefix, matching [`mdt_core::RenderError::message`].
pub(crate) fn render_failure_message(error: MdtError) -> String {
	match error {
		MdtError::TemplateRender(message) => message,
		other => other.to_string(),
	}
}

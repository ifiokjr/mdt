//! <!-- {=mdtMcpOverview|trim|linePrefix:"//! ":true} -->
//! `mdt_mcp` is a [Model Context Protocol](https://modelcontextprotocol.io/) (MCP) server for the [mdt](https://github.com/ifiokjr/mdt) template engine. It exposes mdt functionality as MCP tools that can be used by AI assistants and other MCP-compatible clients.
//!
//! ### Tools
//!
//! - **`mdt_check`** — Verify all consumer blocks are up-to-date.
//! - **`mdt_update`** — Update all consumer blocks with latest provider content.
//! - **`mdt_list`** — List all providers and consumers in the project.
//! - **`mdt_find_reuse`** — Find similar providers and where they are already consumed in markdown and source files to encourage reuse.
//! - **`mdt_get_block`** — Get the content of a specific block by name.
//! - **`mdt_preview`** — Preview the result of applying transformers to a block.
//! - **`mdt_init`** — Initialize a new mdt project with a sample `.templates/template.t.md` file and starter `mdt.toml`.
//!
//! ### Agent Workflow
//!
//! - Prefer reuse before creation: call `mdt_find_reuse` (or `mdt_list`) before introducing a new provider block.
//! - Use the JSON-first tool responses as the source of truth. The MCP server returns structured payloads so agents can inspect results without parsing prose.
//! - Use `mdt_preview` as an authoring workflow: inspect the provider template plus each consumer's rendered output before deciding whether to reuse, edit, or sync.
//! - Keep provider names global and unique in the project to avoid collisions.
//! - After edits, run `mdt_check` (and optionally `mdt_update`) so consumer blocks stay synchronized.
//!
//! ### Usage
//!
//! Start the MCP server via the CLI:
//!
//! ```sh
//! mdt mcp
//! ```
//!
//! Add the following to your MCP client configuration:
//!
//! ```json
//! {
//! 	"mcpServers": {
//! 		"mdt": {
//! 			"command": "mdt",
//! 			"args": ["mcp"]
//! 		}
//! 	}
//! }
//! ```
//! <!-- {/mdtMcpOverview} -->

use std::path::Path;

use mdt_core::BlockType;
use mdt_core::MdtConfig;
use mdt_core::apply_transformers;
use mdt_core::build_render_context;
use mdt_core::check_project;
use mdt_core::compute_updates;
use mdt_core::project::ProjectContext;
use mdt_core::project::is_markdown_path;
use mdt_core::project::levenshtein_distance;
use mdt_core::project::relative_display_path;
use mdt_core::project::resolve_root;
use mdt_core::project::scan_project_with_config;
use mdt_core::render_template;
use mdt_core::write_updates;
use rmcp::ErrorData as McpError;
use rmcp::ServerHandler;
use rmcp::ServiceExt;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::schemars;
use rmcp::serde;
use rmcp::tool;
use rmcp::tool_handler;
use rmcp::tool_router;
use serde::Deserialize;
use serde::Serialize;

/// Parameters for tools that accept an optional project path.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PathParam {
	/// Path to the project root directory. Defaults to the current directory.
	pub path: Option<String>,
}

/// Parameters for tools that need a block name.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BlockParam {
	/// Path to the project root directory. Defaults to the current directory.
	pub path: Option<String>,
	/// The name of the block to look up.
	pub block_name: String,
}

/// Parameters for the update tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateParam {
	/// Path to the project root directory. Defaults to the current directory.
	pub path: Option<String>,
	/// If true, show what would change without writing files.
	#[serde(default)]
	pub dry_run: bool,
}

/// Parameters for the init tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct InitParam {
	/// Path to the project root directory. Defaults to the current directory.
	pub path: Option<String>,
}

/// Parameters for reuse discovery.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReuseParam {
	/// Path to the project root directory. Defaults to the current directory.
	pub path: Option<String>,
	/// Optional proposed block name to match against existing providers.
	pub block_name: Option<String>,
	/// Max number of suggested providers to return.
	#[serde(default = "default_reuse_limit")]
	pub limit: usize,
}

/// A provider info entry for JSON output.
#[derive(Debug, Serialize)]
struct ProviderInfo {
	name: String,
	file: String,
	content: String,
	consumer_count: usize,
}

/// A consumer info entry for JSON output.
#[derive(Debug, Serialize)]
struct ConsumerInfo {
	name: String,
	file: String,
	transformers: Vec<String>,
	is_stale: bool,
}

/// A candidate provider to reuse.
#[derive(Debug, Serialize)]
struct ReuseCandidate {
	name: String,
	file: String,
	consumer_count: usize,
	markdown_files: Vec<String>,
	code_files: Vec<String>,
	distance: Option<usize>,
}

#[derive(Debug, Serialize)]
struct TemplateWarningInfo {
	block_name: String,
	file: String,
	undefined_variables: Vec<String>,
}

#[derive(Debug, Serialize)]
struct RenderErrorInfo {
	block_name: String,
	file: String,
	line: usize,
	column: usize,
	message: String,
}

#[derive(Debug, Serialize)]
struct StaleEntryInfo {
	block_name: String,
	file: String,
	line: usize,
	column: usize,
}

#[derive(Debug, Serialize)]
struct PreviewProviderInfo {
	name: String,
	file: String,
	raw_content: String,
	rendered_with_project_data: String,
	parameters: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PreviewConsumerInfo {
	file: String,
	transformers: Vec<String>,
	arguments: Vec<String>,
	rendered_content: Option<String>,
	current_content: String,
	is_stale: Option<bool>,
	render_error: Option<String>,
}

/// The MCP server for mdt.
#[derive(Debug, Clone)]
pub struct MdtMcpServer {
	tool_router: ToolRouter<Self>,
}

#[tool_handler]
impl ServerHandler for MdtMcpServer {
	fn get_info(&self) -> ServerInfo {
		ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
			"mdt (manage markdown templates) keeps documentation synchronized across your \
			 project using comment-based template tags. MCP tool responses are JSON-first and \
			 include structured content for agent use. Use these tools to check, update, \
			 list, preview, and find reusable blocks. Before creating a new provider, run \
			 mdt_find_reuse or mdt_list to discover similar block names and existing \
			 markdown/source consumers. Use mdt_preview as an authoring workflow to inspect \
			 provider templates plus per-consumer rendered output. Prefer reuse over new \
			 provider names when possible, then run mdt_check (and mdt_update if needed) to \
			 keep consumers synchronized.",
		)
	}
}

fn default_reuse_limit() -> usize {
	5
}

fn scan_ctx(root: &Path) -> Result<ProjectContext, McpError> {
	scan_project_with_config(root).map_err(|e| McpError::internal_error(e.to_string(), None))
}

fn json_result(value: serde_json::Value) -> CallToolResult {
	let text = serde_json::to_string_pretty(&value)
		.unwrap_or_else(|_| "{\"ok\":false,\"summary\":\"failed to serialize\"}".to_string());
	let mut result = CallToolResult::success(vec![Content::text(text)]);
	result.structured_content = Some(value);
	result
}

fn json_error_result(value: serde_json::Value) -> CallToolResult {
	let text = serde_json::to_string_pretty(&value)
		.unwrap_or_else(|_| "{\"ok\":false,\"summary\":\"failed to serialize\"}".to_string());
	let mut result = CallToolResult::error(vec![Content::text(text)]);
	result.structured_content = Some(value);
	result
}

fn warning_info(warning: &mdt_core::TemplateWarning, root: &Path) -> TemplateWarningInfo {
	let mut undefined_variables = warning.undefined_variables.clone();
	undefined_variables.sort();
	TemplateWarningInfo {
		block_name: warning.block_name.clone(),
		file: relative_display_path(&warning.provider_file, root),
		undefined_variables,
	}
}

#[tool_router]
impl MdtMcpServer {
	pub fn new() -> Self {
		Self {
			tool_router: Self::tool_router(),
		}
	}

	#[tool(
		name = "mdt_check",
		description = "Check if all consumer blocks are up to date. Returns a JSON-first summary \
		               of stale blocks, render errors, missing providers, and authoring warnings."
	)]
	async fn check(
		&self,
		Parameters(params): Parameters<PathParam>,
	) -> Result<CallToolResult, McpError> {
		let root = resolve_root(params.path.as_deref().map(Path::new));
		let ctx = scan_ctx(&root)?;

		let mut missing = ctx.find_missing_providers();
		missing.sort();
		let result =
			check_project(&ctx).map_err(|e| McpError::internal_error(e.to_string(), None))?;

		let warnings: Vec<_> = result
			.warnings
			.iter()
			.map(|warning| warning_info(warning, &root))
			.collect();
		let render_errors: Vec<_> = result
			.render_errors
			.iter()
			.map(|err| {
				RenderErrorInfo {
					block_name: err.block_name.clone(),
					file: relative_display_path(&err.file, &root),
					line: err.line,
					column: err.column,
					message: err.message.clone(),
				}
			})
			.collect();
		let stale: Vec<_> = result
			.stale
			.iter()
			.map(|entry| {
				StaleEntryInfo {
					block_name: entry.block_name.clone(),
					file: relative_display_path(&entry.file, &root),
					line: entry.line,
					column: entry.column,
				}
			})
			.collect();

		let ok = result.is_ok() && missing.is_empty();
		let summary = if ok {
			"All consumer blocks are up to date.".to_string()
		} else {
			let mut parts = Vec::new();
			if !render_errors.is_empty() {
				parts.push(format!("{} render error(s)", render_errors.len()));
			}
			if !stale.is_empty() {
				parts.push(format!("{} stale consumer block(s)", stale.len()));
			}
			if !missing.is_empty() {
				parts.push(format!("{} missing provider name(s)", missing.len()));
			}
			if parts.is_empty() {
				"Check completed with warnings.".to_string()
			} else {
				format!("Check found {}.", parts.join(", "))
			}
		};

		Ok(json_result(serde_json::json!({
			"ok": ok,
			"action": "check",
			"summary": summary,
			"stale": stale,
			"render_errors": render_errors,
			"warnings": warnings,
			"missing_provider_names": missing,
		})))
	}

	#[tool(
		name = "mdt_update",
		description = "Update all stale consumer blocks with latest provider content. Returns a \
		               JSON-first summary and supports dry_run mode to preview changes without \
		               writing."
	)]
	async fn update(
		&self,
		Parameters(params): Parameters<UpdateParam>,
	) -> Result<CallToolResult, McpError> {
		let root = resolve_root(params.path.as_deref().map(Path::new));
		let ctx = scan_ctx(&root)?;
		let mut missing_provider_names = ctx.find_missing_providers();
		missing_provider_names.sort();
		let updates =
			compute_updates(&ctx).map_err(|e| McpError::internal_error(e.to_string(), None))?;
		let warnings: Vec<_> = updates
			.warnings
			.iter()
			.map(|warning| warning_info(warning, &root))
			.collect();
		let mut updated_files: Vec<String> = updates
			.updated_files
			.keys()
			.map(|path| relative_display_path(path, &root))
			.collect();
		updated_files.sort();

		if updates.updated_count == 0 {
			return Ok(json_result(serde_json::json!({
				"ok": true,
				"action": "update",
				"dry_run": params.dry_run,
				"summary": "All consumer blocks are already up to date. No changes needed.",
				"updated_count": 0,
				"updated_files": updated_files,
				"warnings": warnings,
				"missing_provider_names": missing_provider_names,
			})));
		}

		if !params.dry_run {
			write_updates(&updates).map_err(|e| McpError::internal_error(e.to_string(), None))?;
		}

		let summary = if params.dry_run {
			format!(
				"Dry run: would update {} block(s) in {} file(s).",
				updates.updated_count,
				updated_files.len()
			)
		} else {
			format!(
				"Updated {} block(s) in {} file(s).",
				updates.updated_count,
				updated_files.len()
			)
		};

		Ok(json_result(serde_json::json!({
			"ok": true,
			"action": "update",
			"dry_run": params.dry_run,
			"summary": summary,
			"updated_count": updates.updated_count,
			"updated_files": updated_files,
			"warnings": warnings,
			"missing_provider_names": missing_provider_names,
		})))
	}

	#[tool(
		name = "mdt_list",
		description = "List all provider and consumer blocks with their names, source files, \
		               consumer counts, transformers, and staleness status."
	)]
	async fn list(
		&self,
		Parameters(params): Parameters<PathParam>,
	) -> Result<CallToolResult, McpError> {
		let root = resolve_root(params.path.as_deref().map(Path::new));
		let ctx = scan_ctx(&root)?;

		let mut providers: Vec<ProviderInfo> = ctx
			.project
			.providers
			.iter()
			.map(|(name, entry)| {
				let consumer_count = ctx
					.project
					.consumers
					.iter()
					.filter(|consumer| consumer.block.r#type == BlockType::Consumer)
					.filter(|c| c.block.name == *name)
					.count();
				ProviderInfo {
					name: name.clone(),
					file: relative_display_path(&entry.file, &root),
					content: entry.content.trim().to_string(),
					consumer_count,
				}
			})
			.collect();
		providers.sort_by(|a, b| a.name.cmp(&b.name));

		let consumers: Vec<ConsumerInfo> = ctx
			.project
			.consumers
			.iter()
			.map(|c| {
				let is_stale = ctx.project.providers.get(&c.block.name).is_some_and(|p| {
					let render_data =
						build_render_context(&ctx.data, p, c).unwrap_or_else(|| ctx.data.clone());
					let rendered = render_template(&p.content, &render_data)
						.unwrap_or_else(|_| p.content.clone());
					let expected = apply_transformers(&rendered, &c.block.transformers);
					c.content != expected
				});
				ConsumerInfo {
					name: c.block.name.clone(),
					file: relative_display_path(&c.file, &root),
					transformers: c
						.block
						.transformers
						.iter()
						.map(|t| t.r#type.to_string())
						.collect(),
					is_stale,
				}
			})
			.collect();

		let output = serde_json::json!({
			"providers": providers,
			"consumers": consumers,
			"summary": format!(
				"{} provider(s), {} consumer(s)",
				providers.len(),
				consumers.len()
			),
		});

		Ok(json_result(output))
	}

	#[tool(
		name = "mdt_find_reuse",
		description = "Find similar existing providers and where they are consumed across \
		               markdown and source files. Use this before creating a new provider to \
		               encourage template reuse."
	)]
	async fn find_reuse(
		&self,
		Parameters(params): Parameters<ReuseParam>,
	) -> Result<CallToolResult, McpError> {
		let root = resolve_root(params.path.as_deref().map(Path::new));
		let ctx = scan_ctx(&root)?;
		let limit = params.limit.clamp(1, 20);
		let query = params
			.block_name
			.as_ref()
			.map(|value| value.trim().to_string())
			.filter(|value| !value.is_empty());

		let mut candidates: Vec<ReuseCandidate> = ctx
			.project
			.providers
			.iter()
			.map(|(name, entry)| {
				let consumers: Vec<_> = ctx
					.project
					.consumers
					.iter()
					.filter(|consumer| consumer.block.r#type == BlockType::Consumer)
					.filter(|consumer| consumer.block.name == *name)
					.collect();

				let mut markdown_files = Vec::new();
				let mut code_files = Vec::new();
				for consumer in &consumers {
					let rel = relative_display_path(&consumer.file, &root);
					if is_markdown_path(&consumer.file) {
						markdown_files.push(rel);
					} else {
						code_files.push(rel);
					}
				}
				markdown_files.sort();
				markdown_files.dedup();
				code_files.sort();
				code_files.dedup();

				ReuseCandidate {
					name: name.clone(),
					file: relative_display_path(&entry.file, &root),
					consumer_count: consumers.len(),
					markdown_files,
					code_files,
					distance: query
						.as_ref()
						.map(|value| levenshtein_distance(value, name)),
				}
			})
			.collect();

		if query.is_some() {
			candidates.sort_by(|a, b| {
				a.distance
					.cmp(&b.distance)
					.then_with(|| b.consumer_count.cmp(&a.consumer_count))
					.then_with(|| a.name.cmp(&b.name))
			});
		} else {
			candidates.sort_by(|a, b| {
				b.consumer_count
					.cmp(&a.consumer_count)
					.then_with(|| a.name.cmp(&b.name))
			});
		}
		candidates.truncate(limit);

		let output = serde_json::json!({
			"query": query,
			"guidance": "Prefer reusing an existing provider when semantics match. Candidates show where blocks are already consumed in markdown and source files.",
			"candidates": candidates,
			"next_steps": [
				"If a candidate already matches your intent, reuse that block name in new consumers.",
				"If no candidate fits, create a new provider in .templates/ (or another configured template path)."
			],
		});

		Ok(json_result(output))
	}

	#[tool(
		name = "mdt_get_block",
		description = "Get full content, metadata, and status of a specific named block (provider \
		               or consumer). Returns the block content, source file, and any transformers."
	)]
	async fn get_block(
		&self,
		Parameters(params): Parameters<BlockParam>,
	) -> Result<CallToolResult, McpError> {
		let root = resolve_root(params.path.as_deref().map(Path::new));
		let ctx = scan_ctx(&root)?;

		if let Some(provider) = ctx.project.providers.get(&params.block_name) {
			let rendered = render_template(&provider.content, &ctx.data)
				.unwrap_or_else(|_| provider.content.clone());
			let consumer_count = ctx
				.project
				.consumers
				.iter()
				.filter(|consumer| consumer.block.r#type == BlockType::Consumer)
				.filter(|c| c.block.name == params.block_name)
				.count();
			let consumer_files: Vec<String> = ctx
				.project
				.consumers
				.iter()
				.filter(|consumer| consumer.block.r#type == BlockType::Consumer)
				.filter(|c| c.block.name == params.block_name)
				.map(|c| relative_display_path(&c.file, &root))
				.collect();

			let output = serde_json::json!({
				"type": "provider",
				"name": params.block_name,
				"file": relative_display_path(&provider.file, &root),
				"raw_content": provider.content,
				"rendered_content": rendered,
				"consumer_count": consumer_count,
				"consumer_files": consumer_files,
			});

			return Ok(json_result(output));
		}

		let consumer_entries: Vec<&mdt_core::project::ConsumerEntry> = ctx
			.project
			.consumers
			.iter()
			.filter(|c| c.block.name == params.block_name)
			.collect();

		if consumer_entries.is_empty() {
			return Ok(json_error_result(serde_json::json!({
				"ok": false,
				"action": "get_block",
				"summary": format!("No block named `{}` found in the project.", params.block_name),
				"block_name": params.block_name,
			})));
		}

		let mut entries = Vec::new();
		for c in &consumer_entries {
			let is_stale = ctx.project.providers.get(&c.block.name).is_some_and(|p| {
				let render_data =
					build_render_context(&ctx.data, p, c).unwrap_or_else(|| ctx.data.clone());
				let rendered =
					render_template(&p.content, &render_data).unwrap_or_else(|_| p.content.clone());
				let expected = apply_transformers(&rendered, &c.block.transformers);
				c.content != expected
			});
			entries.push(serde_json::json!({
				"type": "consumer",
				"name": c.block.name,
				"file": relative_display_path(&c.file, &root),
				"content": c.content,
				"transformers": c.block.transformers.iter().map(|t| t.r#type.to_string()).collect::<Vec<_>>(),
				"is_stale": is_stale,
			}));
		}

		Ok(json_result(serde_json::Value::Array(entries)))
	}

	#[tool(
		name = "mdt_preview",
		description = "Preview a provider as an authoring workflow. Returns JSON with the \
		               provider template plus per-consumer rendered output after interpolation \
		               and transformers."
	)]
	async fn preview(
		&self,
		Parameters(params): Parameters<BlockParam>,
	) -> Result<CallToolResult, McpError> {
		let root = resolve_root(params.path.as_deref().map(Path::new));
		let ctx = scan_ctx(&root)?;

		let Some(provider) = ctx.project.providers.get(&params.block_name) else {
			return Ok(json_error_result(serde_json::json!({
				"ok": false,
				"action": "preview",
				"summary": format!("No provider named `{}` found.", params.block_name),
				"block_name": params.block_name,
			})));
		};

		let rendered_with_project_data = render_template(&provider.content, &ctx.data)
			.unwrap_or_else(|_| provider.content.clone());

		let consumers: Vec<_> = ctx
			.project
			.consumers
			.iter()
			.filter(|consumer| consumer.block.name == params.block_name)
			.collect();
		let consumer_previews: Vec<_> = consumers
			.iter()
			.map(|consumer| {
				let transformers = consumer
					.block
					.transformers
					.iter()
					.map(|transformer| transformer.r#type.to_string())
					.collect::<Vec<_>>();
				let arguments = consumer.block.arguments.clone();
				let rel = relative_display_path(&consumer.file, &root);

				let (rendered_content, is_stale, render_error) =
					match build_render_context(&ctx.data, provider, consumer) {
						Some(render_data) => {
							match render_template(&provider.content, &render_data) {
								Ok(rendered) => {
									let transformed =
										apply_transformers(&rendered, &consumer.block.transformers);
									let stale = consumer.content != transformed;
									(Some(transformed), Some(stale), None)
								}
								Err(error) => (None, None, Some(error.to_string())),
							}
						}
						None => {
							(
								None,
								None,
								Some(format!(
									"argument count mismatch: provider `{}` declares {} \
									 parameter(s), but consumer passes {}",
									params.block_name,
									provider.block.arguments.len(),
									consumer.block.arguments.len()
								)),
							)
						}
					};

				PreviewConsumerInfo {
					file: rel,
					transformers,
					arguments,
					rendered_content,
					current_content: consumer.content.clone(),
					is_stale,
					render_error,
				}
			})
			.collect();

		let provider_preview = PreviewProviderInfo {
			name: params.block_name.clone(),
			file: relative_display_path(&provider.file, &root),
			raw_content: provider.content.clone(),
			rendered_with_project_data,
			parameters: provider.block.arguments.clone(),
		};
		let summary = if consumer_previews.is_empty() {
			format!(
				"Previewed provider `{}` with no consumers.",
				params.block_name
			)
		} else {
			format!(
				"Previewed provider `{}` for {} consumer(s).",
				params.block_name,
				consumer_previews.len()
			)
		};

		Ok(json_result(serde_json::json!({
			"ok": true,
			"action": "preview",
			"summary": summary,
			"block_name": params.block_name,
			"provider": provider_preview,
			"consumers": consumer_previews,
		})))
	}

	#[tool(
		name = "mdt_init",
		description = "Initialize mdt in a project by creating a sample \
		               `.templates/template.t.md` file and starter `mdt.toml`. Returns a \
		               JSON-first summary of created files and next steps."
	)]
	async fn init(
		&self,
		Parameters(params): Parameters<InitParam>,
	) -> Result<CallToolResult, McpError> {
		let root = resolve_root(params.path.as_deref().map(Path::new));
		let canonical_template_path = root.join(".templates/template.t.md");
		let legacy_template_paths = [
			root.join("template.t.md"),
			root.join("templates/template.t.md"),
		];
		let template_path = if canonical_template_path.exists() {
			canonical_template_path.clone()
		} else {
			legacy_template_paths
				.iter()
				.find(|path| path.exists())
				.cloned()
				.unwrap_or_else(|| canonical_template_path.clone())
		};
		let template_exists = template_path.exists();

		let config_path = root.join("mdt.toml");
		let config_exists = MdtConfig::resolve_path(&root).is_some();
		let sample_content = "<!-- {@greeting} -->\n\nHello from mdt! This is a provider \
		                      block.\n\n<!-- {/greeting} -->\n";
		let sample_config =
			"# mdt configuration\n# See \
			 https://ifiokjr.github.io/mdt/reference/configuration.html for full reference.\n\n# \
			 Map data files to template namespaces.\n# Values from these files are available in \
			 provider blocks as {{ namespace.key }}.\n# [data]\n# pkg = \"package.json\"\n# cargo \
			 = \"Cargo.toml\"\n# version = { command = \"cat VERSION\", format = \"text\", watch \
			 = [\"VERSION\"] }\n\n# Control blank lines between tags and content in source \
			 files.\n# Recommended when using formatters (rustfmt, prettier, etc.).\n# \
			 [padding]\n# before = 0\n# after = 0\n";

		if !template_exists {
			if let Some(parent) = template_path.parent() {
				std::fs::create_dir_all(parent)
					.map_err(|e| McpError::internal_error(e.to_string(), None))?;
			}
			std::fs::write(&template_path, sample_content)
				.map_err(|e| McpError::internal_error(e.to_string(), None))?;
		}

		if !config_exists {
			std::fs::write(&config_path, sample_config)
				.map_err(|e| McpError::internal_error(e.to_string(), None))?;
		}

		let next_steps = if template_exists {
			Vec::new()
		} else {
			vec![
				format!(
					"Edit {} to define your template blocks",
					template_path.display()
				),
				"Add consumer tags in your markdown files: <!-- {{=greeting}} --> ... <!-- \
				 {{/greeting}} -->"
					.to_string(),
				"Run `mdt_update` to sync content".to_string(),
			]
		};
		let summary = if template_exists {
			format!("Template file already exists: {}", template_path.display())
		} else {
			format!("Created template file: {}", template_path.display())
		};

		Ok(json_result(serde_json::json!({
			"ok": true,
			"action": "init",
			"summary": summary,
			"template_file": template_path.display().to_string(),
			"template_created": !template_exists,
			"config_file": config_path.display().to_string(),
			"config_created": !config_exists,
			"next_steps": next_steps,
		})))
	}
}

impl Default for MdtMcpServer {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod __tests;

/// Start the MCP server on stdin/stdout.
pub async fn run_server() {
	let server = MdtMcpServer::new();
	let transport = rmcp::transport::io::stdio();

	let service = server.serve(transport).await;

	match service {
		Ok(running) => {
			let _ = running.waiting().await;
		}
		Err(e) => {
			eprintln!("mdt-mcp: failed to start server: {e}");
		}
	}
}

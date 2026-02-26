//! <!-- {=mdtMcpOverview|trim|linePrefix:"//! ":true} -->
//! `mdt_mcp` is a [Model Context Protocol](https://modelcontextprotocol.io/) (MCP) server for the [mdt](https://github.com/ifiokjr/mdt) template engine. It exposes mdt functionality as MCP tools that can be used by AI assistants and other MCP-compatible clients.
//!
//! ### Tools
//!
//! - **`mdt_check`** — Verify all consumer blocks are up-to-date.
//! - **`mdt_update`** — Update all consumer blocks with latest provider content.
//! - **`mdt_list`** — List all providers and consumers in the project.
//! - **`mdt_get_block`** — Get the content of a specific block by name.
//! - **`mdt_preview`** — Preview the result of applying transformers to a block.
//! - **`mdt_init`** — Initialize a new mdt project with a sample template file.
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
use std::path::PathBuf;

use mdt_core::apply_transformers;
use mdt_core::check_project;
use mdt_core::compute_updates;
use mdt_core::project::ProjectContext;
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

/// The MCP server for mdt.
#[derive(Debug, Clone)]
pub struct MdtMcpServer {
	tool_router: ToolRouter<Self>,
}

#[tool_handler]
impl ServerHandler for MdtMcpServer {
	fn get_info(&self) -> ServerInfo {
		ServerInfo {
			instructions: Some(
				"mdt (manage markdown templates) keeps documentation synchronized across your \
				 project using comment-based template tags. Use these tools to check, update, \
				 list, and preview template blocks."
					.into(),
			),
			capabilities: ServerCapabilities::builder().enable_tools().build(),
			..Default::default()
		}
	}
}

fn resolve_root(path: Option<&str>) -> PathBuf {
	path.map_or_else(
		|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
		PathBuf::from,
	)
}

fn make_relative(path: &Path, root: &Path) -> String {
	path.strip_prefix(root)
		.unwrap_or(path)
		.display()
		.to_string()
}

fn scan_ctx(root: &Path) -> Result<ProjectContext, McpError> {
	scan_project_with_config(root).map_err(|e| McpError::internal_error(e.to_string(), None))
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
		description = "Check if all consumer blocks are up to date. Returns an actionable summary \
		               of any stale blocks with file paths and diffs."
	)]
	async fn check(
		&self,
		Parameters(params): Parameters<PathParam>,
	) -> Result<CallToolResult, McpError> {
		let root = resolve_root(params.path.as_deref());
		let ctx = scan_ctx(&root)?;

		let missing = ctx.find_missing_providers();
		let result =
			check_project(&ctx).map_err(|e| McpError::internal_error(e.to_string(), None))?;

		if result.is_ok() && missing.is_empty() {
			return Ok(CallToolResult::success(vec![Content::text(
				"All consumer blocks are up to date.",
			)]));
		}

		let mut parts = Vec::new();

		if !result.render_errors.is_empty() {
			parts.push(format!(
				"{} template render error(s):",
				result.render_errors.len()
			));
			for err in &result.render_errors {
				let rel = make_relative(&err.file, &root);
				parts.push(format!(
					"  - `{}` in {rel}: {}",
					err.block_name, err.message
				));
			}
		}

		if !result.stale.is_empty() {
			parts.push(format!(
				"{} consumer block(s) are stale:",
				result.stale.len()
			));
			for entry in &result.stale {
				let rel = make_relative(&entry.file, &root);
				parts.push(format!("  - `{}` in {rel}", entry.block_name));
			}
			parts.push(String::new());
			parts.push("Run mdt_update to synchronize them.".to_string());
		}

		if !missing.is_empty() {
			parts.push(format!(
				"\n{} consumer block(s) reference missing providers: {}",
				missing.len(),
				missing.join(", ")
			));
		}

		Ok(CallToolResult::success(vec![Content::text(
			parts.join("\n"),
		)]))
	}

	#[tool(
		name = "mdt_update",
		description = "Update all stale consumer blocks with latest provider content. Supports \
		               dry_run mode to preview changes without writing."
	)]
	async fn update(
		&self,
		Parameters(params): Parameters<UpdateParam>,
	) -> Result<CallToolResult, McpError> {
		let root = resolve_root(params.path.as_deref());
		let ctx = scan_ctx(&root)?;
		let updates =
			compute_updates(&ctx).map_err(|e| McpError::internal_error(e.to_string(), None))?;

		if updates.updated_count == 0 {
			return Ok(CallToolResult::success(vec![Content::text(
				"All consumer blocks are already up to date. No changes needed.",
			)]));
		}

		if params.dry_run {
			let files: Vec<String> = updates
				.updated_files
				.keys()
				.map(|p| format!("  - {}", make_relative(p, &root)))
				.collect();
			let msg = format!(
				"Dry run: would update {} block(s) in {} file(s):\n{}",
				updates.updated_count,
				updates.updated_files.len(),
				files.join("\n")
			);
			return Ok(CallToolResult::success(vec![Content::text(msg)]));
		}

		write_updates(&updates).map_err(|e| McpError::internal_error(e.to_string(), None))?;

		let files: Vec<String> = updates
			.updated_files
			.keys()
			.map(|p| make_relative(p, &root))
			.collect();
		let msg = format!(
			"Updated {} block(s) in {} file(s): {}",
			updates.updated_count,
			updates.updated_files.len(),
			files.join(", ")
		);
		Ok(CallToolResult::success(vec![Content::text(msg)]))
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
		let root = resolve_root(params.path.as_deref());
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
					.filter(|c| c.block.name == *name)
					.count();
				ProviderInfo {
					name: name.clone(),
					file: make_relative(&entry.file, &root),
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
					let render_data = mdt_core::build_render_context(&ctx.data, p, c)
						.unwrap_or_else(|| ctx.data.clone());
					let rendered = render_template(&p.content, &render_data)
						.unwrap_or_else(|_| p.content.clone());
					let expected = apply_transformers(&rendered, &c.block.transformers);
					c.content != expected
				});
				ConsumerInfo {
					name: c.block.name.clone(),
					file: make_relative(&c.file, &root),
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

		Ok(CallToolResult::success(vec![Content::text(
			serde_json::to_string_pretty(&output)
				.unwrap_or_else(|_| "Failed to serialize output".to_string()),
		)]))
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
		let root = resolve_root(params.path.as_deref());
		let ctx = scan_ctx(&root)?;

		if let Some(provider) = ctx.project.providers.get(&params.block_name) {
			let rendered = render_template(&provider.content, &ctx.data)
				.unwrap_or_else(|_| provider.content.clone());
			let consumer_count = ctx
				.project
				.consumers
				.iter()
				.filter(|c| c.block.name == params.block_name)
				.count();
			let consumer_files: Vec<String> = ctx
				.project
				.consumers
				.iter()
				.filter(|c| c.block.name == params.block_name)
				.map(|c| make_relative(&c.file, &root))
				.collect();

			let output = serde_json::json!({
				"type": "provider",
				"name": params.block_name,
				"file": make_relative(&provider.file, &root),
				"raw_content": provider.content,
				"rendered_content": rendered,
				"consumer_count": consumer_count,
				"consumer_files": consumer_files,
			});

			return Ok(CallToolResult::success(vec![Content::text(
				serde_json::to_string_pretty(&output)
					.unwrap_or_else(|_| "Failed to serialize".to_string()),
			)]));
		}

		let consumer_entries: Vec<&mdt_core::project::ConsumerEntry> = ctx
			.project
			.consumers
			.iter()
			.filter(|c| c.block.name == params.block_name)
			.collect();

		if consumer_entries.is_empty() {
			return Ok(CallToolResult::error(vec![Content::text(format!(
				"No block named `{}` found in the project.",
				params.block_name
			))]));
		}

		let mut entries = Vec::new();
		for c in &consumer_entries {
			let is_stale = ctx.project.providers.get(&c.block.name).is_some_and(|p| {
				let render_data = mdt_core::build_render_context(&ctx.data, p, c)
					.unwrap_or_else(|| ctx.data.clone());
				let rendered =
					render_template(&p.content, &render_data).unwrap_or_else(|_| p.content.clone());
				let expected = apply_transformers(&rendered, &c.block.transformers);
				c.content != expected
			});
			entries.push(serde_json::json!({
				"type": "consumer",
				"name": c.block.name,
				"file": make_relative(&c.file, &root),
				"content": c.content,
				"transformers": c.block.transformers.iter().map(|t| t.r#type.to_string()).collect::<Vec<_>>(),
				"is_stale": is_stale,
			}));
		}

		Ok(CallToolResult::success(vec![Content::text(
			serde_json::to_string_pretty(&entries)
				.unwrap_or_else(|_| "Failed to serialize".to_string()),
		)]))
	}

	#[tool(
		name = "mdt_preview",
		description = "Preview the rendered content for a specific block after template \
		               interpolation and transformer application."
	)]
	async fn preview(
		&self,
		Parameters(params): Parameters<BlockParam>,
	) -> Result<CallToolResult, McpError> {
		let root = resolve_root(params.path.as_deref());
		let ctx = scan_ctx(&root)?;

		let Some(provider) = ctx.project.providers.get(&params.block_name) else {
			return Ok(CallToolResult::error(vec![Content::text(format!(
				"No provider named `{}` found.",
				params.block_name
			))]));
		};

		let rendered = render_template(&provider.content, &ctx.data)
			.map_err(|e| McpError::internal_error(e.to_string(), None))?;

		let consumers: Vec<_> = ctx
			.project
			.consumers
			.iter()
			.filter(|c| c.block.name == params.block_name)
			.collect();

		let mut parts = Vec::new();
		parts.push(format!(
			"## Provider `{}`\n\nRendered content:\n```\n{}\n```",
			params.block_name,
			rendered.trim()
		));

		if !consumers.is_empty() {
			parts.push(format!("\n## {} consumer(s):", consumers.len()));
			for c in &consumers {
				let transformed = apply_transformers(&rendered, &c.block.transformers);
				let rel = make_relative(&c.file, &root);
				let tf_names: Vec<String> = c
					.block
					.transformers
					.iter()
					.map(|t| t.r#type.to_string())
					.collect();
				let tf_str = if tf_names.is_empty() {
					"(none)".to_string()
				} else {
					tf_names.join(" | ")
				};
				parts.push(format!(
					"\n### {rel}\nTransformers: {tf_str}\n```\n{}\n```",
					transformed.trim()
				));
			}
		}

		Ok(CallToolResult::success(vec![Content::text(
			parts.join("\n"),
		)]))
	}

	#[tool(
		name = "mdt_init",
		description = "Initialize mdt in a project by creating a sample template.t.md file."
	)]
	async fn init(
		&self,
		Parameters(params): Parameters<InitParam>,
	) -> Result<CallToolResult, McpError> {
		let root = resolve_root(params.path.as_deref());
		let template_path = root.join("template.t.md");

		if template_path.exists() {
			return Ok(CallToolResult::success(vec![Content::text(format!(
				"Template file already exists: {}",
				template_path.display()
			))]));
		}

		let sample_content = "<!-- {@greeting} -->\n\nHello from mdt! This is a provider \
		                      block.\n\n<!-- {/greeting} -->\n";

		std::fs::write(&template_path, sample_content)
			.map_err(|e| McpError::internal_error(e.to_string(), None))?;

		Ok(CallToolResult::success(vec![Content::text(format!(
			"Created template file: {}\n\nNext steps:\n1. Edit the template to define your \
			 blocks\n2. Add consumer tags in your files: <!-- {{=greeting}} --> <!-- \
			 {{/greeting}} -->\n3. Run mdt_update to sync content",
			template_path.display()
		))]))
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

//! <!-- {=mdtMcpOverview|trim|linePrefix:"//! ":true} -->
//! `mdt_mcp` is a [Model Context Protocol](https://modelcontextprotocol.io/) (MCP) server for the [mdt](https://github.com/ifiokjr/mdt) template engine. It exposes mdt functionality as MCP tools that can be used by AI assistants and other MCP-compatible clients.
//!
//! ### Tools
//!
//! - **`mdt_check`** — Verify all target blocks are up-to-date.
//! - **`mdt_update`** — Update all target blocks with latest source content.
//! - **`mdt_list`** — List all sources and targets in the project.
//! - **`mdt_find_reuse`** — Find similar providers and where they are already consumed, to encourage reuse.
//! - **`mdt_get_block`** — Get the content of a specific block by name.
//! - **`mdt_preview`** — Preview the result of applying transformers to a block.
//! - **`mdt_init`** — Initialize a new mdt project with a sample `.templates/template.t.md` file and starter `mdt.toml`.
//!
//! ### Agent Workflow
//!
//! - Prefer reuse before creation: call `mdt_find_reuse` (or `mdt_list`) before introducing a new source block.
//! - Use the JSON-first tool responses as the source of truth. The MCP server returns structured payloads so agents can inspect results without parsing prose.
//! - Use `mdt_preview` while authoring: inspect the source template and each target's rendered output before deciding whether to reuse, edit, or sync.
//! - Keep source names global and unique in the project to avoid collisions.
//! - After edits, run `mdt_check` (and optionally `mdt_update`) so target blocks stay synchronized.
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

use std::path::PathBuf;

use mdt_core::project::ValidationOptions;
use mdt_core::project::resolve_root;
use rmcp::RoleServer;
use rmcp::ServerHandler;
use rmcp::ServiceExt;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::Implementation;
use rmcp::model::ServerCapabilities;
use rmcp::model::ServerConfig;
use rmcp::schemars;
use rmcp::serde;
use rmcp::tool;
use rmcp::tool_handler;
use rmcp::tool_router;
use rmcp::transport::IntoTransport;
use serde::Deserialize;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;

use crate::confine::RootRequirement;
use crate::confine::confine_root;
use crate::response::ToolError;

mod confine;
mod response;
mod reuse;
mod tools;

/// Validation switches mirroring the `mdt` CLI's `--ignore-*` flags.
/// Diagnostics they silence are left out of `diagnostics` entirely.
#[derive(Debug, Clone, Default, Deserialize, schemars::JsonSchema)]
#[allow(clippy::struct_excessive_bools)]
pub struct ValidationParam {
	/// Do not report unclosed blocks (`--ignore-unclosed-blocks`).
	#[serde(default)]
	pub ignore_unclosed_blocks: bool,
	/// Do not report providers that have no consumers
	/// (`--ignore-unused-blocks`).
	#[serde(default)]
	pub ignore_unused_blocks: bool,
	/// Do not report comments that look like tags but do not parse
	/// (`--ignore-invalid-names`).
	#[serde(default)]
	pub ignore_invalid_names: bool,
	/// Do not report unknown transformers or wrong transformer argument
	/// counts (`--ignore-invalid-transformers`).
	#[serde(default)]
	pub ignore_invalid_transformers: bool,
}

impl From<ValidationParam> for ValidationOptions {
	fn from(param: ValidationParam) -> Self {
		Self {
			ignore_unclosed_blocks: param.ignore_unclosed_blocks,
			ignore_unused_blocks: param.ignore_unused_blocks,
			ignore_invalid_names: param.ignore_invalid_names,
			ignore_invalid_transformers: param.ignore_invalid_transformers,
		}
	}
}

/// Parameters for `mdt_check`.
#[derive(Debug, Clone, Default, Deserialize, schemars::JsonSchema)]
pub struct CheckParam {
	/// Project root: absolute, or relative to the server root. Must resolve
	/// to an existing directory inside the server root. Defaults to the
	/// server root.
	pub path: Option<String>,
	#[serde(flatten)]
	pub validation: ValidationParam,
}

/// Parameters for `mdt_update`.
#[derive(Debug, Clone, Default, Deserialize, schemars::JsonSchema)]
pub struct UpdateParam {
	/// Project root: absolute, or relative to the server root. Must resolve
	/// to an existing directory inside the server root. Defaults to the
	/// server root.
	pub path: Option<String>,
	/// Report what would change without writing any file.
	#[serde(default)]
	pub dry_run: bool,
	#[serde(flatten)]
	pub validation: ValidationParam,
}

/// Parameters for `mdt_list`.
#[derive(Debug, Clone, Default, Deserialize, schemars::JsonSchema)]
pub struct ListParam {
	/// Project root: absolute, or relative to the server root. Must resolve
	/// to an existing directory inside the server root. Defaults to the
	/// server root.
	pub path: Option<String>,
	/// Include each provider's trimmed body as `content`. Off by default to
	/// keep the response small; `mdt_get_block` returns one block's content.
	#[serde(default)]
	pub include_content: bool,
	#[serde(flatten)]
	pub validation: ValidationParam,
}

/// Parameters for tools that look up one block by name.
#[derive(Debug, Clone, Default, Deserialize, schemars::JsonSchema)]
pub struct BlockParam {
	/// Project root: absolute, or relative to the server root. Must resolve
	/// to an existing directory inside the server root. Defaults to the
	/// server root.
	pub path: Option<String>,
	/// The block name, without tag sigils (`greeting`, not `@greeting`).
	pub block_name: String,
}

/// Parameters for `mdt_init`.
#[derive(Debug, Clone, Default, Deserialize, schemars::JsonSchema)]
pub struct InitParam {
	/// Directory to initialize: absolute, or relative to the server root.
	/// Must resolve inside the server root; created when missing. Defaults
	/// to the server root.
	pub path: Option<String>,
}

/// Parameters for `mdt_find_reuse`.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct ReuseParam {
	/// Project root: absolute, or relative to the server root. Must resolve
	/// to an existing directory inside the server root. Defaults to the
	/// server root.
	pub path: Option<String>,
	/// Proposed block name. Matches exact names first, then names equal up
	/// to case and `-`/`_` separators, then prefixes, substrings, and close
	/// spellings.
	pub block_name: Option<String>,
	/// Text to look for in provider bodies, ignoring case.
	pub content_query: Option<String>,
	/// Maximum number of candidates to return. Values outside 1–20 are
	/// clamped.
	#[serde(default = "default_reuse_limit")]
	#[schemars(range(min = 1, max = 20))]
	pub limit: usize,
}

impl Default for ReuseParam {
	fn default() -> Self {
		Self {
			path: None,
			block_name: None,
			content_query: None,
			limit: default_reuse_limit(),
		}
	}
}

fn default_reuse_limit() -> usize {
	5
}

/// Trimmed `value`, or `None` when it is missing or blank.
fn non_blank(value: Option<String>) -> Option<String> {
	value
		.map(|value| value.trim().to_string())
		.filter(|value| !value.is_empty())
}

const SERVER_INSTRUCTIONS: &str =
	"mdt (manage markdown templates) keeps documentation in sync. Provider blocks (`{@name}` tags \
	 in `*.t.md` files) define content once; consumer blocks (`{=name}` tags in markdown and \
	 source-code comments) receive it. Every tool returns a JSON object, also sent as structured \
	 content, with `ok`, `action`, and `summary`; results marked isError add `error.code` and \
	 `error.message`. Workflow: before creating a provider, call mdt_find_reuse (or mdt_list) and \
	 reuse an existing block when one fits. Use mdt_get_block and mdt_preview to inspect a block \
	 and what each consumer will receive. After editing, run mdt_update, then mdt_check, which \
	 fails on stale consumers, render errors, orphan consumers, and validation errors, exactly \
	 like `mdt check`. Tool paths must stay inside the directory the server serves. For the full \
	 syntax, transformer, and configuration guide, run `mdt skill` (and `mdt skill --reference`) \
	 in a shell.";

/// The MCP server for mdt.
#[derive(Debug, Clone)]
pub struct MdtMcpServer {
	pub tool_router: ToolRouter<Self>,
	/// Canonical directory that every tool path must resolve within.
	base_root: PathBuf,
}

#[tool_handler]
impl ServerHandler for MdtMcpServer {
	fn get_info(&self) -> ServerConfig {
		ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
			.with_server_info(Implementation::new("mdt", env!("CARGO_PKG_VERSION")))
			.with_instructions(SERVER_INSTRUCTIONS)
	}
}

#[tool_router]
impl MdtMcpServer {
	/// Build a server confined to the current directory.
	pub fn new() -> Self {
		Self::with_base_root(resolve_root(None))
	}

	/// Build a server whose tool paths must resolve within `base_root`.
	///
	/// Confinement matters because mdt executes config-declared shell
	/// commands (`[data]` scripts, formatters) and writes files relative to
	/// the project root: without it, a caller-supplied `path` could point the
	/// tools at any directory on disk and run whatever `mdt.toml` it finds
	/// there.
	pub fn with_base_root(base_root: impl Into<PathBuf>) -> Self {
		let base_root = base_root.into();
		// A root that does not exist cannot be canonicalized; keep its
		// absolute form so every tool reports it as missing.
		let base_root = base_root
			.canonicalize()
			.unwrap_or_else(|_| resolve_root(Some(&base_root)));
		Self {
			tool_router: Self::tool_router(),
			base_root,
		}
	}

	/// Confine `path` to the server root, then run `operation` with the
	/// resolved project root on the blocking thread pool, so the stdio
	/// transport keeps serving requests during directory walks, data scripts,
	/// and formatter subprocesses. A [`ToolError`] becomes an `isError`
	/// result for `action`.
	async fn run_tool<F>(
		&self,
		action: &'static str,
		path: Option<String>,
		requirement: RootRequirement,
		operation: F,
	) -> CallToolResult
	where
		F: FnOnce(PathBuf) -> Result<CallToolResult, ToolError> + Send + 'static,
	{
		let base_root = self.base_root.clone();
		let outcome = tokio::task::spawn_blocking(move || {
			operation(confine_root(&base_root, path.as_deref(), requirement)?)
		})
		.await;

		match outcome {
			Ok(Ok(result)) => result,
			Ok(Err(error)) => error.into_result(action),
			Err(error) => {
				ToolError::new(
					"mdt::internal",
					format!("the `{action}` tool failed: {error}"),
				)
				.into_result(action)
			}
		}
	}

	#[tool(
		name = "mdt_check",
		description = "Check that every consumer block matches its provider, exactly like `mdt \
		               check`. Read-only. Returns JSON with `ok` (false on stale consumers, \
		               formatter-only stale files, render errors, orphan consumers, or validation \
		               errors), `summary`, `stale`, `stale_files`, `render_errors`, `orphans` \
		               (with suggested provider names), `diagnostics` (parser and validation \
		               errors and warnings), `warnings` (undefined template variables), and \
		               `missing_provider_names`.",
		annotations(read_only_hint = true)
	)]
	async fn check(&self, Parameters(params): Parameters<CheckParam>) -> CallToolResult {
		let options = ValidationOptions::from(params.validation);
		self.run_tool(
			"check",
			params.path,
			RootRequirement::ExistingDirectory,
			move |root| tools::check(&root, &options),
		)
		.await
	}

	#[tool(
		name = "mdt_update",
		description = "Write the latest provider content into every stale consumer block, exactly \
		               like `mdt update`. With `dry_run: true`, reports what would change without \
		               writing. Refuses to write when validation errors exist (isError, code \
		               `mdt::validation`, with `diagnostics`). Returns JSON with `ok` (false when \
		               a consumer failed to render), `summary`, `dry_run`, `updated_count`, \
		               `updated_files`, `render_errors` (consumers left unchanged), \
		               `diagnostics`, `warnings`, and `missing_provider_names`.",
		annotations(
			read_only_hint = false,
			destructive_hint = true,
			idempotent_hint = true
		)
	)]
	async fn update(&self, Parameters(params): Parameters<UpdateParam>) -> CallToolResult {
		let options = ValidationOptions::from(params.validation);
		let dry_run = params.dry_run;
		self.run_tool(
			"update",
			params.path,
			RootRequirement::ExistingDirectory,
			move |root| tools::update(&root, &options, dry_run),
		)
		.await
	}

	#[tool(
		name = "mdt_list",
		description = "List every provider and consumer block. Read-only. Providers carry \
		               location and `consumer_count`; their bodies are omitted unless \
		               `include_content` is true (use mdt_get_block for one block). Consumers \
		               carry `type` (`consumer` or `inline`), location, transformers, arguments, \
		               and `status` (`current`, `stale`, `render_error`, or `orphan`), computed \
		               exactly as `mdt check` does. Also returns `diagnostics`; `ok` is false \
		               when any is an error.",
		annotations(read_only_hint = true)
	)]
	async fn list(&self, Parameters(params): Parameters<ListParam>) -> CallToolResult {
		let options = ValidationOptions::from(params.validation);
		let include_content = params.include_content;
		self.run_tool(
			"list",
			params.path,
			RootRequirement::ExistingDirectory,
			move |root| tools::list(&root, &options, include_content),
		)
		.await
	}

	#[tool(
		name = "mdt_find_reuse",
		description = "Find existing providers to reuse before creating a new one. Read-only. \
		               With `block_name`, ranks providers by name: exact, then equal up to case \
		               and `-`/`_` separators, then prefix, substring, and close spellings; \
		               unrelated providers are left out. With `content_query`, also matches \
		               providers whose body contains the text. Without either, lists providers by \
		               consumer count. Each candidate carries `match`, `consumer_count`, and the \
		               markdown and code files that already consume it.",
		annotations(read_only_hint = true)
	)]
	async fn find_reuse(&self, Parameters(params): Parameters<ReuseParam>) -> CallToolResult {
		let block_name = non_blank(params.block_name);
		let content_query = non_blank(params.content_query);
		let limit = params.limit.clamp(1, 20);
		self.run_tool(
			"find_reuse",
			params.path,
			RootRequirement::ExistingDirectory,
			move |root| {
				tools::find_reuse(
					&root,
					&tools::ReuseQuery {
						block_name: block_name.as_deref(),
						content: content_query.as_deref(),
						limit,
					},
				)
			},
		)
		.await
	}

	#[tool(
		name = "mdt_get_block",
		description = "Get one block by name. Read-only. Returns `provider` (raw content, content \
		               rendered with project data, parameters, and consumer count; null when no \
		               provider has the name) and `consumers`: every consumer and inline block \
		               with the name, with its `current_content` and `status` exactly as `mdt \
		               check` computes it. `ok` is false when the provider or a consumer fails to \
		               render, or when the consumers have no provider.",
		annotations(read_only_hint = true)
	)]
	async fn get_block(&self, Parameters(params): Parameters<BlockParam>) -> CallToolResult {
		let block_name = params.block_name;
		self.run_tool(
			"get_block",
			params.path,
			RootRequirement::ExistingDirectory,
			move |root| tools::get_block(&root, &block_name),
		)
		.await
	}

	#[tool(
		name = "mdt_preview",
		description = "Preview what `mdt update` would write for a provider. Read-only. Returns \
		               the provider template and its rendering with project data, and for each \
		               consumer its `current_content` next to `rendered_content` (after data, \
		               arguments, transformers, and padding; before `[[formatters]]`) and its \
		               `status`. `ok` is false when rendering fails.",
		annotations(read_only_hint = true)
	)]
	async fn preview(&self, Parameters(params): Parameters<BlockParam>) -> CallToolResult {
		let block_name = params.block_name;
		self.run_tool(
			"preview",
			params.path,
			RootRequirement::ExistingDirectory,
			move |root| tools::preview(&root, &block_name),
		)
		.await
	}

	#[tool(
		name = "mdt_init",
		description = "Initialize mdt exactly like `mdt init`, adding only what is missing: an \
		               annotated `mdt.toml`, a sample `greeting` provider in \
		               `.templates/template.t.md` (unless providers exist), a synced `readme.md` \
		               when the project has no README, and `.mdt/` in `.gitignore` for git \
		               repositories. Never overwrites files. Returns the `config`, `sample`, and \
		               `gitignore` outcomes, `written_files` (relative to the initialized root), \
		               and `next_steps`.",
		annotations(
			read_only_hint = false,
			destructive_hint = false,
			idempotent_hint = true
		)
	)]
	async fn init(&self, Parameters(params): Parameters<InitParam>) -> CallToolResult {
		let base_root = self.base_root.clone();
		self.run_tool(
			"init",
			params.path,
			RootRequirement::Creatable,
			move |root| tools::init(&base_root, &root),
		)
		.await
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

/// Start the MCP server on stdin/stdout, serving the current directory.
pub async fn run_server() {
	run_server_in(resolve_root(None)).await;
}

/// Start the MCP server on stdin/stdout, serving `root`: every tool `path`
/// must resolve inside it (`mdt mcp --path <dir>`).
pub async fn run_server_in(root: PathBuf) {
	serve_in(root, rmcp::transport::io::stdio()).await;
}

/// Serve the tools for `root` on `transport` until the client disconnects.
async fn serve_in<T, E, A>(root: PathBuf, transport: T)
where
	T: IntoTransport<RoleServer, E, A>,
	E: std::error::Error + Send + Sync + 'static,
{
	init_tracing();
	match MdtMcpServer::with_base_root(root).serve(transport).await {
		Ok(running) => {
			if let Err(error) = running.waiting().await {
				tracing::error!("mdt MCP server stopped unexpectedly: {error}");
			}
		}
		Err(error) => tracing::error!("failed to start the mdt MCP server: {error}"),
	}
}

/// Log to stderr, filtered by `MDT_LOG` (default `info`).
///
/// The `mdt` CLI installs its own global subscriber when `MDT_LOG` is set.
/// That subscriber keeps receiving the server's events, so finding one
/// already installed is expected, not an error.
fn init_tracing() {
	let filter = EnvFilter::try_from_env("MDT_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
	let installed = fmt::Subscriber::builder()
		.with_env_filter(filter)
		.with_writer(std::io::stderr)
		.try_init();
	if let Err(error) = installed {
		tracing::debug!("keeping the existing tracing subscriber: {error}");
	}
}

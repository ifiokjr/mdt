//! <!-- {=mdtLspOverview|trim|linePrefix:"//! ":true} -->
//! `mdt_lsp` is a [Language Server Protocol](https://microsoft.github.io/language-server-protocol/) implementation for the [mdt](https://github.com/ifiokjr/mdt) template engine. It brings diagnostics, completions, and navigation for template blocks into the editor.
//!
//! ### Capabilities
//!
//! - **Diagnostics** — reports stale target blocks, missing sources (with name suggestions), duplicate sources, unclosed blocks, unknown transformers, invalid arguments, unused sources, and source blocks in non-template files.
//! - **Completions** — suggests block names after `{=`, `{~`, `{@`, and `{/` tags, and transformer names after `|`.
//! - **Hover** — shows provider source, rendered content, transformer chain, and consumer count when hovering over a block tag.
//! - **Go to definition** — navigates from a target block to its provider, or from a source to all of its consumers.
//! - **References** — finds all source, target, and inline blocks sharing the same name.
//! - **Rename** — renames a block across all provider and target tags (both opening and closing) in the workspace.
//! - **Document symbols** — lists source, target, and inline blocks in the outline/symbol view.
//! - **Code actions** — offers a quick-fix to update stale target blocks in place.
//!
//! ### Usage
//!
//! Start the language server via the CLI:
//!
//! ```sh
//! mdt lsp
//! ```
//!
//! The server communicates over stdin/stdout using the Language Server Protocol.
//! <!-- {/mdtLspOverview} -->

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use mdt_core::Block;
use mdt_core::BlockType;
use mdt_core::CodeBlockFilter;
use mdt_core::ComparisonMode;
use mdt_core::ExpectedContent;
use mdt_core::ParseDiagnostic;
use mdt_core::content_matches;
use mdt_core::expected_consumer_content;
use mdt_core::formatter_applies;
use mdt_core::parse_source_with_diagnostics;
use mdt_core::parse_with_diagnostics;
use mdt_core::project::ConsumerEntry;
use mdt_core::project::Project;
use mdt_core::project::ProjectContext;
use mdt_core::project::ProviderEntry;
use mdt_core::project::extract_content_between_tags;
use mdt_core::project::is_markdown_path;
use mdt_core::project::normalize_line_endings;
use mdt_core::project::scan_project_with_config;
use mdt_core::project::suggest_similar_provider_names;
use tokio::sync::RwLock;
use tower_lsp_server::Client;
use tower_lsp_server::LanguageServer;
use tower_lsp_server::jsonrpc::Result as LspResult;
use tower_lsp_server::ls_types::*;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;

/// State for a single open document.
#[derive(Debug, Clone)]
struct DocumentState {
	/// The full text content of the document.
	content: String,
	/// Parsed mdt blocks in this document.
	blocks: Vec<Block>,
	/// Parse diagnostics (unclosed blocks, unknown transformers, etc.).
	parse_diagnostics: Vec<ParseDiagnostic>,
}

/// Workspace-level state shared across all LSP requests.
#[derive(Debug)]
struct WorkspaceState {
	/// The workspace root path.
	root: Option<PathBuf>,
	/// Open documents keyed by URI.
	documents: HashMap<Uri, DocumentState>,
	/// The last successful project scan: providers, consumers, template
	/// data, and the `mdt.toml` settings that decide what `mdt update`
	/// writes and what `mdt check` accepts. Saved documents update the
	/// providers and consumers incrementally.
	ctx: ProjectContext,
}

impl Default for WorkspaceState {
	fn default() -> Self {
		Self {
			root: None,
			documents: HashMap::new(),
			ctx: empty_project_context(),
		}
	}
}

/// A project with no blocks, no data, and default settings, used until the
/// first successful scan.
fn empty_project_context() -> ProjectContext {
	ProjectContext {
		root: PathBuf::new(),
		project: Project {
			providers: HashMap::new(),
			consumers: Vec::new(),
			diagnostics: Vec::new(),
		},
		data: HashMap::new(),
		padding: None,
		formatters: Vec::new(),
		markdown_codeblocks: CodeBlockFilter::default(),
		comparison: ComparisonMode::default(),
	}
}

impl WorkspaceState {
	/// Store the result of a successful project scan.
	fn apply_scan(&mut self, ctx: ProjectContext) {
		self.ctx = ctx;
	}

	/// Synchronous project scan used by tests. The server itself scans on
	/// the blocking thread pool via
	/// [`MdtLanguageServer::scan_project_offload`].
	#[cfg(test)]
	fn rescan_project(&mut self) {
		let Some(root) = &self.root else {
			return;
		};

		match scan_project_with_config(root) {
			Ok(ctx) => self.apply_scan(ctx),
			Err(e) => tracing::error!("failed to scan project: {e}"),
		}
	}

	/// Incrementally update a single document in the project state.
	/// For template files, this updates providers without a full rescan.
	/// For non-template files, this updates consumers for that file.
	fn update_document_in_project(&mut self, uri: &Uri) {
		let Some(doc) = self.documents.get(uri) else {
			return;
		};

		let Some(file_path) = uri.to_file_path().map(std::borrow::Cow::into_owned) else {
			return;
		};

		let is_template = file_path
			.file_name()
			.and_then(|name| name.to_str())
			.is_some_and(|name| name.ends_with(".t.md"));

		if is_template {
			// Drop this file's previous providers first: providers removed or
			// renamed in the document would otherwise linger (and power
			// completions, goto-definition, and rename edits) until the next
			// full rescan.
			self.ctx
				.project
				.providers
				.retain(|_, entry| entry.file != file_path);
		}

		for block in &doc.blocks {
			let block_content = extract_content_between_tags(&doc.content, block);

			if block.r#type == BlockType::Provider && is_template {
				self.ctx.project.providers.insert(
					block.name.clone(),
					ProviderEntry {
						block: block.clone(),
						file: file_path.clone(),
						content: block_content,
					},
				);
			}
		}

		// Update consumers for this file: remove existing then re-add.
		let consumers = &mut self.ctx.project.consumers;
		consumers.retain(|c| c.file != file_path);
		for block in &doc.blocks {
			if matches!(block.r#type, BlockType::Consumer | BlockType::Inline) {
				let block_content = extract_content_between_tags(&doc.content, block);
				consumers.push(ConsumerEntry {
					block: block.clone(),
					file: file_path.clone(),
					content: block_content,
				});
			}
		}
	}

	/// Parse a single document and update its cached state. Returns the
	/// parsed blocks.
	fn parse_document(&mut self, uri: &Uri, content: String) -> Vec<Block> {
		let (blocks, parse_diagnostics) =
			parse_document_content(uri, &content, &self.ctx.markdown_codeblocks);
		self.documents.insert(
			uri.clone(),
			DocumentState {
				content,
				blocks: blocks.clone(),
				parse_diagnostics,
			},
		);
		blocks
	}
}

/// Parse document content, choosing the right parser based on file extension.
/// Returns both parsed blocks and any parse diagnostics (unclosed blocks,
/// unknown transformers, etc.).
///
/// `markdown_codeblocks` is the configured `[exclude] markdown_codeblocks`
/// filter, so tags the scanner ignores in source-comment code blocks are
/// ignored here too.
fn parse_document_content(
	uri: &Uri,
	content: &str,
	markdown_codeblocks: &CodeBlockFilter,
) -> (Vec<Block>, Vec<ParseDiagnostic>) {
	let result = if is_markdown_path(Path::new(uri.path().as_str())) {
		parse_with_diagnostics(content)
	} else {
		parse_source_with_diagnostics(content, markdown_codeblocks)
	};

	result.unwrap_or_default()
}

/// Count provider definitions for `name` in the current document and collect
/// conflicting provider files from other documents and cached project state.
fn provider_conflicts_for(state: &WorkspaceState, uri: &Uri, name: &str) -> (usize, Vec<PathBuf>) {
	let mut current_count = 0;
	let mut other_files: Vec<PathBuf> = Vec::new();

	// Track conflicts by URI as well as path: `Uri::to_file_path` strips the
	// leading slash of host-less file URIs on Windows, so document-derived
	// paths would never equal stored project paths there. URI comparison is
	// stable across platforms.
	let mut other_uris: Vec<Uri> = Vec::new();

	for (doc_uri, doc) in &state.documents {
		if !doc_uri.path().as_str().ends_with(".t.md") {
			continue;
		}

		let count = doc
			.blocks
			.iter()
			.filter(|block| block.r#type == BlockType::Provider && block.name == name)
			.count();
		if count == 0 {
			continue;
		}

		if doc_uri == uri {
			current_count += count;
			continue;
		}

		if !other_uris.contains(doc_uri) {
			other_uris.push(doc_uri.clone());
			other_files.push(doc_uri.to_file_path().map_or_else(
				|| PathBuf::from(doc_uri.path().as_str()),
				std::borrow::Cow::into_owned,
			));
		}
	}

	if let Some(provider) = state.ctx.project.providers.get(name) {
		if let Some(provider_uri) = path_to_uri(&provider.file) {
			if provider_uri != *uri && !other_uris.contains(&provider_uri) {
				other_uris.push(provider_uri);
				other_files.push(provider.file.clone());
			}
		} else {
			let current_file = uri.to_file_path().map(std::borrow::Cow::into_owned);
			if current_file
				.as_ref()
				.is_none_or(|file| *file != provider.file)
				&& !other_files.contains(&provider.file)
			{
				other_files.push(provider.file.clone());
			}
		}
	}

	(current_count, other_files)
}

/// Convert an mdt `Point` (1-indexed line, 1-indexed column) to an LSP
/// `Position` (0-indexed).
fn to_lsp_position(point: &mdt_core::Point) -> Position {
	Position {
		line: point.line.saturating_sub(1) as u32,
		character: point.column.saturating_sub(1) as u32,
	}
}

/// Length of `text` in UTF-16 code units, the unit LSP positions count.
fn utf16_len(text: &str) -> u32 {
	text.chars().map(|ch| ch.len_utf16() as u32).sum()
}

/// Convert an mdt `Position` to an LSP `Range`.
fn to_lsp_range(pos: &mdt_core::Position) -> Range {
	Range {
		start: to_lsp_position(&pos.start),
		end: to_lsp_position(&pos.end),
	}
}

/// Convert an LSP `Position` (0-indexed line, character in UTF-16 code units)
/// to a byte offset within `content`. Returns `None` if the position is out of
/// bounds.
fn lsp_position_to_offset(content: &str, position: Position) -> Option<usize> {
	let mut offset = 0;
	for (i, line) in content.split('\n').enumerate() {
		if i == position.line as usize {
			// LSP character offsets are UTF-16 code units, so convert to a
			// byte index instead of slicing by code units.
			return utf16_col_to_byte_offset(line, position.character).map(|col| offset + col);
		}
		offset += line.len() + 1; // +1 for '\n'
	}
	None
}

/// Convert a UTF-16 code-unit column to a byte offset within a single line.
///
/// Returns `None` when `character` lands inside a multi-byte character (e.g.
/// between the surrogate halves of an emoji) or past the end of the line —
/// such positions do not map to a character boundary.
fn utf16_col_to_byte_offset(line: &str, character: u32) -> Option<usize> {
	let mut utf16_offset = 0u32;
	for (byte_idx, ch) in line.char_indices() {
		if utf16_offset == character {
			return Some(byte_idx);
		}
		utf16_offset += ch.len_utf16() as u32;
	}
	(utf16_offset == character).then_some(line.len())
}

/// The MDT language server.
#[derive(Debug)]
pub struct MdtLanguageServer {
	client: Client,
	state: RwLock<WorkspaceState>,
}

impl MdtLanguageServer {
	pub fn new(client: Client) -> Self {
		Self {
			client,
			state: RwLock::new(WorkspaceState::default()),
		}
	}

	/// Scan the project from disk on the blocking thread pool so the async
	/// runtime — and the stdio transport — keeps serving requests while the
	/// directory walk, data-source scripts, and file reads run.
	async fn scan_project_offload(root: &Path) -> Result<ProjectContext, String> {
		let root = root.to_path_buf();
		tokio::task::spawn_blocking(move || scan_project_with_config(&root))
			.await
			.map_err(|e| format!("scan task failed: {e}"))?
			.map_err(|e| e.to_string())
	}

	/// Publish diagnostics for a single document.
	async fn publish_diagnostics_for(&self, uri: &Uri) {
		let diagnostics = {
			let state = self.state.read().await;
			compute_diagnostics(&state, uri)
		};

		self.client
			.publish_diagnostics(uri.clone(), diagnostics, None)
			.await;
	}

	/// Handle a document being opened or changed — parse it and publish
	/// diagnostics.
	async fn on_document_change(&self, uri: &Uri, content: String) {
		{
			let mut state = self.state.write().await;
			state.parse_document(uri, content);
		}
		self.publish_diagnostics_for(uri).await;
	}
}

impl LanguageServer for MdtLanguageServer {
	async fn initialize(&self, params: InitializeParams) -> LspResult<InitializeResult> {
		// Determine workspace root — prefer `workspace_folders` (modern LSP),
		// fall back to the deprecated `root_uri` for older clients.
		let root = params
			.workspace_folders
			.as_ref()
			.and_then(|folders| folders.first())
			.and_then(|folder| folder.uri.to_file_path().map(std::borrow::Cow::into_owned))
			.or_else(|| {
				#[allow(deprecated)]
				params
					.root_uri
					.as_ref()
					.and_then(|uri| uri.to_file_path().map(std::borrow::Cow::into_owned))
			});

		// Scan off the async runtime (the walk reads every managed file and
		// may run config-declared data scripts), then apply under a short
		// write lock.
		let scan = match &root {
			Some(root) => Some(Self::scan_project_offload(root).await),
			None => None,
		};

		{
			let mut state = self.state.write().await;
			state.root = root;
			match scan {
				Some(Ok(ctx)) => state.apply_scan(ctx),
				Some(Err(e)) => tracing::error!("failed to scan project: {e}"),
				None => {}
			}
		}

		Ok(InitializeResult {
			capabilities: ServerCapabilities {
				text_document_sync: Some(TextDocumentSyncCapability::Kind(
					TextDocumentSyncKind::INCREMENTAL,
				)),
				hover_provider: Some(HoverProviderCapability::Simple(true)),
				completion_provider: Some(CompletionOptions {
					trigger_characters: Some(vec![
						"=".to_string(),
						"@".to_string(),
						"|".to_string(),
					]),
					..Default::default()
				}),
				definition_provider: Some(OneOf::Left(true)),
				references_provider: Some(OneOf::Left(true)),
				rename_provider: Some(OneOf::Right(RenameOptions {
					prepare_provider: Some(true),
					work_done_progress_options: WorkDoneProgressOptions {
						work_done_progress: None,
					},
				})),
				document_symbol_provider: Some(OneOf::Left(true)),
				code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
				..Default::default()
			},
			server_info: Some(ServerInfo {
				name: "mdt-lsp".to_string(),
				version: Some(env!("CARGO_PKG_VERSION").to_string()),
			}),
			offset_encoding: None,
		})
	}

	async fn initialized(&self, _: InitializedParams) {
		self.client
			.log_message(MessageType::INFO, "mdt language server initialized")
			.await;
	}

	async fn shutdown(&self) -> LspResult<()> {
		Ok(())
	}

	async fn did_open(&self, params: DidOpenTextDocumentParams) {
		let uri = params.text_document.uri;
		let content = params.text_document.text;
		self.on_document_change(&uri, content).await;
	}

	async fn did_change(&self, params: DidChangeTextDocumentParams) {
		let uri = params.text_document.uri;

		// Get the current document content to apply incremental changes to.
		let current_content = {
			let state = self.state.read().await;
			state.documents.get(&uri).map(|doc| doc.content.clone())
		};

		let Some(mut content) = current_content else {
			// Document not tracked yet — use the last change as full content.
			if let Some(change) = params.content_changes.into_iter().next_back() {
				self.on_document_change(&uri, change.text).await;
			}
			return;
		};

		// Apply each content change in order. With INCREMENTAL sync, each
		// change has a `range` indicating the region to replace. If `range`
		// is `None`, treat it as a full content replacement (backward compat).
		for change in params.content_changes {
			if let Some(range) = change.range {
				let start = lsp_position_to_offset(&content, range.start);
				let end = lsp_position_to_offset(&content, range.end);
				if let (Some(start), Some(end)) = (start, end) {
					if start <= end {
						content.replace_range(start..end, &change.text);
					} else {
						tracing::warn!(
							"dropping incremental change with inverted range for {}",
							uri.path().as_str()
						);
					}
				} else {
					// Silently dropping the change would desync the server's
					// document from the editor's.
					tracing::warn!(
						"dropping incremental change with unmappable range for {}",
						uri.path().as_str()
					);
				}
			} else {
				content = change.text;
			}
		}

		self.on_document_change(&uri, content).await;
	}

	async fn did_save(&self, params: DidSaveTextDocumentParams) {
		let uri = &params.text_document.uri;
		let is_config = uri.path().as_str().ends_with("mdt.toml");

		if is_config {
			// Config changed — full rescan needed for data and exclude
			// changes. Scan off the runtime, then apply under a short lock.
			let root = { self.state.read().await.root.clone() };
			if let Some(root) = root {
				match Self::scan_project_offload(&root).await {
					Ok(ctx) => {
						let mut state = self.state.write().await;
						state.apply_scan(ctx);
					}
					Err(e) => tracing::error!("failed to scan project: {e}"),
				}
			}
		} else {
			// Incrementally update this document's providers/consumers.
			{
				let mut state = self.state.write().await;
				state.update_document_in_project(uri);
			}
		}

		self.publish_diagnostics_for(uri).await;
	}

	async fn did_close(&self, params: DidCloseTextDocumentParams) {
		let uri = params.text_document.uri;
		{
			let mut state = self.state.write().await;
			state.documents.remove(&uri);
		}
		// Clear diagnostics for the closed document.
		self.client.publish_diagnostics(uri, Vec::new(), None).await;
	}

	async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
		let uri = &params.text_document_position_params.text_document.uri;
		let position = params.text_document_position_params.position;

		let state = self.state.read().await;
		Ok(compute_hover(&state, uri, position))
	}

	async fn completion(&self, params: CompletionParams) -> LspResult<Option<CompletionResponse>> {
		let uri = &params.text_document_position.text_document.uri;
		let position = params.text_document_position.position;

		let state = self.state.read().await;
		let items = compute_completions(&state, uri, position);

		if items.is_empty() {
			Ok(None)
		} else {
			Ok(Some(CompletionResponse::Array(items)))
		}
	}

	async fn goto_definition(
		&self,
		params: GotoDefinitionParams,
	) -> LspResult<Option<GotoDefinitionResponse>> {
		let uri = &params.text_document_position_params.text_document.uri;
		let position = params.text_document_position_params.position;

		let state = self.state.read().await;
		Ok(compute_goto_definition(&state, uri, position))
	}

	async fn document_symbol(
		&self,
		params: DocumentSymbolParams,
	) -> LspResult<Option<DocumentSymbolResponse>> {
		let uri = &params.text_document.uri;

		let state = self.state.read().await;
		let symbols = compute_document_symbols(&state, uri);

		if symbols.is_empty() {
			Ok(None)
		} else {
			Ok(Some(DocumentSymbolResponse::Nested(symbols)))
		}
	}

	async fn code_action(&self, params: CodeActionParams) -> LspResult<Option<CodeActionResponse>> {
		let uri = &params.text_document.uri;
		let range = params.range;

		let state = self.state.read().await;
		let actions = compute_code_actions(&state, uri, range);

		if actions.is_empty() {
			Ok(None)
		} else {
			Ok(Some(actions))
		}
	}

	async fn references(&self, params: ReferenceParams) -> LspResult<Option<Vec<Location>>> {
		let uri = &params.text_document_position.text_document.uri;
		let position = params.text_document_position.position;

		let state = self.state.read().await;
		Ok(compute_references(&state, uri, position))
	}

	async fn prepare_rename(
		&self,
		params: TextDocumentPositionParams,
	) -> LspResult<Option<PrepareRenameResponse>> {
		let uri = &params.text_document.uri;
		let position = params.position;

		let state = self.state.read().await;
		Ok(compute_prepare_rename(&state, uri, position))
	}

	async fn rename(&self, params: RenameParams) -> LspResult<Option<WorkspaceEdit>> {
		let uri = &params.text_document_position.text_document.uri;
		let position = params.text_document_position.position;
		let new_name = &params.new_name;

		let state = self.state.read().await;
		Ok(compute_rename(&state, uri, position, new_name))
	}
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

/// The content currently between a consumer or inline block's tags in the
/// open document `doc`, and what `mdt update` would write there.
///
/// The expected content comes from [`expected_consumer_content`], the same
/// computation `mdt check` and `mdt update` use (data, transformers,
/// `[padding]`, and the closing tag's comment prefix). The document's
/// in-memory text stands in for the file on disk, so unsaved edits are
/// checked as they are typed.
fn expected_block_content(
	ctx: &ProjectContext,
	uri: &Uri,
	doc: &DocumentState,
	block: &Block,
) -> (String, ExpectedContent) {
	let consumer = ConsumerEntry {
		block: block.clone(),
		// The path decides markdown vs source-file closing-tag prefixes.
		file: document_path(uri),
		content: extract_content_between_tags(&doc.content, block),
	};
	let expected = expected_consumer_content(ctx, &consumer, &doc.content);
	// mdt compares LF text; editors may hold CRLF.
	(normalize_line_endings(&consumer.content), expected)
}

/// The file path of a document, for path-based decisions.
fn document_path(uri: &Uri) -> PathBuf {
	uri.to_file_path().map_or_else(
		|| PathBuf::from(uri.path().as_str()),
		std::borrow::Cow::into_owned,
	)
}

/// Whether stale checks for this document can match `mdt check`. With a
/// `[[formatters]]` entry for the file, `mdt check` compares formatted
/// output, which the language server does not compute on every keystroke.
fn can_check_staleness(ctx: &ProjectContext, uri: &Uri) -> bool {
	!formatter_applies(ctx, &document_path(uri))
}

/// The LSP position of a byte offset in `content`. LSP counts characters in
/// UTF-16 code units, while mdt positions count bytes.
fn offset_to_lsp_position(content: &str, offset: usize) -> Position {
	let before = &content[..offset.min(content.len())];
	let line_start = before.rfind('\n').map_or(0, |index| index + 1);
	Position {
		line: before.matches('\n').count() as u32,
		character: utf16_len(&before[line_start..]),
	}
}

/// How diagnostics and hovers name a block of the given type.
fn block_label(block_type: BlockType) -> &'static str {
	match block_type {
		BlockType::Provider => "Provider block",
		BlockType::Inline => "Inline block",
		BlockType::Consumer => "Consumer block",
		_ => "Block",
	}
}

/// The diagnostic for a consumer or inline block whose content differs from
/// what `mdt update` would write. `data.expected_content` carries the
/// replacement text.
fn stale_diagnostic(block: &Block, expected: &str) -> Diagnostic {
	Diagnostic {
		range: to_lsp_range(&block.opening),
		severity: Some(DiagnosticSeverity::WARNING),
		source: Some("mdt".to_string()),
		message: format!(
			"{} `{}` is out of date",
			block_label(block.r#type),
			block.name
		),
		data: Some(serde_json::json!({
			"kind": "stale",
			"block_name": block.name,
			"expected_content": expected,
		})),
		..Default::default()
	}
}

/// Convert a parser diagnostic to an LSP diagnostic at the position it
/// reports. Returns `None` for kinds this server does not know yet.
fn parse_diagnostic_to_lsp(diagnostic: &ParseDiagnostic) -> Option<Diagnostic> {
	let (line, column, severity, message) = match diagnostic {
		ParseDiagnostic::UnclosedBlock { name, line, column } => {
			(
				line,
				column,
				DiagnosticSeverity::ERROR,
				format!("Missing closing tag for block `{name}`"),
			)
		}
		ParseDiagnostic::UnknownTransformer { name, line, column } => {
			(
				line,
				column,
				DiagnosticSeverity::ERROR,
				format!("Unknown transformer `{name}`"),
			)
		}
		ParseDiagnostic::InvalidTransformerArgs {
			name,
			expected,
			got,
			line,
			column,
		} => {
			(
				line,
				column,
				DiagnosticSeverity::ERROR,
				format!("Transformer `{name}` expects {expected} argument(s), got {got}"),
			)
		}
		ParseDiagnostic::UnmatchedClosingTag { name, line, column } => {
			(
				line,
				column,
				DiagnosticSeverity::ERROR,
				format!(
					"Closing tag `{{/{name}}}` has no matching opening tag. Check both tags for \
					 typos."
				),
			)
		}
		ParseDiagnostic::InvalidTag { tag, line, column } => {
			(
				line,
				column,
				DiagnosticSeverity::ERROR,
				format!(
					"`{tag}` looks like an mdt tag but cannot be parsed, so mdt ignores it. The \
					 sigil (`@`, `=`, `~`, `/`) must directly follow `{{`, and block names must \
					 match `[A-Za-z_][A-Za-z0-9_-]*`."
				),
			)
		}
		ParseDiagnostic::NestedBlock {
			outer,
			inner,
			line,
			column,
		} => {
			(
				line,
				column,
				DiagnosticSeverity::ERROR,
				format!(
					"Block `{inner}` is inside block `{outer}`, whose content `mdt update` \
					 replaces; move `{inner}` outside `{outer}`."
				),
			)
		}
		_ => return None,
	};
	let position = Position {
		line: line.saturating_sub(1) as u32,
		character: column.saturating_sub(1) as u32,
	};
	Some(Diagnostic {
		range: Range {
			start: position,
			end: position,
		},
		severity: Some(severity),
		source: Some("mdt".to_string()),
		message,
		..Default::default()
	})
}

/// Compute diagnostics for a single document, matching what `mdt check`
/// reports for it:
/// - Parse problems: unclosed blocks, closing tags without an opening tag,
///   tag-like comments that do not parse, blocks nested inside consumers,
///   unknown transformers, and invalid transformer arguments
/// - Stale consumer and inline blocks (content differs from what `mdt update`
///   would write)
/// - Blocks whose template fails to render
/// - Consumers without a provider, with did-you-mean suggestions
/// - Duplicate and unused providers, and providers outside `*.t.md` files
fn compute_diagnostics(state: &WorkspaceState, uri: &Uri) -> Vec<Diagnostic> {
	let Some(doc) = state.documents.get(uri) else {
		return Vec::new();
	};

	let mut diagnostics: Vec<Diagnostic> = doc
		.parse_diagnostics
		.iter()
		.filter_map(parse_diagnostic_to_lsp)
		.collect();
	let is_template = uri.path().as_str().ends_with(".t.md");
	let check_staleness = can_check_staleness(&state.ctx, uri);

	for block in &doc.blocks {
		match block.r#type {
			BlockType::Consumer | BlockType::Inline => {
				let (current, expected) = expected_block_content(&state.ctx, uri, doc, block);
				match expected {
					ExpectedContent::Rendered(expected) => {
						if check_staleness
							&& !content_matches(&current, &expected, &state.ctx.comparison)
						{
							diagnostics.push(stale_diagnostic(block, &expected));
						}
					}
					ExpectedContent::NoProvider => {
						diagnostics.push(Diagnostic {
							range: to_lsp_range(&block.opening),
							severity: Some(DiagnosticSeverity::ERROR),
							source: Some("mdt".to_string()),
							message: missing_provider_message(state, &block.name),
							..Default::default()
						});
					}
					ExpectedContent::RenderFailed(message) => {
						diagnostics.push(Diagnostic {
							range: to_lsp_range(&block.opening),
							severity: Some(DiagnosticSeverity::ERROR),
							source: Some("mdt".to_string()),
							message: format!(
								"{} `{}` failed to render: {message}",
								block_label(block.r#type),
								block.name
							),
							..Default::default()
						});
					}
					_ => {}
				}
			}
			BlockType::Provider => {
				if is_template {
					let (current_count, other_files) =
						provider_conflicts_for(state, uri, &block.name);
					if current_count > 1 || !other_files.is_empty() {
						let mut details = Vec::new();
						if current_count > 1 {
							details.push("multiple definitions in this file".to_string());
						}
						details.extend(
							other_files
								.iter()
								.map(|path| format!("`{}`", path.display())),
						);

						diagnostics.push(Diagnostic {
							range: to_lsp_range(&block.opening),
							severity: Some(DiagnosticSeverity::ERROR),
							source: Some("mdt".to_string()),
							message: format!(
								"Duplicate provider block `{}`. Provider names are global; \
								 conflicts found in {}",
								block.name,
								details.join(", ")
							),
							..Default::default()
						});
						continue;
					}

					// Unused providers do not fail `mdt check`; they are
					// warnings.
					let has_consumers = state.ctx.project.consumers.iter().any(|consumer| {
						consumer.block.r#type == BlockType::Consumer
							&& consumer.block.name == block.name
					});
					if !has_consumers {
						diagnostics.push(Diagnostic {
							range: to_lsp_range(&block.opening),
							severity: Some(DiagnosticSeverity::WARNING),
							source: Some("mdt".to_string()),
							message: format!("Provider block `{}` has no consumers", block.name),
							..Default::default()
						});
					}
				} else {
					diagnostics.push(Diagnostic {
						range: to_lsp_range(&block.opening),
						severity: Some(DiagnosticSeverity::WARNING),
						source: Some("mdt".to_string()),
						message: format!(
							"Provider block `{}` is only recognized in *.t.md template files",
							block.name
						),
						..Default::default()
					});
				}
			}
			_ => {}
		}
	}

	diagnostics
}

/// The message for a consumer whose name matches no provider, suggesting
/// similarly named providers.
fn missing_provider_message(state: &WorkspaceState, name: &str) -> String {
	let suggestions = suggest_similar_provider_names(
		name,
		state.ctx.project.providers.keys().map(String::as_str),
	);
	if suggestions.is_empty() {
		return format!("No provider found for consumer block `{name}`");
	}

	format!(
		"No provider found for consumer block `{name}`. Did you mean: {}?",
		suggestions
			.iter()
			.map(|suggestion| format!("`{suggestion}`"))
			.collect::<Vec<_>>()
			.join(", ")
	)
}

// ---------------------------------------------------------------------------
// Hover
// ---------------------------------------------------------------------------

/// Find the block at a given cursor position.
fn find_block_at_position(blocks: &[Block], position: Position) -> Option<&Block> {
	for block in blocks {
		let opening_range = to_lsp_range(&block.opening);
		if position_in_range(position, opening_range) {
			return Some(block);
		}
	}
	None
}

/// Check if a position is within a range.
fn position_in_range(pos: Position, range: Range) -> bool {
	if pos.line < range.start.line || pos.line > range.end.line {
		return false;
	}
	if pos.line == range.start.line && pos.character < range.start.character {
		return false;
	}
	if pos.line == range.end.line && pos.character > range.end.character {
		return false;
	}
	true
}

/// Compute hover information at a position.
fn compute_hover(state: &WorkspaceState, uri: &Uri, position: Position) -> Option<Hover> {
	let doc = state.documents.get(uri)?;
	let block = find_block_at_position(&doc.blocks, position)?;

	let contents = match block.r#type {
		BlockType::Consumer => {
			let mut parts = Vec::new();
			parts.push(format!("**Consumer block:** `{}`", block.name));

			if let Some(provider) = state.ctx.project.providers.get(&block.name) {
				parts.push(format!(
					"\n**Provider source:** `{}`",
					provider.file.display()
				));
				parts.extend(transformer_chain(block));
			}

			match expected_block_content(&state.ctx, uri, doc, block).1 {
				ExpectedContent::Rendered(expected) => parts.push(content_preview(&expected)),
				ExpectedContent::NoProvider => {
					parts.push("\n*No matching provider found*".to_string());
				}
				ExpectedContent::RenderFailed(message) => {
					parts.push(format!("\n*Failed to render provider:* {message}"));
				}
				_ => {}
			}

			parts.join("")
		}
		BlockType::Inline => {
			let mut parts = Vec::new();
			parts.push(format!("**Inline block:** `{}`", block.name));

			if let Some(template) = block.arguments.first() {
				parts.push(format!("\n**Template:** `{template}`"));
				parts.extend(transformer_chain(block));
				match expected_block_content(&state.ctx, uri, doc, block).1 {
					ExpectedContent::Rendered(expected) => parts.push(content_preview(&expected)),
					ExpectedContent::RenderFailed(message) => {
						parts.push(format!("\n*Failed to render inline template:* {message}"));
					}
					_ => {}
				}
			} else {
				parts.push("\n*Missing inline template argument*".to_string());
			}

			parts.join("")
		}
		BlockType::Provider => {
			let mut parts = Vec::new();
			parts.push(format!("**Provider block:** `{}`", block.name));

			let content = extract_content_between_tags(&doc.content, block);
			let consumer_count = state
				.ctx
				.project
				.consumers
				.iter()
				.filter(|c| c.block.name == block.name)
				.count();

			parts.push(format!("\n**Referenced by:** {consumer_count} consumer(s)"));

			// List consumer locations
			let consumer_files: Vec<String> = state
				.ctx
				.project
				.consumers
				.iter()
				.filter(|c| c.block.name == block.name)
				.map(|c| format!("`{}`", c.file.display()))
				.collect();

			if !consumer_files.is_empty() {
				parts.push(format!("\n**Consumers in:** {}", consumer_files.join(", ")));
			}

			parts.push(content_preview(&content));

			parts.join("")
		}
		_ => return None,
	};

	Some(Hover {
		contents: HoverContents::Markup(MarkupContent {
			kind: MarkupKind::Markdown,
			value: contents,
		}),
		range: Some(to_lsp_range(&block.opening)),
	})
}

/// The `**Transformers:**` hover line for a block with a transformer chain.
fn transformer_chain(block: &Block) -> Option<String> {
	if block.transformers.is_empty() {
		return None;
	}

	let names: Vec<String> = block
		.transformers
		.iter()
		.map(|transformer| transformer.r#type.to_string())
		.collect();
	Some(format!("\n**Transformers:** {}", names.join(" | ")))
}

/// A hover section showing `text` in a code block, without surrounding
/// whitespace.
fn content_preview(text: &str) -> String {
	format!("\n---\n\n```\n{}\n```", text.trim())
}

// ---------------------------------------------------------------------------
// Completions
// ---------------------------------------------------------------------------

/// Compute completion items at a position.
fn compute_completions(
	state: &WorkspaceState,
	uri: &Uri,
	position: Position,
) -> Vec<CompletionItem> {
	let Some(doc) = state.documents.get(uri) else {
		return Vec::new();
	};

	// Check if we're inside an HTML comment context by looking at text before
	// cursor.
	let line_idx = position.line as usize;

	let lines: Vec<&str> = doc.content.lines().collect();
	let Some(line) = lines.get(line_idx) else {
		return Vec::new();
	};

	// LSP character offsets are UTF-16 code units; slicing the line by them
	// directly panics whenever the cursor follows a multi-byte character.
	let before_cursor = match utf16_col_to_byte_offset(line, position.character) {
		Some(byte_offset) => &line[..byte_offset],
		None => line,
	};

	// Check if we're in a context where block name completion makes sense:
	// after `{=`, `{@`, or `{/`
	let in_tag_context = before_cursor.contains("{=")
		|| before_cursor.contains("{@")
		|| before_cursor.contains("{~")
		|| before_cursor.contains("{/");

	// Check if we're after a pipe for transformer completion.
	let in_transformer_context = {
		// Look for `|` after a `{=name` pattern on the current line.
		if let Some(tag_start) = before_cursor.rfind("{=") {
			let after_tag = &before_cursor[tag_start..];
			after_tag.contains('|')
		} else {
			false
		}
	};

	if in_transformer_context {
		return transformer_completions();
	}

	if in_tag_context {
		return block_name_completions(state);
	}

	Vec::new()
}

/// Generate completion items for all known block names.
fn block_name_completions(state: &WorkspaceState) -> Vec<CompletionItem> {
	state
		.ctx
		.project
		.providers
		.iter()
		.map(|(name, entry)| {
			CompletionItem {
				label: name.clone(),
				kind: Some(CompletionItemKind::REFERENCE),
				detail: Some(format!("Provider from {}", entry.file.display())),
				documentation: Some(Documentation::MarkupContent(MarkupContent {
					kind: MarkupKind::Markdown,
					value: format!("```\n{}\n```", entry.content.trim()),
				})),
				..Default::default()
			}
		})
		.collect()
}

/// Generate completion items for transformer names.
fn transformer_completions() -> Vec<CompletionItem> {
	let transformers = [
		("trim", "Remove leading and trailing whitespace"),
		("trimStart", "Remove leading whitespace"),
		("trimEnd", "Remove trailing whitespace"),
		(
			"indent",
			"Indent each line with a string. Usage: `indent:\"  \"`",
		),
		(
			"prefix",
			"Add a prefix before content. Usage: `prefix:\"// \"`",
		),
		(
			"suffix",
			"Add a suffix after content. Usage: `suffix:\"\\n\"`",
		),
		(
			"linePrefix",
			"Add a prefix before each line. Usage: `linePrefix:\"/// \"`",
		),
		(
			"lineSuffix",
			"Add a suffix after each line. Usage: `lineSuffix:\" \\\\\"`",
		),
		("wrap", "Wrap content with a string. Usage: `wrap:\"**\"`"),
		(
			"codeBlock",
			"Wrap in a fenced code block. Usage: `codeBlock:\"ts\"`",
		),
		("code", "Wrap in inline code backticks"),
		(
			"replace",
			"Replace a substring. Usage: `replace:\"old\":\"new\"`",
		),
		(
			"if",
			"Conditionally include content based on a data value. Usage: \
			 `if:\"config.features.enabled\"`",
		),
	];

	transformers
		.iter()
		.enumerate()
		.map(|(i, (name, desc))| {
			CompletionItem {
				label: (*name).to_string(),
				kind: Some(CompletionItemKind::FUNCTION),
				detail: Some((*desc).to_string()),
				sort_text: Some(format!("{i:02}")),
				..Default::default()
			}
		})
		.collect()
}

// ---------------------------------------------------------------------------
// Go to Definition
// ---------------------------------------------------------------------------

/// Compute go-to-definition: consumer → provider.
/// Convert a workspace file path to a file URI.
///
/// `Uri::from_file_path` requires an absolute path (with a drive letter on
/// Windows). Scanned project files always satisfy this, but synthetic
/// Unix-style paths such as `/tmp/test/readme.md` are not absolute on
/// Windows, so build the URI text directly in that case to keep behavior
/// identical across platforms.
fn path_to_uri(path: &Path) -> Option<Uri> {
	if let Some(uri) = Uri::from_file_path(path) {
		return Some(uri);
	}

	let text = path.display().to_string();
	if text.starts_with('/') {
		return format!("file://{text}").parse().ok();
	}

	None
}

fn compute_goto_definition(
	state: &WorkspaceState,
	uri: &Uri,
	position: Position,
) -> Option<GotoDefinitionResponse> {
	let doc = state.documents.get(uri)?;
	let block = find_block_at_position(&doc.blocks, position)?;

	match block.r#type {
		BlockType::Consumer => {
			// Navigate to the provider definition.
			let provider = state.ctx.project.providers.get(&block.name)?;
			let target_uri = path_to_uri(&provider.file)?;
			let target_range = to_lsp_range(&provider.block.opening);

			Some(GotoDefinitionResponse::Scalar(Location {
				uri: target_uri,
				range: target_range,
			}))
		}
		BlockType::Provider => {
			// Navigate to all consumers of this provider.
			let locations: Vec<Location> = state
				.ctx
				.project
				.consumers
				.iter()
				.filter(|c| c.block.name == block.name)
				.filter_map(|c| {
					let consumer_uri = path_to_uri(&c.file)?;
					Some(Location {
						uri: consumer_uri,
						range: to_lsp_range(&c.block.opening),
					})
				})
				.collect();

			if locations.is_empty() {
				None
			} else if locations.len() == 1 {
				Some(GotoDefinitionResponse::Scalar(
					locations.into_iter().next()?,
				))
			} else {
				Some(GotoDefinitionResponse::Array(locations))
			}
		}
		_ => None,
	}
}

// ---------------------------------------------------------------------------
// Document Symbols
// ---------------------------------------------------------------------------

/// Compute document symbols for the outline view using `DocumentSymbol`
/// (hierarchical, non-deprecated).
fn compute_document_symbols(state: &WorkspaceState, uri: &Uri) -> Vec<DocumentSymbol> {
	let Some(doc) = state.documents.get(uri) else {
		return Vec::new();
	};

	doc.blocks
		.iter()
		.map(|block| {
			let kind = match block.r#type {
				BlockType::Provider => SymbolKind::CLASS,
				_ => SymbolKind::VARIABLE,
			};
			let prefix = match block.r#type {
				BlockType::Provider => "@",
				BlockType::Consumer => "=",
				BlockType::Inline => "~",
				_ => "?",
			};
			let full_range = Range {
				start: to_lsp_position(&block.opening.start),
				end: to_lsp_position(&block.closing.end),
			};
			let selection_range = to_lsp_range(&block.opening);

			#[allow(deprecated)]
			DocumentSymbol {
				name: format!("{prefix}{}", block.name),
				detail: None,
				kind,
				tags: None,
				deprecated: None,
				range: full_range,
				selection_range,
				children: None,
			}
		})
		.collect()
}

// ---------------------------------------------------------------------------
// Code Actions
// ---------------------------------------------------------------------------

/// Compute code actions for a range. Offers "Update block" for consumer and
/// inline blocks whose content differs from what `mdt update` would write.
/// Blocks that fail to render or have no provider get no fix: `mdt update`
/// leaves them untouched too.
fn compute_code_actions(
	state: &WorkspaceState,
	uri: &Uri,
	range: Range,
) -> Vec<CodeActionOrCommand> {
	let Some(doc) = state.documents.get(uri) else {
		return Vec::new();
	};

	let mut actions = Vec::new();

	for block in &doc.blocks {
		if !matches!(block.r#type, BlockType::Consumer | BlockType::Inline) {
			continue;
		}

		// Check if the user's selection/cursor overlaps with this block.
		if !ranges_overlap(
			range,
			Range {
				start: to_lsp_position(&block.opening.start),
				end: to_lsp_position(&block.closing.end),
			},
		) {
			continue;
		}

		if !can_check_staleness(&state.ctx, uri) {
			continue;
		}
		let (current, ExpectedContent::Rendered(expected)) =
			expected_block_content(&state.ctx, uri, doc, block)
		else {
			continue;
		};
		if content_matches(&current, &expected, &state.ctx.comparison) {
			continue;
		}

		let diagnostic = stale_diagnostic(block, &expected);
		// Replace exactly the bytes between the tags, keeping the document's
		// line endings.
		let new_text = if doc.content.contains("\r\n") {
			expected.replace('\n', "\r\n")
		} else {
			expected
		};
		let edit = TextEdit {
			range: Range {
				start: offset_to_lsp_position(&doc.content, block.opening.end.offset),
				end: offset_to_lsp_position(&doc.content, block.closing.start.offset),
			},
			new_text,
		};

		let mut changes = HashMap::new();
		changes.insert(uri.clone(), vec![edit]);

		actions.push(CodeActionOrCommand::CodeAction(CodeAction {
			title: format!("Update block `{}`", block.name),
			kind: Some(CodeActionKind::QUICKFIX),
			diagnostics: Some(vec![diagnostic]),
			edit: Some(WorkspaceEdit {
				changes: Some(changes),
				..Default::default()
			}),
			..Default::default()
		}));
	}

	actions
}

/// Check if two ranges overlap.
fn ranges_overlap(a: Range, b: Range) -> bool {
	!(a.end.line < b.start.line
		|| (a.end.line == b.start.line && a.end.character < b.start.character)
		|| b.end.line < a.start.line
		|| (b.end.line == a.start.line && b.end.character < a.start.character))
}

// ---------------------------------------------------------------------------
// References
// ---------------------------------------------------------------------------

/// Compute references: return all locations that share the same block name.
/// If on a consumer, return the provider + all other consumers.
/// If on a provider, return all consumers (and the provider itself if
/// `include_declaration` would apply — but we always include all for
/// simplicity).
fn compute_references(
	state: &WorkspaceState,
	uri: &Uri,
	position: Position,
) -> Option<Vec<Location>> {
	let doc = state.documents.get(uri)?;
	let block = find_block_at_position(&doc.blocks, position)?;
	let name = &block.name;

	let mut locations = Vec::new();

	if block.r#type == BlockType::Inline {
		// Inline blocks only reference other inline blocks of the same name.
		for consumer in &state.ctx.project.consumers {
			if consumer.block.r#type == BlockType::Inline && consumer.block.name == *name {
				if let Some(consumer_uri) = path_to_uri(&consumer.file) {
					locations.push(Location {
						uri: consumer_uri,
						range: to_lsp_range(&consumer.block.opening),
					});
				}
			}
		}
	} else {
		// Include the provider location if it exists.
		if let Some(provider) = state.ctx.project.providers.get(name) {
			if let Some(provider_uri) = path_to_uri(&provider.file) {
				locations.push(Location {
					uri: provider_uri,
					range: to_lsp_range(&provider.block.opening),
				});
			}
		}

		// Include all consumer locations.
		for consumer in &state.ctx.project.consumers {
			if consumer.block.r#type == BlockType::Consumer && consumer.block.name == *name {
				if let Some(consumer_uri) = path_to_uri(&consumer.file) {
					locations.push(Location {
						uri: consumer_uri,
						range: to_lsp_range(&consumer.block.opening),
					});
				}
			}
		}
	}

	if locations.is_empty() {
		None
	} else {
		Some(locations)
	}
}

// ---------------------------------------------------------------------------
// Rename
// ---------------------------------------------------------------------------

/// Find the range of the block name within a tag, given the tag text and
/// the tag's starting LSP position. The name appears after `{@`, `{=`, `{~`, or
/// `{/` in the tag text.
fn find_name_range_in_tag(tag_text: &str, tag_start: Position, name: &str) -> Option<Range> {
	// Tags have the form: `<!-- ` + open/close marker + name + ` -->`.
	// Open markers: `{@` (provider), `{=` (consumer), `{~` (inline).
	// Close marker: `{/`.
	// We find one of these markers, then look for the name immediately after.
	let tag_prefix_patterns = ["{@", "{=", "{~", "{/"];
	let mut search_start = 0;

	for pattern in &tag_prefix_patterns {
		if let Some(pos) = tag_text[search_start..].find(pattern) {
			search_start = search_start + pos + pattern.len();
			break;
		}
	}

	// Find the name after the tag prefix.
	let name_start_in_tag = tag_text[search_start..].find(name)?;
	let name_byte_offset = search_start + name_start_in_tag;

	// Calculate the LSP position of the name by counting characters from
	// the tag start. LSP positions count UTF-16 code units, so measure the
	// prefix and name in code units rather than bytes.
	let before_name = &tag_text[..name_byte_offset];
	let lines_before: Vec<&str> = before_name.split('\n').collect();
	let newline_count = lines_before.len() - 1;

	let start_line = tag_start.line + newline_count as u32;
	let start_character = if newline_count > 0 {
		lines_before.last().map_or(0, |line| utf16_len(line))
	} else {
		tag_start.character + utf16_len(before_name)
	};

	let name_end = &tag_text[name_byte_offset..name_byte_offset + name.len()];
	let name_lines: Vec<&str> = name_end.split('\n').collect();
	let name_newlines = name_lines.len() - 1;

	let end_line = start_line + name_newlines as u32;
	let end_character = if name_newlines > 0 {
		name_lines.last().map_or(0, |line| utf16_len(line))
	} else {
		start_character + utf16_len(name)
	};

	Some(Range {
		start: Position {
			line: start_line,
			character: start_character,
		},
		end: Position {
			line: end_line,
			character: end_character,
		},
	})
}

/// Extract the text of a tag from the document content using the tag's mdt
/// `Position`.
fn extract_tag_text<'a>(content: &'a str, tag_pos: &mdt_core::Position) -> &'a str {
	let start = tag_pos.start.offset;
	let end = tag_pos.end.offset;
	if end <= content.len() && start <= end {
		&content[start..end]
	} else {
		""
	}
}

/// Compute `prepare_rename`: validate the cursor is on a block name and return
/// its range.
fn compute_prepare_rename(
	state: &WorkspaceState,
	uri: &Uri,
	position: Position,
) -> Option<PrepareRenameResponse> {
	let doc = state.documents.get(uri)?;
	let block = find_block_at_position(&doc.blocks, position)?;

	let tag_text = extract_tag_text(&doc.content, &block.opening);
	let name_range =
		find_name_range_in_tag(tag_text, to_lsp_position(&block.opening.start), &block.name)?;

	Some(PrepareRenameResponse::Range(name_range))
}

/// Compute rename: rename a block name across all provider and consumer tags.
fn compute_rename(
	state: &WorkspaceState,
	uri: &Uri,
	position: Position,
	new_name: &str,
) -> Option<WorkspaceEdit> {
	let doc = state.documents.get(uri)?;
	let block = find_block_at_position(&doc.blocks, position)?;
	let old_name = &block.name;

	let mut changes: HashMap<Uri, Vec<TextEdit>> = HashMap::new();

	// Collect all blocks to rename: the provider + all consumers with this
	// name.
	let mut blocks_to_rename: Vec<(&Block, &str, Uri)> = Vec::new();

	// Add the provider if it exists.
	if let Some(provider) = state.ctx.project.providers.get(old_name) {
		if let Some(provider_uri) = path_to_uri(&provider.file) {
			blocks_to_rename.push((&provider.block, "", provider_uri));
		}
	}

	// Add all consumers with this name.
	for consumer in &state.ctx.project.consumers {
		if consumer.block.name == *old_name {
			if let Some(consumer_uri) = path_to_uri(&consumer.file) {
				blocks_to_rename.push((&consumer.block, "", consumer_uri));
			}
		}
	}

	for (blk, _, blk_uri) in &blocks_to_rename {
		// Get the document content for this block's file.
		let content = if let Some(doc) = state.documents.get(blk_uri) {
			&doc.content
		} else {
			// For files not currently open, we need to read the file content
			// from the provider/consumer entry.
			continue;
		};

		let mut edits = Vec::new();

		// Rename in the opening tag.
		let open_text = extract_tag_text(content, &blk.opening);
		if let Some(range) =
			find_name_range_in_tag(open_text, to_lsp_position(&blk.opening.start), old_name)
		{
			edits.push(TextEdit {
				range,
				new_text: new_name.to_string(),
			});
		}

		// Rename in the closing tag.
		let close_text = extract_tag_text(content, &blk.closing);
		if let Some(range) =
			find_name_range_in_tag(close_text, to_lsp_position(&blk.closing.start), old_name)
		{
			edits.push(TextEdit {
				range,
				new_text: new_name.to_string(),
			});
		}

		if !edits.is_empty() {
			changes.entry(blk_uri.clone()).or_default().extend(edits);
		}
	}

	// Also handle files that are not currently open in the editor.
	// For the provider file, if not open we can try reading from disk via
	// the stored file path.
	if let Some(provider) = state.ctx.project.providers.get(old_name) {
		let provider_uri_opt = path_to_uri(&provider.file);
		if let Some(provider_uri) = provider_uri_opt {
			if !state.documents.contains_key(&provider_uri) {
				// Read file from disk.
				if let Ok(content) = std::fs::read_to_string(&provider.file) {
					let mut edits = Vec::new();

					let open_text = extract_tag_text(&content, &provider.block.opening);
					if let Some(range) = find_name_range_in_tag(
						open_text,
						to_lsp_position(&provider.block.opening.start),
						old_name,
					) {
						edits.push(TextEdit {
							range,
							new_text: new_name.to_string(),
						});
					}

					let close_text = extract_tag_text(&content, &provider.block.closing);
					if let Some(range) = find_name_range_in_tag(
						close_text,
						to_lsp_position(&provider.block.closing.start),
						old_name,
					) {
						edits.push(TextEdit {
							range,
							new_text: new_name.to_string(),
						});
					}

					if !edits.is_empty() {
						changes.entry(provider_uri).or_default().extend(edits);
					}
				}
			}
		}
	}

	// For consumer files not currently open.
	for consumer in &state.ctx.project.consumers {
		if consumer.block.name != *old_name {
			continue;
		}
		let consumer_uri_opt = path_to_uri(&consumer.file);
		if let Some(consumer_uri) = consumer_uri_opt {
			if !state.documents.contains_key(&consumer_uri) {
				if let Ok(content) = std::fs::read_to_string(&consumer.file) {
					let mut edits = Vec::new();

					let open_text = extract_tag_text(&content, &consumer.block.opening);
					if let Some(range) = find_name_range_in_tag(
						open_text,
						to_lsp_position(&consumer.block.opening.start),
						old_name,
					) {
						edits.push(TextEdit {
							range,
							new_text: new_name.to_string(),
						});
					}

					let close_text = extract_tag_text(&content, &consumer.block.closing);
					if let Some(range) = find_name_range_in_tag(
						close_text,
						to_lsp_position(&consumer.block.closing.start),
						old_name,
					) {
						edits.push(TextEdit {
							range,
							new_text: new_name.to_string(),
						});
					}

					if !edits.is_empty() {
						changes.entry(consumer_uri).or_default().extend(edits);
					}
				}
			}
		}
	}

	if changes.is_empty() {
		None
	} else {
		Some(WorkspaceEdit {
			changes: Some(changes),
			..Default::default()
		})
	}
}

/// Log to stderr (stdout carries the protocol), filtered by `MDT_LOG`.
///
/// `mdt lsp` runs inside the CLI, which may already have installed a global
/// subscriber (for example with `MDT_LOG=info mdt lsp`); that one is kept.
fn init_tracing() {
	let filter = EnvFilter::try_from_env("MDT_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
	let installed = fmt::Subscriber::builder()
		.with_env_filter(filter)
		.with_writer(std::io::stderr)
		.try_init();
	if installed.is_err() {
		tracing::debug!("keeping the tracing subscriber that is already installed");
	}
}

/// Start the LSP server on stdin/stdout. This is used by both the standalone
/// `mdt-lsp` binary and the `mdt lsp` CLI subcommand.
pub async fn run_server() {
	init_tracing();

	let stdin = tokio::io::stdin();
	let stdout = tokio::io::stdout();

	let (service, socket) = tower_lsp_server::LspService::new(MdtLanguageServer::new);
	tower_lsp_server::Server::new(stdin, stdout, socket)
		.serve(service)
		.await;
}

#[cfg(test)]
mod __tests;

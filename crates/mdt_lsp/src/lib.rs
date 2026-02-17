use std::collections::HashMap;
use std::path::PathBuf;

use mdt::Block;
use mdt::BlockType;
use mdt::TransformerType;
use mdt::apply_transformers;
use mdt::parse;
use mdt::parse_source;
use mdt::project::ConsumerEntry;
use mdt::project::ProviderEntry;
use mdt::project::extract_content_between_tags;
use mdt::project::scan_project_with_config;
use mdt::render_template;
use tokio::sync::RwLock;
use tower_lsp::Client;
use tower_lsp::LanguageServer;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::*;

/// State for a single open document.
#[derive(Debug, Clone)]
struct DocumentState {
	/// The full text content of the document.
	content: String,
	/// Parsed mdt blocks in this document.
	blocks: Vec<Block>,
}

/// Workspace-level state shared across all LSP requests.
#[derive(Debug, Default)]
struct WorkspaceState {
	/// The workspace root path.
	root: Option<PathBuf>,
	/// Open documents keyed by URI.
	documents: HashMap<Url, DocumentState>,
	/// Cached providers from the last project scan.
	providers: HashMap<String, ProviderEntry>,
	/// Cached consumers from the last project scan.
	consumers: Vec<ConsumerEntry>,
	/// Template data from mdt.toml config.
	data: HashMap<String, serde_json::Value>,
}

impl WorkspaceState {
	/// Rescan the project from disk. Called on initialize, save, and
	/// workspace changes.
	fn rescan_project(&mut self) {
		let Some(root) = &self.root else {
			return;
		};

		match scan_project_with_config(root) {
			Ok((project, data)) => {
				self.providers = project.providers;
				self.consumers = project.consumers;
				self.data = data;
			}
			Err(e) => {
				eprintln!("mdt-lsp: failed to scan project: {e}");
			}
		}
	}

	/// Parse a single document and update its cached state. Returns the
	/// parsed blocks.
	fn parse_document(&mut self, uri: &Url, content: String) -> Vec<Block> {
		let blocks = parse_document_content(uri, &content);
		self.documents.insert(
			uri.clone(),
			DocumentState {
				content,
				blocks: blocks.clone(),
			},
		);
		blocks
	}
}

/// Parse document content, choosing the right parser based on file extension.
fn parse_document_content(uri: &Url, content: &str) -> Vec<Block> {
	let is_markdown = uri
		.path()
		.rsplit('.')
		.next()
		.is_some_and(|ext| matches!(ext, "md" | "mdx" | "markdown"));

	let result = if is_markdown {
		parse(content)
	} else {
		parse_source(content)
	};

	result.unwrap_or_default()
}

/// Convert an mdt `Point` (1-indexed line, 1-indexed column) to an LSP
/// `Position` (0-indexed).
fn to_lsp_position(point: &mdt::Point) -> Position {
	Position {
		line: point.line.saturating_sub(1) as u32,
		character: point.column.saturating_sub(1) as u32,
	}
}

/// Convert an mdt `Position` to an LSP `Range`.
fn to_lsp_range(pos: &mdt::Position) -> Range {
	Range {
		start: to_lsp_position(&pos.start),
		end: to_lsp_position(&pos.end),
	}
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

	/// Publish diagnostics for a single document.
	async fn publish_diagnostics_for(&self, uri: &Url) {
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
	async fn on_document_change(&self, uri: &Url, content: String) {
		{
			let mut state = self.state.write().await;
			state.parse_document(uri, content);
		}
		self.publish_diagnostics_for(uri).await;
	}
}

#[tower_lsp::async_trait]
impl LanguageServer for MdtLanguageServer {
	async fn initialize(&self, params: InitializeParams) -> LspResult<InitializeResult> {
		// Determine workspace root
		let root = params
			.root_uri
			.as_ref()
			.and_then(|uri| uri.to_file_path().ok());

		{
			let mut state = self.state.write().await;
			state.root = root;
			state.rescan_project();
		}

		Ok(InitializeResult {
			capabilities: ServerCapabilities {
				text_document_sync: Some(TextDocumentSyncCapability::Kind(
					TextDocumentSyncKind::FULL,
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
		// We use Full sync, so the last change contains the entire document.
		if let Some(change) = params.content_changes.into_iter().next_back() {
			self.on_document_change(&uri, change.text).await;
		}
	}

	async fn did_save(&self, params: DidSaveTextDocumentParams) {
		// On save, rescan the entire project to pick up cross-file changes.
		{
			let mut state = self.state.write().await;
			state.rescan_project();
		}
		self.publish_diagnostics_for(&params.text_document.uri)
			.await;
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
			Ok(Some(DocumentSymbolResponse::Flat(symbols)))
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
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

/// Compute diagnostics for a single document.
fn compute_diagnostics(state: &WorkspaceState, uri: &Url) -> Vec<Diagnostic> {
	let Some(doc) = state.documents.get(uri) else {
		return Vec::new();
	};

	let mut diagnostics = Vec::new();

	for block in &doc.blocks {
		match block.r#type {
			BlockType::Consumer => {
				let consumer_content = extract_content_between_tags(&doc.content, block);

				if let Some(provider) = state.providers.get(&block.name) {
					// Check if the consumer is stale.
					let rendered = render_template(&provider.content, &state.data)
						.unwrap_or_else(|_| provider.content.clone());
					let expected = apply_transformers(&rendered, &block.transformers);

					if consumer_content != expected {
						diagnostics.push(Diagnostic {
							range: to_lsp_range(&block.opening),
							severity: Some(DiagnosticSeverity::WARNING),
							source: Some("mdt".to_string()),
							message: format!("Consumer block `{}` is out of date", block.name),
							data: Some(serde_json::json!({
								"kind": "stale",
								"block_name": block.name,
								"expected_content": expected,
							})),
							..Default::default()
						});
					}
				} else {
					// Missing provider
					diagnostics.push(Diagnostic {
						range: to_lsp_range(&block.opening),
						severity: Some(DiagnosticSeverity::WARNING),
						source: Some("mdt".to_string()),
						message: format!("No provider found for consumer block `{}`", block.name),
						..Default::default()
					});
				}
			}
			BlockType::Provider => {
				// Check if this provider is in a template file.
				let is_template = uri.path().ends_with(".t.md");
				if !is_template {
					diagnostics.push(Diagnostic {
						range: to_lsp_range(&block.opening),
						severity: Some(DiagnosticSeverity::INFORMATION),
						source: Some("mdt".to_string()),
						message: format!(
							"Provider block `{}` is only recognized in *.t.md template files",
							block.name
						),
						..Default::default()
					});
				}
			}
		}
	}

	diagnostics
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
fn compute_hover(state: &WorkspaceState, uri: &Url, position: Position) -> Option<Hover> {
	let doc = state.documents.get(uri)?;
	let block = find_block_at_position(&doc.blocks, position)?;

	let contents = match block.r#type {
		BlockType::Consumer => {
			let mut parts = Vec::new();
			parts.push(format!("**Consumer block:** `{}`", block.name));

			if let Some(provider) = state.providers.get(&block.name) {
				let rendered = render_template(&provider.content, &state.data)
					.unwrap_or_else(|_| provider.content.clone());
				let expected = apply_transformers(&rendered, &block.transformers);

				parts.push(format!(
					"\n**Provider source:** `{}`",
					provider.file.display()
				));

				if !block.transformers.is_empty() {
					let names: Vec<String> = block
						.transformers
						.iter()
						.map(|t| transformer_type_name(t.r#type))
						.collect();
					parts.push(format!("\n**Transformers:** {}", names.join(" | ")));
				}

				parts.push(format!("\n---\n\n```\n{}\n```", expected.trim()));
			} else {
				parts.push("\n*No matching provider found*".to_string());
			}

			parts.join("")
		}
		BlockType::Provider => {
			let mut parts = Vec::new();
			parts.push(format!("**Provider block:** `{}`", block.name));

			let content = extract_content_between_tags(&doc.content, block);
			let consumer_count = state
				.consumers
				.iter()
				.filter(|c| c.block.name == block.name)
				.count();

			parts.push(format!("\n**Referenced by:** {consumer_count} consumer(s)"));

			// List consumer locations
			let consumer_files: Vec<String> = state
				.consumers
				.iter()
				.filter(|c| c.block.name == block.name)
				.map(|c| format!("`{}`", c.file.display()))
				.collect();

			if !consumer_files.is_empty() {
				parts.push(format!("\n**Consumers in:** {}", consumer_files.join(", ")));
			}

			parts.push(format!("\n---\n\n```\n{}\n```", content.trim()));

			parts.join("")
		}
	};

	Some(Hover {
		contents: HoverContents::Markup(MarkupContent {
			kind: MarkupKind::Markdown,
			value: contents,
		}),
		range: Some(to_lsp_range(&block.opening)),
	})
}

/// Get the display name for a transformer type.
fn transformer_type_name(t: TransformerType) -> String {
	match t {
		TransformerType::Trim => "trim".to_string(),
		TransformerType::TrimStart => "trimStart".to_string(),
		TransformerType::TrimEnd => "trimEnd".to_string(),
		TransformerType::Indent => "indent".to_string(),
		TransformerType::Prefix => "prefix".to_string(),
		TransformerType::Wrap => "wrap".to_string(),
		TransformerType::CodeBlock => "codeBlock".to_string(),
		TransformerType::Code => "code".to_string(),
		TransformerType::Replace => "replace".to_string(),
	}
}

// ---------------------------------------------------------------------------
// Completions
// ---------------------------------------------------------------------------

/// Compute completion items at a position.
fn compute_completions(
	state: &WorkspaceState,
	uri: &Url,
	position: Position,
) -> Vec<CompletionItem> {
	let Some(doc) = state.documents.get(uri) else {
		return Vec::new();
	};

	// Check if we're inside an HTML comment context by looking at text before
	// cursor.
	let line_idx = position.line as usize;
	let col = position.character as usize;

	let lines: Vec<&str> = doc.content.lines().collect();
	let Some(line) = lines.get(line_idx) else {
		return Vec::new();
	};

	let before_cursor = if col <= line.len() {
		&line[..col]
	} else {
		line
	};

	// Check if we're in a context where block name completion makes sense:
	// after `{=`, `{@`, or `{/`
	let in_tag_context = before_cursor.contains("{=")
		|| before_cursor.contains("{@")
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
fn compute_goto_definition(
	state: &WorkspaceState,
	uri: &Url,
	position: Position,
) -> Option<GotoDefinitionResponse> {
	let doc = state.documents.get(uri)?;
	let block = find_block_at_position(&doc.blocks, position)?;

	match block.r#type {
		BlockType::Consumer => {
			// Navigate to the provider definition.
			let provider = state.providers.get(&block.name)?;
			let target_uri = Url::from_file_path(&provider.file).ok()?;
			let target_range = to_lsp_range(&provider.block.opening);

			Some(GotoDefinitionResponse::Scalar(Location {
				uri: target_uri,
				range: target_range,
			}))
		}
		BlockType::Provider => {
			// Navigate to all consumers of this provider.
			let locations: Vec<Location> = state
				.consumers
				.iter()
				.filter(|c| c.block.name == block.name)
				.filter_map(|c| {
					let consumer_uri = Url::from_file_path(&c.file).ok()?;
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
	}
}

// ---------------------------------------------------------------------------
// Document Symbols
// ---------------------------------------------------------------------------

/// Compute document symbols for the outline view.
#[allow(deprecated)]
fn compute_document_symbols(state: &WorkspaceState, uri: &Url) -> Vec<SymbolInformation> {
	let Some(doc) = state.documents.get(uri) else {
		return Vec::new();
	};

	doc.blocks
		.iter()
		.map(|block| {
			let kind = match block.r#type {
				BlockType::Provider => SymbolKind::CLASS,
				BlockType::Consumer => SymbolKind::VARIABLE,
			};
			let prefix = match block.r#type {
				BlockType::Provider => "@",
				BlockType::Consumer => "=",
			};

			SymbolInformation {
				name: format!("{prefix}{}", block.name),
				kind,
				tags: None,
				deprecated: None,
				location: Location {
					uri: uri.clone(),
					range: Range {
						start: to_lsp_position(&block.opening.start),
						end: to_lsp_position(&block.closing.end),
					},
				},
				container_name: None,
			}
		})
		.collect()
}

// ---------------------------------------------------------------------------
// Code Actions
// ---------------------------------------------------------------------------

/// Compute code actions for a range. Offers "Update block" for stale
/// consumers.
fn compute_code_actions(
	state: &WorkspaceState,
	uri: &Url,
	range: Range,
) -> Vec<CodeActionOrCommand> {
	let Some(doc) = state.documents.get(uri) else {
		return Vec::new();
	};

	let mut actions = Vec::new();

	for block in &doc.blocks {
		if block.r#type != BlockType::Consumer {
			continue;
		}

		let opening_range = to_lsp_range(&block.opening);
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

		let Some(provider) = state.providers.get(&block.name) else {
			continue;
		};

		let rendered = render_template(&provider.content, &state.data)
			.unwrap_or_else(|_| provider.content.clone());
		let expected = apply_transformers(&rendered, &block.transformers);
		let current = extract_content_between_tags(&doc.content, block);

		if current == expected {
			continue;
		}

		// Build a text edit that replaces the content between the tags.
		let content_start = to_lsp_position(&block.opening.end);
		let content_end = to_lsp_position(&block.closing.start);

		let edit = TextEdit {
			range: Range {
				start: content_start,
				end: content_end,
			},
			new_text: expected,
		};

		let mut changes = HashMap::new();
		changes.insert(uri.clone(), vec![edit]);

		actions.push(CodeActionOrCommand::CodeAction(CodeAction {
			title: format!("Update block `{}`", block.name),
			kind: Some(CodeActionKind::QUICKFIX),
			diagnostics: Some(vec![Diagnostic {
				range: opening_range,
				severity: Some(DiagnosticSeverity::WARNING),
				source: Some("mdt".to_string()),
				message: format!("Consumer block `{}` is out of date", block.name),
				..Default::default()
			}]),
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

/// Start the LSP server on stdin/stdout. This is used by both the standalone
/// `mdt-lsp` binary and the `mdt lsp` CLI subcommand.
pub async fn run_server() {
	let stdin = tokio::io::stdin();
	let stdout = tokio::io::stdout();

	let (service, socket) = tower_lsp::LspService::new(MdtLanguageServer::new);
	tower_lsp::Server::new(stdin, stdout, socket)
		.serve(service)
		.await;
}

#[cfg(test)]
mod __tests;

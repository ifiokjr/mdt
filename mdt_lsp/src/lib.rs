use std::collections::HashMap;
use std::path::PathBuf;

use mdt_core::Block;
use mdt_core::BlockType;
use mdt_core::ParseDiagnostic;
use mdt_core::apply_transformers;
use mdt_core::parse_source_with_diagnostics;
use mdt_core::parse_with_diagnostics;
use mdt_core::project::ConsumerEntry;
use mdt_core::project::ProviderEntry;
use mdt_core::project::extract_content_between_tags;
use mdt_core::project::scan_project_with_config;
use mdt_core::render_template;
use serde_json::Value;
use tokio::sync::RwLock;
use tower_lsp_server::Client;
use tower_lsp_server::LanguageServer;
use tower_lsp_server::jsonrpc::Result as LspResult;
use tower_lsp_server::ls_types::*;

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
#[derive(Debug, Default)]
struct WorkspaceState {
	/// The workspace root path.
	root: Option<PathBuf>,
	/// Open documents keyed by URI.
	documents: HashMap<Uri, DocumentState>,
	/// Cached providers from the last project scan.
	providers: HashMap<String, ProviderEntry>,
	/// Cached consumers from the last project scan.
	consumers: Vec<ConsumerEntry>,
	/// Template data from mdt.toml config.
	data: HashMap<String, Value>,
}

impl WorkspaceState {
	/// Rescan the project from disk. Called on initialize, save, and
	/// workspace changes.
	fn rescan_project(&mut self) {
		let Some(root) = &self.root else {
			return;
		};

		match scan_project_with_config(root) {
			Ok(ctx) => {
				self.providers = ctx.project.providers;
				self.consumers = ctx.project.consumers;
				self.data = ctx.data;
			}
			Err(e) => {
				eprintln!("mdt-lsp: failed to scan project: {e}");
			}
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

		let is_template = uri.path().as_str().ends_with(".t.md");

		for block in &doc.blocks {
			let block_content = extract_content_between_tags(&doc.content, block);

			if block.r#type == BlockType::Provider && is_template {
				self.providers.insert(
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
		self.consumers.retain(|c| c.file != file_path);
		for block in &doc.blocks {
			if block.r#type == BlockType::Consumer {
				let block_content = extract_content_between_tags(&doc.content, block);
				self.consumers.push(ConsumerEntry {
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
		let (blocks, parse_diagnostics) = parse_document_content(uri, &content);
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
fn parse_document_content(uri: &Uri, content: &str) -> (Vec<Block>, Vec<ParseDiagnostic>) {
	let is_markdown = uri
		.path()
		.as_str()
		.rsplit('.')
		.next()
		.is_some_and(|ext| matches!(ext, "md" | "mdx" | "markdown"));

	let result = if is_markdown {
		parse_with_diagnostics(content)
	} else {
		parse_source_with_diagnostics(content, &mdt_core::CodeBlockFilter::default())
	};

	result.unwrap_or_default()
}

/// Compute the Levenshtein edit distance between two strings.
/// Used for suggesting similar block names when a provider is missing.
fn levenshtein_distance(a: &str, b: &str) -> usize {
	let a_len = a.len();
	let b_len = b.len();

	if a_len == 0 {
		return b_len;
	}
	if b_len == 0 {
		return a_len;
	}

	let mut prev_row: Vec<usize> = (0..=b_len).collect();
	let mut curr_row = vec![0; b_len + 1];

	for (i, a_char) in a.chars().enumerate() {
		curr_row[0] = i + 1;
		for (j, b_char) in b.chars().enumerate() {
			let cost = usize::from(a_char != b_char);
			curr_row[j + 1] = (prev_row[j + 1] + 1)
				.min(curr_row[j] + 1)
				.min(prev_row[j] + cost);
		}
		std::mem::swap(&mut prev_row, &mut curr_row);
	}

	prev_row[b_len]
}

/// Find the most similar provider names for a given consumer name.
/// Returns up to 3 suggestions with a maximum edit distance threshold.
fn suggest_similar_names<'a>(
	name: &str,
	providers: &'a HashMap<String, ProviderEntry>,
) -> Vec<&'a str> {
	// Use a threshold based on name length: allow roughly 40% character changes.
	let max_distance = (name.len() / 2).max(2);
	let mut candidates: Vec<(&str, usize)> = providers
		.keys()
		.map(|p| (p.as_str(), levenshtein_distance(name, p)))
		.filter(|(_, d)| *d <= max_distance && *d > 0)
		.collect();
	candidates.sort_by_key(|(_, d)| *d);
	candidates.truncate(3);
	candidates.into_iter().map(|(name, _)| name).collect()
}

/// Merge provider parameter names with consumer argument values into a data
/// context for template rendering.
fn merge_block_args(
	base_data: &HashMap<String, Value>,
	provider: &Block,
	consumer: &Block,
) -> HashMap<String, Value> {
	if provider.arguments.is_empty() {
		return base_data.clone();
	}
	let mut data = base_data.clone();
	for (name, value) in provider.arguments.iter().zip(consumer.arguments.iter()) {
		data.insert(name.clone(), Value::String(value.clone()));
	}
	data
}

/// Convert an mdt `Point` (1-indexed line, 1-indexed column) to an LSP
/// `Position` (0-indexed).
fn to_lsp_position(point: &mdt_core::Point) -> Position {
	Position {
		line: point.line.saturating_sub(1) as u32,
		character: point.column.saturating_sub(1) as u32,
	}
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
			// LSP character offsets are in UTF-16 code units, so we need to
			// walk the line converting from UTF-16 units to byte indices.
			let mut utf16_offset = 0u32;
			for (byte_idx, c) in line.char_indices() {
				if utf16_offset == position.character {
					return Some(offset + byte_idx);
				}
				utf16_offset += c.len_utf16() as u32;
			}
			// Position at end of line (past last character).
			if utf16_offset == position.character {
				return Some(offset + line.len());
			}
			return None;
		}
		offset += line.len() + 1; // +1 for '\n'
	}
	None
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

		{
			let mut state = self.state.write().await;
			state.root = root;
			state.rescan_project();
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
					content.replace_range(start..end, &change.text);
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

		{
			let mut state = self.state.write().await;
			if is_config {
				// Config changed — full rescan needed for data and exclude changes.
				state.rescan_project();
			} else {
				// Incrementally update this document's providers/consumers.
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
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

/// Compute diagnostics for a single document. This includes:
/// - Stale consumer blocks (content doesn't match provider)
/// - Missing providers (consumer references a non-existent provider)
/// - Name suggestions for missing providers (Levenshtein distance)
/// - Provider blocks in non-template files
/// - Unused provider blocks (no consumers reference them)
/// - Unclosed blocks (opening tag without matching close)
/// - Unknown transformer names
/// - Invalid transformer arguments
fn compute_diagnostics(state: &WorkspaceState, uri: &Uri) -> Vec<Diagnostic> {
	let Some(doc) = state.documents.get(uri) else {
		return Vec::new();
	};

	let mut diagnostics = Vec::new();
	let is_template = uri.path().as_str().ends_with(".t.md");

	// Surface parse diagnostics (unclosed blocks, unknown transformers).
	for parse_diag in &doc.parse_diagnostics {
		match parse_diag {
			ParseDiagnostic::UnclosedBlock { name, line, column } => {
				let position = Position {
					line: line.saturating_sub(1) as u32,
					character: column.saturating_sub(1) as u32,
				};
				diagnostics.push(Diagnostic {
					range: Range {
						start: position,
						end: position,
					},
					severity: Some(DiagnosticSeverity::ERROR),
					source: Some("mdt".to_string()),
					message: format!("Missing closing tag for block `{name}`"),
					..Default::default()
				});
			}
			ParseDiagnostic::UnknownTransformer { name, line, column } => {
				let position = Position {
					line: line.saturating_sub(1) as u32,
					character: column.saturating_sub(1) as u32,
				};
				diagnostics.push(Diagnostic {
					range: Range {
						start: position,
						end: position,
					},
					severity: Some(DiagnosticSeverity::ERROR),
					source: Some("mdt".to_string()),
					message: format!("Unknown transformer `{name}`"),
					..Default::default()
				});
			}
			ParseDiagnostic::InvalidTransformerArgs {
				name,
				expected,
				got,
				line,
				column,
			} => {
				let position = Position {
					line: line.saturating_sub(1) as u32,
					character: column.saturating_sub(1) as u32,
				};
				diagnostics.push(Diagnostic {
					range: Range {
						start: position,
						end: position,
					},
					severity: Some(DiagnosticSeverity::ERROR),
					source: Some("mdt".to_string()),
					message: format!(
						"Transformer `{name}` expects {expected} argument(s), got {got}"
					),
					..Default::default()
				});
			}
		}
	}

	for block in &doc.blocks {
		match block.r#type {
			BlockType::Consumer => {
				let consumer_content = extract_content_between_tags(&doc.content, block);

				if let Some(provider) = state.providers.get(&block.name) {
					// Check if the consumer is stale.
					let render_data = merge_block_args(&state.data, &provider.block, block);
					let rendered = render_template(&provider.content, &render_data)
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
					// Missing provider — suggest similar names.
					let suggestions = suggest_similar_names(&block.name, &state.providers);
					let message = if suggestions.is_empty() {
						format!("No provider found for consumer block `{}`", block.name)
					} else {
						format!(
							"No provider found for consumer block `{}`. Did you mean: {}?",
							block.name,
							suggestions
								.iter()
								.map(|s| format!("`{s}`"))
								.collect::<Vec<_>>()
								.join(", ")
						)
					};

					diagnostics.push(Diagnostic {
						range: to_lsp_range(&block.opening),
						severity: Some(DiagnosticSeverity::WARNING),
						source: Some("mdt".to_string()),
						message,
						..Default::default()
					});
				}
			}
			BlockType::Provider => {
				if is_template {
					// Check for unused providers (no consumers reference this
					// block).
					let has_consumers = state.consumers.iter().any(|c| c.block.name == block.name);
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
fn compute_hover(state: &WorkspaceState, uri: &Uri, position: Position) -> Option<Hover> {
	let doc = state.documents.get(uri)?;
	let block = find_block_at_position(&doc.blocks, position)?;

	let contents = match block.r#type {
		BlockType::Consumer => {
			let mut parts = Vec::new();
			parts.push(format!("**Consumer block:** `{}`", block.name));

			if let Some(provider) = state.providers.get(&block.name) {
				let render_data = merge_block_args(&state.data, &provider.block, block);
				let rendered = render_template(&provider.content, &render_data)
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
						.map(|t| t.r#type.to_string())
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
	uri: &Uri,
	position: Position,
) -> Option<GotoDefinitionResponse> {
	let doc = state.documents.get(uri)?;
	let block = find_block_at_position(&doc.blocks, position)?;

	match block.r#type {
		BlockType::Consumer => {
			// Navigate to the provider definition.
			let provider = state.providers.get(&block.name)?;
			let target_uri = Uri::from_file_path(&provider.file)?;
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
					let consumer_uri = Uri::from_file_path(&c.file)?;
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
				BlockType::Consumer => SymbolKind::VARIABLE,
			};
			let prefix = match block.r#type {
				BlockType::Provider => "@",
				BlockType::Consumer => "=",
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

/// Compute code actions for a range. Offers "Update block" for stale
/// consumers.
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

		let render_data = merge_block_args(&state.data, &provider.block, block);
		let rendered = render_template(&provider.content, &render_data)
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

	let (service, socket) = tower_lsp_server::LspService::new(MdtLanguageServer::new);
	tower_lsp_server::Server::new(stdin, stdout, socket)
		.serve(service)
		.await;
}

#[cfg(test)]
mod __tests;

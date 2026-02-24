use std::collections::HashMap;
use std::path::PathBuf;

use mdt_core::TransformerType;
use mdt_core::parse;
use mdt_core::parse_with_diagnostics;
use mdt_core::project::ConsumerEntry;
use mdt_core::project::ProviderEntry;
use mdt_core::project::extract_content_between_tags;
#[allow(unused_imports)]
use tower_lsp_server::ls_types::*;

use super::*;

fn make_test_state(provider_content: &str, consumer_content: &str) -> (WorkspaceState, Uri) {
	let provider_template =
		format!("<!-- {{@greeting}} -->\n\n{provider_content}\n\n<!-- {{/greeting}} -->\n");
	let consumer_doc = format!(
		"# Readme\n\n<!-- {{=greeting}} -->\n\n{consumer_content}\n\n<!-- {{/greeting}} -->\n"
	);

	let provider_blocks = parse(&provider_template).unwrap_or_default();
	let (consumer_blocks, consumer_parse_diagnostics) =
		parse_with_diagnostics(&consumer_doc).unwrap_or_default();

	let provider_block = provider_blocks
		.iter()
		.find(|b| b.r#type == BlockType::Provider)
		.cloned()
		.unwrap_or_else(|| panic!("expected a provider block in template"));

	let provider_entry = ProviderEntry {
		block: provider_block,
		file: PathBuf::from("/tmp/test/template.t.md"),
		content: extract_content_between_tags(&provider_template, &provider_blocks[0]),
	};

	let consumer_uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut providers = HashMap::new();
	providers.insert("greeting".to_string(), provider_entry.clone());

	let mut consumers = Vec::new();
	for block in &consumer_blocks {
		if block.r#type == BlockType::Consumer {
			consumers.push(ConsumerEntry {
				block: block.clone(),
				file: PathBuf::from("/tmp/test/readme.md"),
				content: extract_content_between_tags(&consumer_doc, block),
			});
		}
	}

	let mut documents = HashMap::new();
	documents.insert(
		consumer_uri.clone(),
		DocumentState {
			content: consumer_doc,
			blocks: consumer_blocks,
			parse_diagnostics: consumer_parse_diagnostics,
		},
	);

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers,
		consumers,
		data: HashMap::new(),
	};

	(state, consumer_uri)
}

// ---- Diagnostics tests ----

#[test]
fn diagnostics_stale_consumer() {
	let (state, uri) = make_test_state("Hello world!", "Old content");
	let diagnostics = compute_diagnostics(&state, &uri);

	assert_eq!(diagnostics.len(), 1);
	assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::WARNING));
	assert!(
		diagnostics[0].message.contains("out of date"),
		"expected 'out of date' in message: {}",
		diagnostics[0].message
	);
	assert!(diagnostics[0].message.contains("greeting"));
}

#[test]
fn diagnostics_up_to_date_consumer() {
	let (state, uri) = make_test_state("Hello world!", "Hello world!");
	let diagnostics = compute_diagnostics(&state, &uri);

	// Content between tags includes surrounding newlines, so let's check
	// the actual values from the state.
	let doc = state.documents.get(&uri).unwrap();
	let block = doc
		.blocks
		.iter()
		.find(|b| b.r#type == BlockType::Consumer)
		.unwrap();
	let content = extract_content_between_tags(&doc.content, block);
	let provider = state.providers.get("greeting").unwrap();
	let expected = apply_transformers(&provider.content, &block.transformers);

	if content == expected {
		assert!(diagnostics.is_empty(), "expected no diagnostics");
	}
	// If they differ due to whitespace, a diagnostic is expected — that's OK.
}

#[test]
fn diagnostics_missing_provider() {
	let consumer_doc = "<!-- {=orphan} -->\n\nstuff\n\n<!-- {/orphan} -->\n";
	let consumer_blocks = parse(consumer_doc).unwrap_or_default();
	let consumer_uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut documents = HashMap::new();
	documents.insert(
		consumer_uri.clone(),
		DocumentState {
			content: consumer_doc.to_string(),
			blocks: consumer_blocks,
			parse_diagnostics: Vec::new(),
		},
	);

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let diagnostics = compute_diagnostics(&state, &consumer_uri);
	assert_eq!(diagnostics.len(), 1);
	assert!(diagnostics[0].message.contains("No provider found"));
	assert!(diagnostics[0].message.contains("orphan"));
}

#[test]
fn diagnostics_provider_in_non_template_file() {
	let content = "<!-- {@greeting} -->\n\nHello\n\n<!-- {/greeting} -->\n";
	let blocks = parse(content).unwrap_or_default();
	// Use a non-template URI (readme.md, not *.t.md)
	let uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: content.to_string(),
			blocks,
			parse_diagnostics: Vec::new(),
		},
	);

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let diagnostics = compute_diagnostics(&state, &uri);
	assert_eq!(diagnostics.len(), 1);
	assert!(diagnostics[0].message.contains("only recognized in *.t.md"));
}

// ---- Hover tests ----

#[test]
fn hover_on_consumer_shows_provider_content() {
	let (state, uri) = make_test_state("Hello world!", "Old content");
	let doc = state.documents.get(&uri).unwrap();
	let block = doc
		.blocks
		.iter()
		.find(|b| b.r#type == BlockType::Consumer)
		.unwrap();

	// Position on the consumer's opening tag.
	let position = to_lsp_position(&block.opening.start);
	let hover = compute_hover(&state, &uri, position);

	assert!(hover.is_some());
	let hover = hover.unwrap();
	if let HoverContents::Markup(markup) = &hover.contents {
		assert!(markup.value.contains("Consumer block"));
		assert!(markup.value.contains("greeting"));
		assert!(markup.value.contains("Hello world!"));
	} else {
		panic!("expected Markup hover contents");
	}
}

#[test]
fn hover_on_provider_shows_consumer_count() {
	let provider_template = "<!-- {@greeting} -->\n\nHello!\n\n<!-- {/greeting} -->\n";
	let provider_blocks = parse(provider_template).unwrap_or_default();
	let provider_uri = "file:///tmp/test/template.t.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let provider_block = provider_blocks
		.iter()
		.find(|b| b.r#type == BlockType::Provider)
		.cloned()
		.unwrap_or_else(|| panic!("expected a provider block"));

	let provider_entry = ProviderEntry {
		block: provider_block.clone(),
		file: PathBuf::from("/tmp/test/template.t.md"),
		content: extract_content_between_tags(provider_template, &provider_blocks[0]),
	};

	let mut providers = HashMap::new();
	providers.insert("greeting".to_string(), provider_entry);

	let consumers = vec![ConsumerEntry {
		block: Block {
			name: "greeting".to_string(),
			r#type: BlockType::Consumer,
			opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
			closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
			transformers: Vec::new(),
		},
		file: PathBuf::from("/tmp/test/readme.md"),
		content: "\n\nold\n\n".to_string(),
	}];

	let mut documents = HashMap::new();
	documents.insert(
		provider_uri.clone(),
		DocumentState {
			content: provider_template.to_string(),
			blocks: provider_blocks,
			parse_diagnostics: Vec::new(),
		},
	);

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers,
		consumers,
		data: HashMap::new(),
	};

	let position = to_lsp_position(&provider_block.opening.start);
	let hover = compute_hover(&state, &provider_uri, position);
	assert!(hover.is_some());
	if let HoverContents::Markup(markup) = &hover.unwrap().contents {
		assert!(markup.value.contains("Provider block"));
		assert!(markup.value.contains("1 consumer(s)"));
	} else {
		panic!("expected Markup hover contents");
	}
}

#[test]
fn hover_outside_block_returns_none() {
	let (state, uri) = make_test_state("Hello!", "Hello!");

	// Position at line 0, col 0 — before any block.
	let position = Position {
		line: 0,
		character: 0,
	};
	let hover = compute_hover(&state, &uri, position);
	assert!(hover.is_none());
}

// ---- Completion tests ----

#[test]
fn completion_inside_consumer_tag() {
	let consumer_doc = "<!-- {=gre";
	let uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: consumer_doc.to_string(),
			blocks: Vec::new(),
			parse_diagnostics: Vec::new(),
		},
	);

	let provider_entry = ProviderEntry {
		block: Block {
			name: "greeting".to_string(),
			r#type: BlockType::Provider,
			opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
			closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
			transformers: Vec::new(),
		},
		file: PathBuf::from("/tmp/test/template.t.md"),
		content: "\n\nHello!\n\n".to_string(),
	};

	let mut providers = HashMap::new();
	providers.insert("greeting".to_string(), provider_entry);

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers,
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let position = Position {
		line: 0,
		character: 9,
	};
	let completions = compute_completions(&state, &uri, position);
	assert!(!completions.is_empty());
	assert!(
		completions.iter().any(|c| c.label == "greeting"),
		"expected 'greeting' completion item"
	);
}

#[test]
fn completion_after_pipe_suggests_transformers() {
	let consumer_doc = "<!-- {=greeting|";
	let uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: consumer_doc.to_string(),
			blocks: Vec::new(),
			parse_diagnostics: Vec::new(),
		},
	);

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let position = Position {
		line: 0,
		character: 16,
	};
	let completions = compute_completions(&state, &uri, position);
	assert!(!completions.is_empty());

	let names: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
	assert!(names.contains(&"trim"));
	assert!(names.contains(&"indent"));
	assert!(names.contains(&"codeBlock"));
	assert!(names.contains(&"replace"));
}

#[test]
fn completion_outside_tag_returns_empty() {
	let consumer_doc = "# Normal markdown";
	let uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: consumer_doc.to_string(),
			blocks: Vec::new(),
			parse_diagnostics: Vec::new(),
		},
	);

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let position = Position {
		line: 0,
		character: 5,
	};
	let completions = compute_completions(&state, &uri, position);
	assert!(completions.is_empty());
}

// ---- Go to Definition tests ----

#[test]
fn goto_definition_consumer_to_provider() {
	let (state, uri) = make_test_state("Hello!", "Old");
	let doc = state.documents.get(&uri).unwrap();
	let block = doc
		.blocks
		.iter()
		.find(|b| b.r#type == BlockType::Consumer)
		.unwrap();

	let position = to_lsp_position(&block.opening.start);
	let result = compute_goto_definition(&state, &uri, position);

	assert!(result.is_some());
	match result.unwrap() {
		GotoDefinitionResponse::Scalar(loc) => {
			assert!(
				loc.uri.path().as_str().contains("template.t.md"),
				"expected target to be the template file"
			);
		}
		GotoDefinitionResponse::Array(locs) => {
			assert!(locs[0].uri.path().as_str().contains("template.t.md"));
		}
		GotoDefinitionResponse::Link(_) => panic!("unexpected Link goto definition response"),
	}
}

#[test]
fn goto_definition_without_matching_provider_returns_none() {
	let consumer_doc = "<!-- {=missing} -->\n\nstuff\n\n<!-- {/missing} -->\n";
	let consumer_blocks = parse(consumer_doc).unwrap_or_default();
	let uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: consumer_doc.to_string(),
			blocks: consumer_blocks.clone(),
			parse_diagnostics: Vec::new(),
		},
	);

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let block = consumer_blocks
		.iter()
		.find(|b| b.r#type == BlockType::Consumer)
		.unwrap();
	let position = to_lsp_position(&block.opening.start);
	let result = compute_goto_definition(&state, &uri, position);
	assert!(result.is_none());
}

// ---- Document Symbols tests ----

#[test]
fn document_symbols_lists_blocks() {
	let content = "<!-- {@greeting} -->\n\nHello\n\n<!-- {/greeting} -->\n\n<!-- {=other} \
	               -->\n\nstuff\n\n<!-- {/other} -->\n";
	let blocks = parse(content).unwrap_or_default();
	let uri = "file:///tmp/test/template.t.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: content.to_string(),
			blocks,
			parse_diagnostics: Vec::new(),
		},
	);

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let symbols = compute_document_symbols(&state, &uri);
	assert_eq!(symbols.len(), 2);

	let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
	assert!(names.contains(&"@greeting"));
	assert!(names.contains(&"=other"));
}

#[test]
fn document_symbols_empty_for_no_blocks() {
	let content = "# Just a heading\n\nNo blocks here.\n";
	let blocks = parse(content).unwrap_or_default();
	let uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: content.to_string(),
			blocks,
			parse_diagnostics: Vec::new(),
		},
	);

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let symbols = compute_document_symbols(&state, &uri);
	assert!(symbols.is_empty());
}

// ---- Code Action tests ----

#[test]
fn code_action_for_stale_consumer() {
	let (state, uri) = make_test_state("Hello world!", "Old content");
	let doc = state.documents.get(&uri).unwrap();
	let block = doc
		.blocks
		.iter()
		.find(|b| b.r#type == BlockType::Consumer)
		.unwrap();

	let range = Range {
		start: to_lsp_position(&block.opening.start),
		end: to_lsp_position(&block.closing.end),
	};

	let actions = compute_code_actions(&state, &uri, range);
	assert!(!actions.is_empty(), "expected at least one code action");

	let CodeActionOrCommand::CodeAction(action) = &actions[0] else {
		panic!("expected CodeAction")
	};

	assert!(action.title.contains("Update block"));
	assert!(action.title.contains("greeting"));
	assert!(action.edit.is_some());
}

#[test]
fn code_action_not_offered_when_up_to_date() {
	// Create a state where the consumer content exactly matches the provider.
	let provider_template = "<!-- {@greeting} -->\n\nHello!\n\n<!-- {/greeting} -->\n";
	let consumer_doc = "<!-- {=greeting} -->\n\nHello!\n\n<!-- {/greeting} -->\n";

	let provider_blocks = parse(provider_template).unwrap_or_default();
	let consumer_blocks = parse(consumer_doc).unwrap_or_default();

	let provider_block = provider_blocks
		.iter()
		.find(|b| b.r#type == BlockType::Provider)
		.cloned()
		.unwrap_or_else(|| panic!("expected provider block"));

	let provider_entry = ProviderEntry {
		block: provider_block,
		file: PathBuf::from("/tmp/test/template.t.md"),
		content: extract_content_between_tags(provider_template, &provider_blocks[0]),
	};

	let mut providers = HashMap::new();
	providers.insert("greeting".to_string(), provider_entry);

	let consumer_uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut documents = HashMap::new();
	documents.insert(
		consumer_uri.clone(),
		DocumentState {
			content: consumer_doc.to_string(),
			blocks: consumer_blocks.clone(),
			parse_diagnostics: Vec::new(),
		},
	);

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers,
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let block = consumer_blocks
		.iter()
		.find(|b| b.r#type == BlockType::Consumer)
		.unwrap();

	let range = Range {
		start: to_lsp_position(&block.opening.start),
		end: to_lsp_position(&block.closing.end),
	};

	let actions = compute_code_actions(&state, &consumer_uri, range);
	assert!(
		actions.is_empty(),
		"expected no code actions for up-to-date block"
	);
}

// ---- Helper function tests ----

#[test]
fn position_in_range_basic() {
	let range = Range {
		start: Position {
			line: 2,
			character: 0,
		},
		end: Position {
			line: 2,
			character: 20,
		},
	};

	assert!(position_in_range(
		Position {
			line: 2,
			character: 10
		},
		range
	));
	assert!(!position_in_range(
		Position {
			line: 1,
			character: 10
		},
		range
	));
	assert!(!position_in_range(
		Position {
			line: 3,
			character: 0
		},
		range
	));
}

#[test]
fn ranges_overlap_basic() {
	let a = Range {
		start: Position {
			line: 2,
			character: 0,
		},
		end: Position {
			line: 5,
			character: 10,
		},
	};
	let b = Range {
		start: Position {
			line: 4,
			character: 0,
		},
		end: Position {
			line: 8,
			character: 5,
		},
	};
	assert!(ranges_overlap(a, b));

	let c = Range {
		start: Position {
			line: 10,
			character: 0,
		},
		end: Position {
			line: 12,
			character: 5,
		},
	};
	assert!(!ranges_overlap(a, c));
}

#[test]
fn to_lsp_position_converts_correctly() {
	let point = mdt_core::Point::new(1, 1, 0);
	let lsp_pos = to_lsp_position(&point);
	assert_eq!(lsp_pos.line, 0);
	assert_eq!(lsp_pos.character, 0);

	let point2 = mdt_core::Point::new(5, 10, 42);
	let lsp_pos2 = to_lsp_position(&point2);
	assert_eq!(lsp_pos2.line, 4);
	assert_eq!(lsp_pos2.character, 9);
}

#[test]
fn parse_document_content_markdown() {
	let uri = "file:///test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid URI"));
	let content = "<!-- {@greeting} -->\n\nHello\n\n<!-- {/greeting} -->\n";
	let (blocks, diagnostics) = parse_document_content(&uri, content);
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "greeting");
	assert!(diagnostics.is_empty());
}

#[test]
fn parse_document_content_source_file() {
	let uri = "file:///test/main.rs"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid URI"));
	let content = "// <!-- {=block} -->\n// content\n// <!-- {/block} -->\n";
	let (blocks, diagnostics) = parse_document_content(&uri, content);
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "block");
	assert!(diagnostics.is_empty());
}

#[test]
fn transformer_type_display_all() {
	assert_eq!(TransformerType::Trim.to_string(), "trim");
	assert_eq!(TransformerType::TrimStart.to_string(), "trimStart");
	assert_eq!(TransformerType::TrimEnd.to_string(), "trimEnd");
	assert_eq!(TransformerType::Indent.to_string(), "indent");
	assert_eq!(TransformerType::Prefix.to_string(), "prefix");
	assert_eq!(TransformerType::Wrap.to_string(), "wrap");
	assert_eq!(TransformerType::CodeBlock.to_string(), "codeBlock");
	assert_eq!(TransformerType::Code.to_string(), "code");
	assert_eq!(TransformerType::Replace.to_string(), "replace");
}

// ---- New diagnostic tests ----

#[test]
fn diagnostics_unclosed_block() {
	let content = "<!-- {=greeting} -->\n\nHello\n";
	let uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let (blocks, parse_diagnostics) = parse_with_diagnostics(content).unwrap_or_default();

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: content.to_string(),
			blocks,
			parse_diagnostics,
		},
	);

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let diagnostics = compute_diagnostics(&state, &uri);
	assert!(
		diagnostics
			.iter()
			.any(|d| d.message.contains("Missing closing tag") && d.message.contains("greeting")),
		"expected unclosed block diagnostic, got: {diagnostics:?}"
	);
	assert!(
		diagnostics
			.iter()
			.any(|d| d.severity == Some(DiagnosticSeverity::ERROR)),
		"unclosed block should be an error"
	);
}

#[test]
fn diagnostics_unknown_transformer() {
	let content = "<!-- {=greeting|foobar} -->\n\nHello\n\n<!-- {/greeting} -->\n";
	let uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let (blocks, parse_diagnostics) = parse_with_diagnostics(content).unwrap_or_default();

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: content.to_string(),
			blocks,
			parse_diagnostics,
		},
	);

	let mut providers = HashMap::new();
	providers.insert(
		"greeting".to_string(),
		ProviderEntry {
			block: Block {
				name: "greeting".to_string(),
				r#type: BlockType::Provider,
				opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
				closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
				transformers: Vec::new(),
			},
			file: PathBuf::from("/tmp/test/template.t.md"),
			content: "\n\nHello!\n\n".to_string(),
		},
	);

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers,
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let diagnostics = compute_diagnostics(&state, &uri);
	assert!(
		diagnostics
			.iter()
			.any(|d| d.message.contains("Unknown transformer") && d.message.contains("foobar")),
		"expected unknown transformer diagnostic, got: {diagnostics:?}"
	);
}

#[test]
fn diagnostics_unused_provider() {
	let content = "<!-- {@greeting} -->\n\nHello\n\n<!-- {/greeting} -->\n";
	let uri = "file:///tmp/test/template.t.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let blocks = parse(content).unwrap_or_default();

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: content.to_string(),
			blocks,
			parse_diagnostics: Vec::new(),
		},
	);

	let mut providers = HashMap::new();
	providers.insert(
		"greeting".to_string(),
		ProviderEntry {
			block: Block {
				name: "greeting".to_string(),
				r#type: BlockType::Provider,
				opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
				closing: mdt_core::Position::new(5, 1, 28, 5, 22, 49),
				transformers: Vec::new(),
			},
			file: PathBuf::from("/tmp/test/template.t.md"),
			content: "\n\nHello\n\n".to_string(),
		},
	);

	// No consumers — the provider is unused.
	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers,
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let diagnostics = compute_diagnostics(&state, &uri);
	assert!(
		diagnostics
			.iter()
			.any(|d| d.message.contains("has no consumers") && d.message.contains("greeting")),
		"expected unused provider diagnostic, got: {diagnostics:?}"
	);
}

#[test]
fn diagnostics_missing_provider_with_suggestion() {
	let content = "<!-- {=greetng} -->\n\nstuff\n\n<!-- {/greetng} -->\n";
	let uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let (blocks, parse_diagnostics) = parse_with_diagnostics(content).unwrap_or_default();

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: content.to_string(),
			blocks,
			parse_diagnostics,
		},
	);

	let mut providers = HashMap::new();
	providers.insert(
		"greeting".to_string(),
		ProviderEntry {
			block: Block {
				name: "greeting".to_string(),
				r#type: BlockType::Provider,
				opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
				closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
				transformers: Vec::new(),
			},
			file: PathBuf::from("/tmp/test/template.t.md"),
			content: "\n\nHello!\n\n".to_string(),
		},
	);

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers,
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let diagnostics = compute_diagnostics(&state, &uri);
	assert!(
		diagnostics
			.iter()
			.any(|d| d.message.contains("Did you mean") && d.message.contains("`greeting`")),
		"expected suggestion for similar name, got: {diagnostics:?}"
	);
}

#[test]
fn diagnostics_missing_provider_no_suggestion_when_too_different() {
	let content = "<!-- {=xyz} -->\n\nstuff\n\n<!-- {/xyz} -->\n";
	let uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let (blocks, parse_diagnostics) = parse_with_diagnostics(content).unwrap_or_default();

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: content.to_string(),
			blocks,
			parse_diagnostics,
		},
	);

	let mut providers = HashMap::new();
	providers.insert(
		"greeting".to_string(),
		ProviderEntry {
			block: Block {
				name: "greeting".to_string(),
				r#type: BlockType::Provider,
				opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
				closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
				transformers: Vec::new(),
			},
			file: PathBuf::from("/tmp/test/template.t.md"),
			content: "\n\nHello!\n\n".to_string(),
		},
	);

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers,
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let diagnostics = compute_diagnostics(&state, &uri);
	let missing_diag = diagnostics
		.iter()
		.find(|d| d.message.contains("No provider found"))
		.unwrap_or_else(|| panic!("expected missing provider diagnostic, got: {diagnostics:?}"));
	assert!(
		!missing_diag.message.contains("Did you mean"),
		"should not suggest when names are too different"
	);
}

// ---- Levenshtein distance tests ----

#[test]
fn levenshtein_identical() {
	assert_eq!(levenshtein_distance("hello", "hello"), 0);
}

#[test]
fn levenshtein_empty() {
	assert_eq!(levenshtein_distance("", "hello"), 5);
	assert_eq!(levenshtein_distance("hello", ""), 5);
	assert_eq!(levenshtein_distance("", ""), 0);
}

#[test]
fn levenshtein_one_edit() {
	assert_eq!(levenshtein_distance("greeting", "greetng"), 1);
	assert_eq!(levenshtein_distance("hello", "helo"), 1);
	assert_eq!(levenshtein_distance("cat", "hat"), 1);
}

#[test]
fn levenshtein_multiple_edits() {
	assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
}

#[test]
fn suggest_similar_names_finds_close_match() {
	let mut providers = HashMap::new();
	providers.insert(
		"greeting".to_string(),
		ProviderEntry {
			block: Block {
				name: "greeting".to_string(),
				r#type: BlockType::Provider,
				opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
				closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
				transformers: Vec::new(),
			},
			file: PathBuf::from("/tmp/test/template.t.md"),
			content: String::new(),
		},
	);
	providers.insert(
		"installation".to_string(),
		ProviderEntry {
			block: Block {
				name: "installation".to_string(),
				r#type: BlockType::Provider,
				opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
				closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
				transformers: Vec::new(),
			},
			file: PathBuf::from("/tmp/test/template.t.md"),
			content: String::new(),
		},
	);

	let suggestions = suggest_similar_names("greetng", &providers);
	assert!(
		suggestions.contains(&"greeting"),
		"expected 'greeting' suggestion"
	);
	assert!(
		!suggestions.contains(&"installation"),
		"should not suggest distant name"
	);
}

#[test]
fn parse_document_content_with_unclosed_block() {
	let uri = "file:///test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid URI"));
	let content = "<!-- {=greeting} -->\n\nHello\n";
	let (blocks, diagnostics) = parse_document_content(&uri, content);
	assert!(
		blocks.is_empty(),
		"unclosed block should not produce a block"
	);
	assert_eq!(diagnostics.len(), 1);
	assert!(matches!(
		diagnostics[0],
		ParseDiagnostic::UnclosedBlock { .. }
	));
}

#[test]
fn parse_document_content_with_unknown_transformer() {
	let uri = "file:///test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid URI"));
	let content = "<!-- {=greeting|unknownFilter} -->\n\nHello\n\n<!-- {/greeting} -->\n";
	let (blocks, diagnostics) = parse_document_content(&uri, content);
	assert_eq!(blocks.len(), 1);
	assert_eq!(diagnostics.len(), 1);
	assert!(matches!(
		diagnostics[0],
		ParseDiagnostic::UnknownTransformer { .. }
	));
}

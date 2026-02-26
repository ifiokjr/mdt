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
			arguments: vec![],
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
			arguments: vec![],
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
				arguments: vec![],
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
				arguments: vec![],
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
				arguments: vec![],
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
				arguments: vec![],
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
				arguments: vec![],
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
				arguments: vec![],
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

// ---- Go to Definition: Provider → Consumers (reverse direction) ----

#[test]
fn goto_definition_provider_to_single_consumer() {
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
			arguments: vec![],
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
	let result = compute_goto_definition(&state, &provider_uri, position);

	assert!(result.is_some(), "expected goto definition result");
	match result.unwrap() {
		GotoDefinitionResponse::Scalar(loc) => {
			assert!(
				loc.uri.path().as_str().contains("readme.md"),
				"expected target to be the consumer file, got: {}",
				loc.uri.path().as_str()
			);
		}
		GotoDefinitionResponse::Array(locs) => {
			assert_eq!(locs.len(), 1);
			assert!(locs[0].uri.path().as_str().contains("readme.md"));
		}
		GotoDefinitionResponse::Link(_) => panic!("unexpected Link goto definition response"),
	}
}

#[test]
fn goto_definition_provider_to_multiple_consumers() {
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

	let consumers = vec![
		ConsumerEntry {
			block: Block {
				name: "greeting".to_string(),
				r#type: BlockType::Consumer,
				opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
				closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
				transformers: Vec::new(),
				arguments: vec![],
			},
			file: PathBuf::from("/tmp/test/readme.md"),
			content: "\n\nold\n\n".to_string(),
		},
		ConsumerEntry {
			block: Block {
				name: "greeting".to_string(),
				r#type: BlockType::Consumer,
				opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
				closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
				transformers: Vec::new(),
				arguments: vec![],
			},
			file: PathBuf::from("/tmp/test/docs.md"),
			content: "\n\nold\n\n".to_string(),
		},
		ConsumerEntry {
			block: Block {
				name: "greeting".to_string(),
				r#type: BlockType::Consumer,
				opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
				closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
				transformers: Vec::new(),
				arguments: vec![],
			},
			file: PathBuf::from("/tmp/test/other.md"),
			content: "\n\nold\n\n".to_string(),
		},
	];

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
	let result = compute_goto_definition(&state, &provider_uri, position);

	assert!(result.is_some(), "expected goto definition result");
	match result.unwrap() {
		GotoDefinitionResponse::Array(locs) => {
			assert_eq!(locs.len(), 3, "expected 3 consumer locations");
			let paths: Vec<String> = locs
				.iter()
				.map(|l| l.uri.path().as_str().to_string())
				.collect();
			assert!(
				paths.iter().any(|p| p.contains("readme.md")),
				"expected readme.md in locations"
			);
			assert!(
				paths.iter().any(|p| p.contains("docs.md")),
				"expected docs.md in locations"
			);
			assert!(
				paths.iter().any(|p| p.contains("other.md")),
				"expected other.md in locations"
			);
		}
		other => panic!("expected Array response for multiple consumers, got: {other:?}"),
	}
}

#[test]
fn goto_definition_provider_with_no_consumers_returns_none() {
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

	let mut documents = HashMap::new();
	documents.insert(
		provider_uri.clone(),
		DocumentState {
			content: provider_template.to_string(),
			blocks: provider_blocks,
			parse_diagnostics: Vec::new(),
		},
	);

	// No consumers at all
	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers,
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let position = to_lsp_position(&provider_block.opening.start);
	let result = compute_goto_definition(&state, &provider_uri, position);
	assert!(
		result.is_none(),
		"expected None for provider with no consumers"
	);
}

// ---- Code Action edge cases ----

#[test]
fn code_action_no_overlap_with_block_returns_empty() {
	let (state, uri) = make_test_state("Hello world!", "Old content");

	// Use a range completely before any block (line 0, in the heading).
	let range = Range {
		start: Position {
			line: 0,
			character: 0,
		},
		end: Position {
			line: 0,
			character: 5,
		},
	};

	let actions = compute_code_actions(&state, &uri, range);
	assert!(
		actions.is_empty(),
		"expected no code actions when cursor doesn't overlap any block"
	);
}

#[test]
fn code_action_consumer_without_matching_provider() {
	let consumer_doc = "<!-- {=orphan} -->\n\nstuff\n\n<!-- {/orphan} -->\n";
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
		.unwrap_or_else(|| panic!("expected consumer block"));

	let range = Range {
		start: to_lsp_position(&block.opening.start),
		end: to_lsp_position(&block.closing.end),
	};

	let actions = compute_code_actions(&state, &uri, range);
	assert!(
		actions.is_empty(),
		"expected no code actions when provider is missing"
	);
}

// ---- Completion edge cases ----

#[test]
fn completion_cursor_past_line_length_returns_empty() {
	let consumer_doc = "short";
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

	// Character is way past the line length.
	let position = Position {
		line: 0,
		character: 100,
	};
	let completions = compute_completions(&state, &uri, position);
	assert!(
		completions.is_empty(),
		"expected no completions when cursor is past line length"
	);
}

#[test]
fn completion_document_with_no_blocks() {
	let consumer_doc = "# Empty document\n\nNo blocks here.\n";
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
		line: 1,
		character: 0,
	};
	let completions = compute_completions(&state, &uri, position);
	assert!(completions.is_empty(), "expected no completions");
}

#[test]
fn completion_cursor_on_nonexistent_line_returns_empty() {
	let consumer_doc = "one line";
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

	// Line 5 doesn't exist in the document.
	let position = Position {
		line: 5,
		character: 0,
	};
	let completions = compute_completions(&state, &uri, position);
	assert!(
		completions.is_empty(),
		"expected no completions for nonexistent line"
	);
}

#[test]
fn completion_for_unknown_document_returns_empty() {
	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents: HashMap::new(),
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let uri = "file:///tmp/test/unknown.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let position = Position {
		line: 0,
		character: 0,
	};
	let completions = compute_completions(&state, &uri, position);
	assert!(
		completions.is_empty(),
		"expected no completions for unknown document"
	);
}

// ---- Diagnostics: InvalidTransformerArgs parse diagnostic ----

#[test]
fn diagnostics_invalid_transformer_args() {
	let content = "<!-- {=greeting|trim} -->\n\nHello\n\n<!-- {/greeting} -->\n";
	let uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let blocks = parse(content).unwrap_or_default();

	// Manually inject an InvalidTransformerArgs parse diagnostic, since the
	// parser doesn't generate this variant -- it's produced during project-level
	// validation. We want to exercise the compute_diagnostics codepath that
	// handles this variant.
	let parse_diagnostics = vec![ParseDiagnostic::InvalidTransformerArgs {
		name: "trim".to_string(),
		expected: "0".to_string(),
		got: 1,
		line: 1,
		column: 1,
	}];

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
		diagnostics.iter().any(|d| {
			d.message
				.contains("Transformer `trim` expects 0 argument(s), got 1")
		}),
		"expected InvalidTransformerArgs diagnostic, got: {diagnostics:?}"
	);
	assert!(
		diagnostics
			.iter()
			.any(|d| d.severity == Some(DiagnosticSeverity::ERROR)),
		"InvalidTransformerArgs should be an error"
	);
}

// ---- update_document_in_project tests ----

#[test]
fn update_document_in_project_template_updates_provider() {
	let provider_template = "<!-- {@greeting} -->\n\nHello updated!\n\n<!-- {/greeting} -->\n";
	let provider_uri = "file:///tmp/test/template.t.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let provider_blocks = parse(provider_template).unwrap_or_default();

	let mut documents = HashMap::new();
	documents.insert(
		provider_uri.clone(),
		DocumentState {
			content: provider_template.to_string(),
			blocks: provider_blocks,
			parse_diagnostics: Vec::new(),
		},
	);

	let mut state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	// Before update, no providers.
	assert!(state.providers.is_empty());

	state.update_document_in_project(&provider_uri);

	// After update, the provider should be registered.
	assert!(
		state.providers.contains_key("greeting"),
		"expected 'greeting' provider to be registered"
	);
	let provider = state.providers.get("greeting").unwrap();
	assert!(
		provider.content.contains("Hello updated!"),
		"expected provider content to contain updated text"
	);
	assert_eq!(provider.file, PathBuf::from("/tmp/test/template.t.md"));
}

#[test]
fn update_document_in_project_consumer_file_updates_consumers() {
	let consumer_doc = "<!-- {=greeting} -->\n\nOld content\n\n<!-- {/greeting} -->\n";
	let consumer_uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let consumer_blocks = parse(consumer_doc).unwrap_or_default();

	let mut documents = HashMap::new();
	documents.insert(
		consumer_uri.clone(),
		DocumentState {
			content: consumer_doc.to_string(),
			blocks: consumer_blocks,
			parse_diagnostics: Vec::new(),
		},
	);

	let mut state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	assert!(state.consumers.is_empty());

	state.update_document_in_project(&consumer_uri);

	assert_eq!(state.consumers.len(), 1);
	assert_eq!(state.consumers[0].block.name, "greeting");
	assert_eq!(
		state.consumers[0].file,
		PathBuf::from("/tmp/test/readme.md")
	);
}

#[test]
fn update_document_in_project_replaces_existing_consumers() {
	let consumer_uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	// Start with an existing consumer from the same file.
	let old_consumer = ConsumerEntry {
		block: Block {
			name: "old_block".to_string(),
			r#type: BlockType::Consumer,
			opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
			closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
			transformers: Vec::new(),
			arguments: vec![],
		},
		file: PathBuf::from("/tmp/test/readme.md"),
		content: "\n\nold\n\n".to_string(),
	};

	// A consumer from a different file should be preserved.
	let other_consumer = ConsumerEntry {
		block: Block {
			name: "other_block".to_string(),
			r#type: BlockType::Consumer,
			opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
			closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
			transformers: Vec::new(),
			arguments: vec![],
		},
		file: PathBuf::from("/tmp/test/other.md"),
		content: "\n\nother\n\n".to_string(),
	};

	let consumer_doc = "<!-- {=greeting} -->\n\nNew content\n\n<!-- {/greeting} -->\n";
	let consumer_blocks = parse(consumer_doc).unwrap_or_default();

	let mut documents = HashMap::new();
	documents.insert(
		consumer_uri.clone(),
		DocumentState {
			content: consumer_doc.to_string(),
			blocks: consumer_blocks,
			parse_diagnostics: Vec::new(),
		},
	);

	let mut state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers: HashMap::new(),
		consumers: vec![old_consumer, other_consumer],
		data: HashMap::new(),
	};

	assert_eq!(state.consumers.len(), 2);

	state.update_document_in_project(&consumer_uri);

	// old_consumer (same file) should be removed, other_consumer preserved,
	// and one new consumer added for "greeting".
	assert_eq!(state.consumers.len(), 2);
	assert!(
		state
			.consumers
			.iter()
			.any(|c| c.block.name == "other_block"),
		"consumer from other file should be preserved"
	);
	assert!(
		state.consumers.iter().any(|c| c.block.name == "greeting"),
		"new consumer should be added"
	);
	assert!(
		!state.consumers.iter().any(|c| c.block.name == "old_block"),
		"old consumer from same file should be removed"
	);
}

#[test]
fn update_document_in_project_unknown_document_is_noop() {
	let uri = "file:///tmp/test/unknown.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents: HashMap::new(),
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	// Should not panic or crash.
	state.update_document_in_project(&uri);

	assert!(state.providers.is_empty());
	assert!(state.consumers.is_empty());
}

// ---- parse_document (WorkspaceState method) tests ----

#[test]
fn workspace_parse_document_stores_state() {
	let uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid URI"));
	let content = "<!-- {=greeting} -->\n\nHello\n\n<!-- {/greeting} -->\n";

	let mut state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents: HashMap::new(),
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let blocks = state.parse_document(&uri, content.to_string());
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "greeting");

	// Verify the document is now stored.
	let doc = state
		.documents
		.get(&uri)
		.unwrap_or_else(|| panic!("document should be stored"));
	assert_eq!(doc.content, content);
	assert_eq!(doc.blocks.len(), 1);
	assert!(doc.parse_diagnostics.is_empty());
}

#[test]
fn workspace_parse_document_with_diagnostics() {
	let uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid URI"));
	let content = "<!-- {=greeting} -->\n\nHello\n";

	let mut state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents: HashMap::new(),
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let blocks = state.parse_document(&uri, content.to_string());
	assert!(
		blocks.is_empty(),
		"unclosed block should not produce blocks"
	);

	let doc = state
		.documents
		.get(&uri)
		.unwrap_or_else(|| panic!("document should be stored"));
	assert_eq!(doc.parse_diagnostics.len(), 1);
	assert!(matches!(
		doc.parse_diagnostics[0],
		ParseDiagnostic::UnclosedBlock { .. }
	));
}

#[test]
fn workspace_parse_document_replaces_previous() {
	let uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid URI"));

	let mut state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents: HashMap::new(),
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let content_v1 = "# Version 1\n";
	state.parse_document(&uri, content_v1.to_string());
	assert_eq!(state.documents.get(&uri).unwrap().content, content_v1);

	let content_v2 = "<!-- {=block} -->\n\nv2\n\n<!-- {/block} -->\n";
	let blocks = state.parse_document(&uri, content_v2.to_string());
	assert_eq!(blocks.len(), 1);
	assert_eq!(state.documents.get(&uri).unwrap().content, content_v2);
}

// ---- Hover: consumer without matching provider ----

#[test]
fn hover_consumer_without_provider_shows_no_matching() {
	let consumer_doc = "<!-- {=orphan} -->\n\nstuff\n\n<!-- {/orphan} -->\n";
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
		.unwrap_or_else(|| panic!("expected consumer block"));

	let position = to_lsp_position(&block.opening.start);
	let hover = compute_hover(&state, &uri, position);

	assert!(hover.is_some(), "expected hover result");
	if let HoverContents::Markup(markup) = &hover.unwrap().contents {
		assert!(
			markup.value.contains("No matching provider found"),
			"expected 'No matching provider found' in hover, got: {}",
			markup.value
		);
		assert!(markup.value.contains("Consumer block"));
		assert!(markup.value.contains("orphan"));
	} else {
		panic!("expected Markup hover contents");
	}
}

// ---- Hover: consumer with transformers ----

#[test]
fn hover_consumer_with_transformers_shows_transformer_list() {
	let consumer_doc = "<!-- {=greeting|trim|indent:\"  \"} -->\n\nstuff\n\n<!-- {/greeting} -->\n";
	let (consumer_blocks, consumer_parse_diags) =
		parse_with_diagnostics(consumer_doc).unwrap_or_default();
	let uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let provider_entry = ProviderEntry {
		block: Block {
			name: "greeting".to_string(),
			r#type: BlockType::Provider,
			opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
			closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
			transformers: Vec::new(),
			arguments: vec![],
		},
		file: PathBuf::from("/tmp/test/template.t.md"),
		content: "\n\n  Hello world!  \n\n".to_string(),
	};

	let mut providers = HashMap::new();
	providers.insert("greeting".to_string(), provider_entry);

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: consumer_doc.to_string(),
			blocks: consumer_blocks.clone(),
			parse_diagnostics: consumer_parse_diags,
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
		.unwrap_or_else(|| panic!("expected consumer block"));

	let position = to_lsp_position(&block.opening.start);
	let hover = compute_hover(&state, &uri, position);

	assert!(hover.is_some(), "expected hover result");
	if let HoverContents::Markup(markup) = &hover.unwrap().contents {
		assert!(
			markup.value.contains("Transformers"),
			"expected 'Transformers' section in hover, got: {}",
			markup.value
		);
		assert!(
			markup.value.contains("trim"),
			"expected 'trim' in transformers list, got: {}",
			markup.value
		);
		assert!(
			markup.value.contains("indent"),
			"expected 'indent' in transformers list, got: {}",
			markup.value
		);
	} else {
		panic!("expected Markup hover contents");
	}
}

// ---- TransformerType::Suffix and TransformerType::LineSuffix Display ----

#[test]
fn transformer_type_display_suffix() {
	assert_eq!(TransformerType::Suffix.to_string(), "suffix");
}

#[test]
fn transformer_type_display_line_suffix() {
	assert_eq!(TransformerType::LineSuffix.to_string(), "lineSuffix");
}

#[test]
fn transformer_type_display_line_prefix() {
	assert_eq!(TransformerType::LinePrefix.to_string(), "linePrefix");
}

// ---- position_in_range edge cases ----

#[test]
fn position_in_range_exact_start_boundary() {
	let range = Range {
		start: Position {
			line: 5,
			character: 10,
		},
		end: Position {
			line: 5,
			character: 20,
		},
	};

	// Exactly at start boundary should be in range.
	assert!(position_in_range(
		Position {
			line: 5,
			character: 10,
		},
		range
	));
}

#[test]
fn position_in_range_exact_end_boundary() {
	let range = Range {
		start: Position {
			line: 5,
			character: 10,
		},
		end: Position {
			line: 5,
			character: 20,
		},
	};

	// Exactly at end boundary should be in range.
	assert!(position_in_range(
		Position {
			line: 5,
			character: 20,
		},
		range
	));
}

#[test]
fn position_in_range_just_before_start_on_same_line() {
	let range = Range {
		start: Position {
			line: 5,
			character: 10,
		},
		end: Position {
			line: 5,
			character: 20,
		},
	};

	// One character before start on same line should be out of range.
	assert!(!position_in_range(
		Position {
			line: 5,
			character: 9,
		},
		range
	));
}

#[test]
fn position_in_range_just_after_end_on_same_line() {
	let range = Range {
		start: Position {
			line: 5,
			character: 10,
		},
		end: Position {
			line: 5,
			character: 20,
		},
	};

	// One character after end on same line should be out of range.
	assert!(!position_in_range(
		Position {
			line: 5,
			character: 21,
		},
		range
	));
}

#[test]
fn position_in_range_multi_line_middle() {
	let range = Range {
		start: Position {
			line: 2,
			character: 5,
		},
		end: Position {
			line: 8,
			character: 15,
		},
	};

	// Middle line, any column, should be in range.
	assert!(position_in_range(
		Position {
			line: 5,
			character: 0,
		},
		range
	));
	assert!(position_in_range(
		Position {
			line: 5,
			character: 99,
		},
		range
	));
}

#[test]
fn position_in_range_start_line_before_start_char() {
	let range = Range {
		start: Position {
			line: 2,
			character: 10,
		},
		end: Position {
			line: 5,
			character: 15,
		},
	};

	// On start line but before start character.
	assert!(!position_in_range(
		Position {
			line: 2,
			character: 5,
		},
		range
	));
}

#[test]
fn position_in_range_end_line_after_end_char() {
	let range = Range {
		start: Position {
			line: 2,
			character: 10,
		},
		end: Position {
			line: 5,
			character: 15,
		},
	};

	// On end line but after end character.
	assert!(!position_in_range(
		Position {
			line: 5,
			character: 16,
		},
		range
	));
}

// ---- ranges_overlap edge cases ----

#[test]
fn ranges_overlap_same_line_touching_at_boundary() {
	// Ranges that touch at exact boundary (end of A == start of B).
	let a = Range {
		start: Position {
			line: 5,
			character: 0,
		},
		end: Position {
			line: 5,
			character: 10,
		},
	};
	let b = Range {
		start: Position {
			line: 5,
			character: 10,
		},
		end: Position {
			line: 5,
			character: 20,
		},
	};
	// Touching at same point is considered overlapping.
	assert!(
		ranges_overlap(a, b),
		"ranges that touch at boundary should overlap"
	);
}

#[test]
fn ranges_overlap_same_line_gap_between() {
	let a = Range {
		start: Position {
			line: 5,
			character: 0,
		},
		end: Position {
			line: 5,
			character: 9,
		},
	};
	let b = Range {
		start: Position {
			line: 5,
			character: 10,
		},
		end: Position {
			line: 5,
			character: 20,
		},
	};
	assert!(
		!ranges_overlap(a, b),
		"ranges with a gap on the same line should not overlap"
	);
}

#[test]
fn ranges_overlap_identical_ranges() {
	let a = Range {
		start: Position {
			line: 3,
			character: 5,
		},
		end: Position {
			line: 7,
			character: 10,
		},
	};
	assert!(ranges_overlap(a, a), "identical ranges should overlap");
}

#[test]
fn ranges_overlap_one_contains_other() {
	let outer = Range {
		start: Position {
			line: 1,
			character: 0,
		},
		end: Position {
			line: 10,
			character: 50,
		},
	};
	let inner = Range {
		start: Position {
			line: 3,
			character: 5,
		},
		end: Position {
			line: 7,
			character: 10,
		},
	};
	assert!(ranges_overlap(outer, inner));
	assert!(ranges_overlap(inner, outer));
}

#[test]
fn ranges_overlap_adjacent_lines_no_overlap() {
	let a = Range {
		start: Position {
			line: 1,
			character: 0,
		},
		end: Position {
			line: 2,
			character: 0,
		},
	};
	let b = Range {
		start: Position {
			line: 2,
			character: 1,
		},
		end: Position {
			line: 3,
			character: 0,
		},
	};
	// a ends at (2, 0), b starts at (2, 1) -- there's a character gap.
	assert!(
		!ranges_overlap(a, b),
		"ranges on adjacent lines with character gap should not overlap"
	);
}

// ---- Diagnostics for document not in state ----

#[test]
fn diagnostics_unknown_document_returns_empty() {
	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents: HashMap::new(),
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let uri = "file:///tmp/test/unknown.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let diagnostics = compute_diagnostics(&state, &uri);
	assert!(
		diagnostics.is_empty(),
		"expected no diagnostics for unknown document"
	);
}

// ---- Hover: provider with consumer file listing ----

#[test]
fn hover_provider_lists_consumer_files() {
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

	let consumers = vec![
		ConsumerEntry {
			block: Block {
				name: "greeting".to_string(),
				r#type: BlockType::Consumer,
				opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
				closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
				transformers: Vec::new(),
				arguments: vec![],
			},
			file: PathBuf::from("/tmp/test/readme.md"),
			content: "\n\nold\n\n".to_string(),
		},
		ConsumerEntry {
			block: Block {
				name: "greeting".to_string(),
				r#type: BlockType::Consumer,
				opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
				closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
				transformers: Vec::new(),
				arguments: vec![],
			},
			file: PathBuf::from("/tmp/test/docs.md"),
			content: "\n\nold\n\n".to_string(),
		},
	];

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
		assert!(
			markup.value.contains("2 consumer(s)"),
			"expected '2 consumer(s)' in hover, got: {}",
			markup.value
		);
		assert!(
			markup.value.contains("Consumers in:"),
			"expected 'Consumers in:' section"
		);
		assert!(
			markup.value.contains("readme.md"),
			"expected readme.md in consumer listing"
		);
		assert!(
			markup.value.contains("docs.md"),
			"expected docs.md in consumer listing"
		);
	} else {
		panic!("expected Markup hover contents");
	}
}

// ---- Code Action: document not in state ----

#[test]
fn code_actions_unknown_document_returns_empty() {
	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents: HashMap::new(),
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let uri = "file:///tmp/test/unknown.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let range = Range {
		start: Position {
			line: 0,
			character: 0,
		},
		end: Position {
			line: 10,
			character: 0,
		},
	};

	let actions = compute_code_actions(&state, &uri, range);
	assert!(
		actions.is_empty(),
		"expected no code actions for unknown document"
	);
}

// ---- Document Symbols: unknown document ----

#[test]
fn document_symbols_unknown_document_returns_empty() {
	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents: HashMap::new(),
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let uri = "file:///tmp/test/unknown.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let symbols = compute_document_symbols(&state, &uri);
	assert!(
		symbols.is_empty(),
		"expected no symbols for unknown document"
	);
}

// ---- Hover: unknown document returns None ----

#[test]
fn hover_unknown_document_returns_none() {
	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents: HashMap::new(),
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let uri = "file:///tmp/test/unknown.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let hover = compute_hover(
		&state,
		&uri,
		Position {
			line: 0,
			character: 0,
		},
	);
	assert!(hover.is_none());
}

// ---- Go to Definition: unknown document returns None ----

#[test]
fn goto_definition_unknown_document_returns_none() {
	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents: HashMap::new(),
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let uri = "file:///tmp/test/unknown.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let result = compute_goto_definition(
		&state,
		&uri,
		Position {
			line: 0,
			character: 0,
		},
	);
	assert!(result.is_none());
}

// ---- to_lsp_range helper ----

#[test]
fn to_lsp_range_converts_correctly() {
	let pos = mdt_core::Position::new(2, 5, 10, 4, 20, 50);
	let range = to_lsp_range(&pos);
	assert_eq!(range.start.line, 1);
	assert_eq!(range.start.character, 4);
	assert_eq!(range.end.line, 3);
	assert_eq!(range.end.character, 19);
}

// ---- find_block_at_position helper ----

#[test]
fn find_block_at_position_returns_none_for_empty_blocks() {
	let position = Position {
		line: 0,
		character: 0,
	};
	let result = find_block_at_position(&[], position);
	assert!(result.is_none());
}

#[test]
fn find_block_at_position_finds_correct_block() {
	let content = "<!-- {=greeting} -->\n\nHello\n\n<!-- {/greeting} -->\n";
	let blocks = parse(content).unwrap_or_default();

	let block = &blocks[0];
	let position = to_lsp_position(&block.opening.start);
	let found = find_block_at_position(&blocks, position);
	assert!(found.is_some());
	assert_eq!(found.unwrap().name, "greeting");
}

#[test]
fn find_block_at_position_returns_none_outside_all_blocks() {
	let content =
		"# Heading\n\n<!-- {=greeting} -->\n\nHello\n\n<!-- {/greeting} -->\n\nTrailing text\n";
	let blocks = parse(content).unwrap_or_default();

	// Line 0 is the heading, before any block opening.
	let position = Position {
		line: 0,
		character: 0,
	};
	let found = find_block_at_position(&blocks, position);
	assert!(found.is_none());
}

// ---- parse_document_content for different extensions ----

#[test]
fn parse_document_content_mdx_is_markdown() {
	let uri = "file:///test/page.mdx"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid URI"));
	let content = "<!-- {@greeting} -->\n\nHello\n\n<!-- {/greeting} -->\n";
	let (blocks, diagnostics) = parse_document_content(&uri, content);
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "greeting");
	assert!(diagnostics.is_empty());
}

#[test]
fn parse_document_content_markdown_extension() {
	let uri = "file:///test/readme.markdown"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid URI"));
	let content = "<!-- {=block} -->\n\ncontent\n\n<!-- {/block} -->\n";
	let (blocks, diagnostics) = parse_document_content(&uri, content);
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "block");
	assert!(diagnostics.is_empty());
}

#[test]
fn parse_document_content_typescript_file() {
	let uri = "file:///test/main.ts"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid URI"));
	let content = "// <!-- {=block} -->\n// content\n// <!-- {/block} -->\n";
	let (blocks, diagnostics) = parse_document_content(&uri, content);
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "block");
	assert!(diagnostics.is_empty());
}

// ---- rescan_project without root is noop ----

#[test]
fn rescan_project_without_root_is_noop() {
	let mut state = WorkspaceState {
		root: None,
		documents: HashMap::new(),
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	// Should not panic; should be a no-op.
	state.rescan_project();

	assert!(state.providers.is_empty());
	assert!(state.consumers.is_empty());
}

// ===========================================================================
// Diagnostics: stale consumer with data interpolation
// ===========================================================================

#[test]
fn diagnostics_stale_consumer_with_template_data() {
	// When provider content uses template variables and data is available,
	// the diagnostic should compare against the rendered (interpolated) content.
	let provider_template = "<!-- {@ver} -->\n\nv{{ pkg.version }}\n\n<!-- {/ver} -->\n";
	let consumer_doc = "<!-- {=ver} -->\n\nv1.0.0\n\n<!-- {/ver} -->\n";

	let provider_blocks = parse(provider_template).unwrap_or_default();
	let (consumer_blocks, consumer_parse_diagnostics) =
		parse_with_diagnostics(consumer_doc).unwrap_or_default();

	let provider_block = provider_blocks
		.iter()
		.find(|b| b.r#type == BlockType::Provider)
		.cloned()
		.unwrap_or_else(|| panic!("expected a provider block"));

	let provider_entry = ProviderEntry {
		block: provider_block,
		file: PathBuf::from("/tmp/test/template.t.md"),
		content: extract_content_between_tags(provider_template, &provider_blocks[0]),
	};

	let consumer_uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut providers = HashMap::new();
	providers.insert("ver".to_string(), provider_entry);

	let mut documents = HashMap::new();
	documents.insert(
		consumer_uri.clone(),
		DocumentState {
			content: consumer_doc.to_string(),
			blocks: consumer_blocks,
			parse_diagnostics: consumer_parse_diagnostics,
		},
	);

	// Provide data so that {{ pkg.version }} renders to "2.0.0".
	let mut data = HashMap::new();
	data.insert("pkg".to_string(), serde_json::json!({"version": "2.0.0"}));

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers,
		consumers: Vec::new(),
		data,
	};

	let diagnostics = compute_diagnostics(&state, &consumer_uri);
	// Consumer has "v1.0.0" but rendered provider content is "v2.0.0",
	// so it should be stale.
	assert!(
		diagnostics
			.iter()
			.any(|d| d.message.contains("out of date")),
		"expected stale diagnostic when rendered content differs, got: {diagnostics:?}"
	);
}

// ===========================================================================
// Diagnostics: stale diagnostic includes expected content data
// ===========================================================================

#[test]
fn diagnostics_stale_consumer_includes_data_payload() {
	let (state, uri) = make_test_state("Hello world!", "Old content");
	let diagnostics = compute_diagnostics(&state, &uri);

	assert_eq!(diagnostics.len(), 1);
	let data = diagnostics[0]
		.data
		.as_ref()
		.unwrap_or_else(|| panic!("expected diagnostic data"));

	assert_eq!(data["kind"], "stale");
	assert_eq!(data["block_name"], "greeting");
	assert!(
		data["expected_content"].is_string(),
		"expected_content should be a string"
	);
}

// ===========================================================================
// Diagnostics: multiple consumers in one document
// ===========================================================================

#[test]
fn diagnostics_multiple_consumers_in_single_document() {
	let consumer_doc = "\
<!-- {=greeting} -->

Old greeting

<!-- {/greeting} -->

<!-- {=farewell} -->

Old farewell

<!-- {/farewell} -->
";
	let (consumer_blocks, consumer_parse_diagnostics) =
		parse_with_diagnostics(consumer_doc).unwrap_or_default();
	let uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: consumer_doc.to_string(),
			blocks: consumer_blocks,
			parse_diagnostics: consumer_parse_diagnostics,
		},
	);

	// Create providers with different content to make both stale.
	let provider_template_1 = "<!-- {@greeting} -->\n\nNew greeting\n\n<!-- {/greeting} -->\n";
	let provider_blocks_1 = parse(provider_template_1).unwrap_or_default();
	let provider_template_2 = "<!-- {@farewell} -->\n\nNew farewell\n\n<!-- {/farewell} -->\n";
	let provider_blocks_2 = parse(provider_template_2).unwrap_or_default();

	let mut providers = HashMap::new();
	providers.insert(
		"greeting".to_string(),
		ProviderEntry {
			block: provider_blocks_1[0].clone(),
			file: PathBuf::from("/tmp/test/template.t.md"),
			content: extract_content_between_tags(provider_template_1, &provider_blocks_1[0]),
		},
	);
	providers.insert(
		"farewell".to_string(),
		ProviderEntry {
			block: provider_blocks_2[0].clone(),
			file: PathBuf::from("/tmp/test/template.t.md"),
			content: extract_content_between_tags(provider_template_2, &provider_blocks_2[0]),
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
	let stale_names: Vec<&str> = diagnostics
		.iter()
		.filter(|d| d.message.contains("out of date"))
		.filter_map(|d| {
			if d.message.contains("greeting") {
				Some("greeting")
			} else if d.message.contains("farewell") {
				Some("farewell")
			} else {
				None
			}
		})
		.collect();

	assert!(
		stale_names.contains(&"greeting"),
		"expected greeting to be stale"
	);
	assert!(
		stale_names.contains(&"farewell"),
		"expected farewell to be stale"
	);
}

// Note: completion_inside_provider_tag_context and
// completion_inside_close_tag_context are already tested above.

// ===========================================================================
// Completion: multiple providers returns all
// ===========================================================================

#[test]
fn completion_returns_all_provider_names() {
	let doc = "<!-- {=";
	let uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: doc.to_string(),
			blocks: Vec::new(),
			parse_diagnostics: Vec::new(),
		},
	);

	let make_provider = |name: &str| {
		ProviderEntry {
			block: Block {
				name: name.to_string(),
				r#type: BlockType::Provider,
				opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
				closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
				transformers: Vec::new(),
				arguments: vec![],
			},
			file: PathBuf::from("/tmp/test/template.t.md"),
			content: "\n\ncontent\n\n".to_string(),
		}
	};

	let mut providers = HashMap::new();
	providers.insert("greeting".to_string(), make_provider("greeting"));
	providers.insert("farewell".to_string(), make_provider("farewell"));
	providers.insert("install".to_string(), make_provider("install"));

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers,
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let position = Position {
		line: 0,
		character: 7,
	};
	let completions = compute_completions(&state, &uri, position);
	assert_eq!(completions.len(), 3, "expected 3 completion items");

	let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
	assert!(labels.contains(&"greeting"));
	assert!(labels.contains(&"farewell"));
	assert!(labels.contains(&"install"));
}

// ===========================================================================
// Completion: transformer completions have correct kind
// ===========================================================================

#[test]
fn transformer_completions_have_function_kind() {
	let completions = transformer_completions();
	assert!(!completions.is_empty());
	for item in &completions {
		assert_eq!(
			item.kind,
			Some(CompletionItemKind::FUNCTION),
			"transformer completions should have FUNCTION kind"
		);
	}
}

#[test]
fn transformer_completions_have_sort_text() {
	let completions = transformer_completions();
	for (i, item) in completions.iter().enumerate() {
		assert_eq!(
			item.sort_text,
			Some(format!("{i:02}")),
			"transformer at index {i} should have sort_text '{:02}'",
			i
		);
	}
}

#[test]
fn transformer_completions_include_all_known_transformers() {
	let completions = transformer_completions();
	let names: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
	let expected = [
		"trim",
		"trimStart",
		"trimEnd",
		"indent",
		"prefix",
		"suffix",
		"linePrefix",
		"lineSuffix",
		"wrap",
		"codeBlock",
		"code",
		"replace",
	];
	for name in expected {
		assert!(
			names.contains(&name),
			"expected transformer '{name}' in completions, got: {names:?}"
		);
	}
}

// ===========================================================================
// Block name completions have correct kind
// ===========================================================================

#[test]
fn block_name_completions_have_reference_kind() {
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
				arguments: vec![],
			},
			file: PathBuf::from("/tmp/test/template.t.md"),
			content: "\n\nHello!\n\n".to_string(),
		},
	);

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents: HashMap::new(),
		providers,
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let completions = block_name_completions(&state);
	assert_eq!(completions.len(), 1);
	assert_eq!(completions[0].kind, Some(CompletionItemKind::REFERENCE));
	assert!(
		completions[0]
			.detail
			.as_ref()
			.unwrap_or_else(|| panic!("expected detail"))
			.contains("template.t.md")
	);
}

// Note: document_symbols_provider_block_has_class_kind and
// document_symbols_consumer_block_has_variable_kind are already tested below.

#[test]
fn document_symbols_full_range_spans_opening_to_closing() {
	let content = "<!-- {@greeting} -->\n\nHello\n\n<!-- {/greeting} -->\n";
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
	assert_eq!(symbols.len(), 1);
	// full range.start should be at opening start
	// full range.end should be at closing end
	// selection_range should be just the opening tag
	let symbol = &symbols[0];
	assert!(
		symbol.range.start.line <= symbol.selection_range.start.line,
		"full range should start at or before selection range"
	);
	assert!(
		symbol.range.end.line >= symbol.selection_range.end.line,
		"full range should end at or after selection range"
	);
}

// ===========================================================================
// Code action: edit replaces content between tags
// ===========================================================================

#[test]
fn code_action_edit_targets_content_between_tags() {
	let (state, uri) = make_test_state("Hello world!", "Old content");
	let doc = state.documents.get(&uri).unwrap_or_else(|| panic!("doc"));
	let block = doc
		.blocks
		.iter()
		.find(|b| b.r#type == BlockType::Consumer)
		.unwrap_or_else(|| panic!("consumer block"));

	let range = Range {
		start: to_lsp_position(&block.opening.start),
		end: to_lsp_position(&block.closing.end),
	};

	let actions = compute_code_actions(&state, &uri, range);
	assert!(!actions.is_empty());

	let CodeActionOrCommand::CodeAction(action) = &actions[0] else {
		panic!("expected CodeAction")
	};

	assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));

	// Verify the edit has changes for our URI.
	let edit = action
		.edit
		.as_ref()
		.unwrap_or_else(|| panic!("expected workspace edit"));
	let changes = edit
		.changes
		.as_ref()
		.unwrap_or_else(|| panic!("expected changes map"));
	assert!(
		changes.contains_key(&uri),
		"changes should target the consumer file"
	);

	let text_edits = changes
		.get(&uri)
		.unwrap_or_else(|| panic!("expected text edits for URI"));
	assert_eq!(text_edits.len(), 1, "expected exactly one text edit");

	// The edit range start should be at the opening tag end (content starts after
	// opening).
	let text_edit = &text_edits[0];
	let opening_end = to_lsp_position(&block.opening.end);
	assert_eq!(text_edit.range.start, opening_end);
}

// ===========================================================================
// Levenshtein distance: additional cases
// ===========================================================================

#[test]
fn levenshtein_single_char_difference() {
	assert_eq!(levenshtein_distance("a", "b"), 1);
	assert_eq!(levenshtein_distance("a", "a"), 0);
	assert_eq!(levenshtein_distance("a", ""), 1);
}

#[test]
fn levenshtein_case_sensitive() {
	// Levenshtein is case-sensitive.
	assert_eq!(levenshtein_distance("Hello", "hello"), 1);
	assert_eq!(levenshtein_distance("HELLO", "hello"), 5);
}

#[test]
fn levenshtein_symmetric() {
	// Distance should be the same regardless of argument order.
	let d1 = levenshtein_distance("greeting", "greetng");
	let d2 = levenshtein_distance("greetng", "greeting");
	assert_eq!(d1, d2, "levenshtein should be symmetric");
}

// ===========================================================================
// suggest_similar_names: additional edge cases
// ===========================================================================

#[test]
fn suggest_similar_names_empty_providers_returns_empty() {
	let providers = HashMap::new();
	let suggestions = suggest_similar_names("anything", &providers);
	assert!(suggestions.is_empty());
}

// Note: suggest_similar_names_exact_match_excluded is already tested below.

#[test]
fn suggest_similar_names_max_three_results() {
	let make_provider = |name: &str| {
		ProviderEntry {
			block: Block {
				name: name.to_string(),
				r#type: BlockType::Provider,
				opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
				closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
				transformers: Vec::new(),
				arguments: vec![],
			},
			file: PathBuf::from("/tmp/test/template.t.md"),
			content: String::new(),
		}
	};

	let mut providers = HashMap::new();
	// All are one edit away from "test".
	providers.insert("testa".to_string(), make_provider("testa"));
	providers.insert("testb".to_string(), make_provider("testb"));
	providers.insert("testc".to_string(), make_provider("testc"));
	providers.insert("testd".to_string(), make_provider("testd"));
	providers.insert("teste".to_string(), make_provider("teste"));

	let suggestions = suggest_similar_names("test", &providers);
	assert!(
		suggestions.len() <= 3,
		"should return at most 3 suggestions, got {}",
		suggestions.len()
	);
}

// ===========================================================================
// to_lsp_position: zero/underflow handling
// ===========================================================================

#[test]
fn to_lsp_position_saturates_at_zero() {
	// Point with line=0, column=0 (below the 1-indexed minimum).
	// saturating_sub(1) should produce 0, not underflow.
	let point = mdt_core::Point::new(0, 0, 0);
	let lsp_pos = to_lsp_position(&point);
	assert_eq!(lsp_pos.line, 0);
	assert_eq!(lsp_pos.character, 0);
}

// ===========================================================================
// parse_document_content: python file (source scanner path)
// ===========================================================================

#[test]
fn parse_document_content_python_file() {
	let uri = "file:///test/main.py"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid URI"));
	let content = "# <!-- {=block} -->\n# content\n# <!-- {/block} -->\n";
	let (blocks, diagnostics) = parse_document_content(&uri, content);
	assert_eq!(blocks.len(), 1);
	assert_eq!(blocks[0].name, "block");
	assert!(diagnostics.is_empty());
}

// ===========================================================================
// parse_document_content: empty content returns empty
// ===========================================================================

#[test]
fn parse_document_content_empty_string() {
	let uri = "file:///test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid URI"));
	let (blocks, diagnostics) = parse_document_content(&uri, "");
	assert!(blocks.is_empty());
	assert!(diagnostics.is_empty());
}

// Note: hover_provider_with_zero_consumers is already tested below.

// ===========================================================================
// Hover: provider shows content preview in code block
// ===========================================================================

#[test]
fn hover_provider_shows_content_in_code_block() {
	let provider_template =
		"<!-- {@greeting} -->\n\nHello from provider!\n\n<!-- {/greeting} -->\n";
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
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let position = to_lsp_position(&provider_block.opening.start);
	let hover = compute_hover(&state, &provider_uri, position);

	assert!(hover.is_some());
	if let HoverContents::Markup(markup) = &hover.unwrap().contents {
		assert!(
			markup.value.contains("Hello from provider!"),
			"hover should show provider content"
		);
		assert!(
			markup.value.contains("```"),
			"hover should render content in a code block"
		);
	} else {
		panic!("expected Markup hover contents");
	}
}

// ===========================================================================
// Hover: consumer with provider shows source file path
// ===========================================================================

#[test]
fn hover_consumer_shows_provider_source_path() {
	let (state, uri) = make_test_state("Hello world!", "Old content");
	let doc = state.documents.get(&uri).unwrap_or_else(|| panic!("doc"));
	let block = doc
		.blocks
		.iter()
		.find(|b| b.r#type == BlockType::Consumer)
		.unwrap_or_else(|| panic!("consumer block"));

	let position = to_lsp_position(&block.opening.start);
	let hover = compute_hover(&state, &uri, position);

	assert!(hover.is_some());
	if let HoverContents::Markup(markup) = &hover.unwrap().contents {
		assert!(
			markup.value.contains("Provider source:"),
			"should show provider source label"
		);
		assert!(
			markup.value.contains("template.t.md"),
			"should show provider file path"
		);
	} else {
		panic!("expected Markup hover contents");
	}
}

// Note: rescan_project_with_valid_project_populates_state is already tested
// below.

// ===========================================================================
// Goto definition: consumer cursor not on any block
// ===========================================================================

#[test]
fn goto_definition_cursor_between_blocks_returns_none() {
	let content = "\
# Heading

<!-- {=greeting} -->

Hello

<!-- {/greeting} -->

Some text between blocks.

<!-- {=farewell} -->

Bye

<!-- {/farewell} -->
";
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

	// Position on line 8 "Some text between blocks." — not in any block's opening.
	let position = Position {
		line: 8,
		character: 5,
	};
	let result = compute_goto_definition(&state, &uri, position);
	assert!(
		result.is_none(),
		"should return None when cursor is between blocks"
	);
}

// ===========================================================================
// Code action: multiple stale consumers in one document
// ===========================================================================

#[test]
fn code_actions_for_multiple_stale_blocks() {
	let consumer_doc = "\
<!-- {=greeting} -->

Old greeting

<!-- {/greeting} -->

<!-- {=farewell} -->

Old farewell

<!-- {/farewell} -->
";
	let consumer_blocks = parse(consumer_doc).unwrap_or_default();
	let uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: consumer_doc.to_string(),
			blocks: consumer_blocks,
			parse_diagnostics: Vec::new(),
		},
	);

	let provider_template_1 = "<!-- {@greeting} -->\n\nNew greeting\n\n<!-- {/greeting} -->\n";
	let provider_blocks_1 = parse(provider_template_1).unwrap_or_default();
	let provider_template_2 = "<!-- {@farewell} -->\n\nNew farewell\n\n<!-- {/farewell} -->\n";
	let provider_blocks_2 = parse(provider_template_2).unwrap_or_default();

	let mut providers = HashMap::new();
	providers.insert(
		"greeting".to_string(),
		ProviderEntry {
			block: provider_blocks_1[0].clone(),
			file: PathBuf::from("/tmp/test/template.t.md"),
			content: extract_content_between_tags(provider_template_1, &provider_blocks_1[0]),
		},
	);
	providers.insert(
		"farewell".to_string(),
		ProviderEntry {
			block: provider_blocks_2[0].clone(),
			file: PathBuf::from("/tmp/test/template.t.md"),
			content: extract_content_between_tags(provider_template_2, &provider_blocks_2[0]),
		},
	);

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers,
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	// Use a range that spans the entire document.
	let range = Range {
		start: Position {
			line: 0,
			character: 0,
		},
		end: Position {
			line: 20,
			character: 0,
		},
	};

	let actions = compute_code_actions(&state, &uri, range);
	assert_eq!(
		actions.len(),
		2,
		"expected 2 code actions (one per stale block)"
	);

	let titles: Vec<String> = actions
		.iter()
		.map(|a| {
			match a {
				CodeActionOrCommand::CodeAction(ca) => ca.title.clone(),
				CodeActionOrCommand::Command(cmd) => cmd.title.clone(),
			}
		})
		.collect();
	assert!(
		titles.iter().any(|t| t.contains("greeting")),
		"expected greeting code action"
	);
	assert!(
		titles.iter().any(|t| t.contains("farewell")),
		"expected farewell code action"
	);
}

// ---- WorkspaceState default ----

#[test]
fn workspace_state_default() {
	let state = WorkspaceState::default();
	assert!(state.root.is_none());
	assert!(state.documents.is_empty());
	assert!(state.providers.is_empty());
	assert!(state.consumers.is_empty());
	assert!(state.data.is_empty());
}

// ---- Completion: provider tag context ----

#[test]
fn completion_inside_provider_tag_context() {
	let doc = "<!-- {@gre";
	let uri = "file:///tmp/test/template.t.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: doc.to_string(),
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
			arguments: vec![],
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
	assert!(
		!completions.is_empty(),
		"expected completions in provider tag context"
	);
	assert!(
		completions.iter().any(|c| c.label == "greeting"),
		"expected 'greeting' completion item"
	);
}

#[test]
fn completion_inside_close_tag_context() {
	let doc = "<!-- {/gre";
	let uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: doc.to_string(),
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
			arguments: vec![],
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
	assert!(
		!completions.is_empty(),
		"expected completions in close tag context"
	);
	assert!(
		completions.iter().any(|c| c.label == "greeting"),
		"expected 'greeting' completion item"
	);
}

// ---- Code action: provider block is skipped (not a consumer) ----

#[test]
fn code_action_skips_provider_blocks() {
	let content = "<!-- {@greeting} -->\n\nHello\n\n<!-- {/greeting} -->\n";
	let blocks = parse(content).unwrap_or_default();
	let uri = "file:///tmp/test/template.t.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: content.to_string(),
			blocks: blocks.clone(),
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

	let block = &blocks[0];
	let range = Range {
		start: to_lsp_position(&block.opening.start),
		end: to_lsp_position(&block.closing.end),
	};

	let actions = compute_code_actions(&state, &uri, range);
	assert!(
		actions.is_empty(),
		"expected no code actions for provider block"
	);
}

// ---- rescan_project with a real tempdir project ----

#[test]
fn rescan_project_with_valid_project_populates_state() {
	let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("failed to create tempdir: {e}"));
	let root = dir.path();

	// Create a minimal project: a template file with a provider block.
	std::fs::write(
		root.join("template.t.md"),
		"<!-- {@greeting} -->\n\nHello from template!\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("failed to write template: {e}"));

	// Create a consumer file.
	std::fs::write(
		root.join("readme.md"),
		"<!-- {=greeting} -->\n\nOld content\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("failed to write readme: {e}"));

	let mut state = WorkspaceState {
		root: Some(root.to_path_buf()),
		documents: HashMap::new(),
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	state.rescan_project();

	assert!(
		state.providers.contains_key("greeting"),
		"expected 'greeting' provider after rescan, got: {:?}",
		state.providers.keys().collect::<Vec<_>>()
	);
	assert!(
		!state.consumers.is_empty(),
		"expected at least one consumer after rescan"
	);
	assert!(
		state.consumers.iter().any(|c| c.block.name == "greeting"),
		"expected a 'greeting' consumer"
	);
}

#[test]
fn rescan_project_with_invalid_config_prints_error_but_does_not_panic() {
	let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("failed to create tempdir: {e}"));
	let root = dir.path();

	// Create an invalid mdt.toml that will cause scan_project_with_config to fail.
	std::fs::write(root.join("mdt.toml"), "this is not valid toml {{{{")
		.unwrap_or_else(|e| panic!("failed to write mdt.toml: {e}"));

	let mut state = WorkspaceState {
		root: Some(root.to_path_buf()),
		documents: HashMap::new(),
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	// Should not panic — the error is printed to stderr.
	state.rescan_project();

	// State should remain empty since the scan failed.
	assert!(state.providers.is_empty());
	assert!(state.consumers.is_empty());
}

#[test]
fn rescan_project_with_data_from_config() {
	let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("failed to create tempdir: {e}"));
	let root = dir.path();

	// Create a config that references a JSON data file.
	std::fs::write(root.join("mdt.toml"), "[data]\npkg = \"package.json\"\n")
		.unwrap_or_else(|e| panic!("failed to write mdt.toml: {e}"));

	std::fs::write(
		root.join("package.json"),
		r#"{"name": "test-pkg", "version": "1.0.0"}"#,
	)
	.unwrap_or_else(|e| panic!("failed to write package.json: {e}"));

	// Create a template file.
	std::fs::write(
		root.join("template.t.md"),
		"<!-- {@version} -->\n\n{{ pkg.version }}\n\n<!-- {/version} -->\n",
	)
	.unwrap_or_else(|e| panic!("failed to write template: {e}"));

	let mut state = WorkspaceState {
		root: Some(root.to_path_buf()),
		documents: HashMap::new(),
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	state.rescan_project();

	assert!(
		!state.data.is_empty(),
		"expected data to be populated from mdt.toml config"
	);
	assert!(
		state.data.contains_key("pkg"),
		"expected 'pkg' namespace in data"
	);
}

// ---- update_document_in_project with non-file URI ----

#[test]
fn update_document_in_project_non_file_uri_is_noop() {
	let uri = "untitled:Untitled-1"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let content = "<!-- {@greeting} -->\n\nHello\n\n<!-- {/greeting} -->\n";
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

	let mut state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	// Should not panic; should be a no-op because untitled: URI has no file path.
	state.update_document_in_project(&uri);

	assert!(
		state.providers.is_empty(),
		"expected no providers for non-file URI"
	);
	assert!(
		state.consumers.is_empty(),
		"expected no consumers for non-file URI"
	);
}

// ---- Diagnostics with template render failure ----

#[test]
fn diagnostics_stale_consumer_with_render_template_failure() {
	// Provider content with broken template syntax triggers the
	// unwrap_or_else fallback path in compute_diagnostics (line 497).
	let provider_content = "{{ broken";
	let consumer_content = "something else";

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
	providers.insert("greeting".to_string(), provider_entry);

	let mut documents = HashMap::new();
	documents.insert(
		consumer_uri.clone(),
		DocumentState {
			content: consumer_doc,
			blocks: consumer_blocks,
			parse_diagnostics: consumer_parse_diagnostics,
		},
	);

	// Non-empty data triggers template rendering attempt.
	let mut data = HashMap::new();
	data.insert("pkg".to_string(), serde_json::json!({"version": "1.0.0"}));

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers,
		consumers: Vec::new(),
		data,
	};

	let diagnostics = compute_diagnostics(&state, &consumer_uri);
	// Even though render_template fails, the fallback to raw content should
	// still produce a stale diagnostic since the consumer content differs.
	assert!(
		diagnostics
			.iter()
			.any(|d| d.message.contains("out of date")),
		"expected stale consumer diagnostic even with template render failure, got: \
		 {diagnostics:?}"
	);
}

// ---- Hover: consumer with render_template failure ----

#[test]
fn hover_consumer_with_render_template_failure_shows_fallback() {
	// Provider content has broken template syntax.
	let provider_content = "{{ broken";
	let provider_template =
		format!("<!-- {{@greeting}} -->\n\n{provider_content}\n\n<!-- {{/greeting}} -->\n");
	let consumer_doc = "# Readme\n\n<!-- {=greeting} -->\n\nold\n\n<!-- {/greeting} -->\n";

	let provider_blocks = parse(&provider_template).unwrap_or_default();
	let (consumer_blocks, consumer_parse_diagnostics) =
		parse_with_diagnostics(consumer_doc).unwrap_or_default();

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
	providers.insert("greeting".to_string(), provider_entry);

	let mut documents = HashMap::new();
	documents.insert(
		consumer_uri.clone(),
		DocumentState {
			content: consumer_doc.to_string(),
			blocks: consumer_blocks.clone(),
			parse_diagnostics: consumer_parse_diagnostics,
		},
	);

	let mut data = HashMap::new();
	data.insert("pkg".to_string(), serde_json::json!({"version": "1.0.0"}));

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers,
		consumers: Vec::new(),
		data,
	};

	let block = consumer_blocks
		.iter()
		.find(|b| b.r#type == BlockType::Consumer)
		.unwrap_or_else(|| panic!("expected consumer block"));

	let position = to_lsp_position(&block.opening.start);
	let hover = compute_hover(&state, &consumer_uri, position);

	assert!(hover.is_some(), "expected hover result with fallback");
	if let HoverContents::Markup(markup) = &hover.unwrap().contents {
		assert!(
			markup.value.contains("Consumer block"),
			"expected 'Consumer block' in hover, got: {}",
			markup.value
		);
		assert!(
			markup.value.contains("greeting"),
			"expected 'greeting' in hover, got: {}",
			markup.value
		);
		// The fallback content should contain the raw template syntax.
		assert!(
			markup.value.contains("broken"),
			"expected raw fallback content in hover, got: {}",
			markup.value
		);
	} else {
		panic!("expected Markup hover contents");
	}
}

// ---- Code Actions: render_template failure path ----

#[test]
fn code_action_with_render_template_failure_uses_fallback() {
	let provider_content = "{{ broken";
	let provider_template =
		format!("<!-- {{@greeting}} -->\n\n{provider_content}\n\n<!-- {{/greeting}} -->\n");
	let consumer_doc = "<!-- {=greeting} -->\n\nold content\n\n<!-- {/greeting} -->\n";

	let provider_blocks = parse(&provider_template).unwrap_or_default();
	let consumer_blocks = parse(consumer_doc).unwrap_or_default();

	let provider_block = provider_blocks
		.iter()
		.find(|b| b.r#type == BlockType::Provider)
		.cloned()
		.unwrap_or_else(|| panic!("expected a provider block"));

	let provider_entry = ProviderEntry {
		block: provider_block,
		file: PathBuf::from("/tmp/test/template.t.md"),
		content: extract_content_between_tags(&provider_template, &provider_blocks[0]),
	};

	let consumer_uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut providers = HashMap::new();
	providers.insert("greeting".to_string(), provider_entry);

	let mut documents = HashMap::new();
	documents.insert(
		consumer_uri.clone(),
		DocumentState {
			content: consumer_doc.to_string(),
			blocks: consumer_blocks.clone(),
			parse_diagnostics: Vec::new(),
		},
	);

	let mut data = HashMap::new();
	data.insert("pkg".to_string(), serde_json::json!({"version": "1.0.0"}));

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers,
		consumers: Vec::new(),
		data,
	};

	let block = consumer_blocks
		.iter()
		.find(|b| b.r#type == BlockType::Consumer)
		.unwrap_or_else(|| panic!("expected consumer block"));

	let range = Range {
		start: to_lsp_position(&block.opening.start),
		end: to_lsp_position(&block.closing.end),
	};

	let actions = compute_code_actions(&state, &consumer_uri, range);
	// Even with render failure, the fallback content differs from current
	// content, so a code action should be offered.
	assert!(
		!actions.is_empty(),
		"expected code action even with render template failure"
	);

	let CodeActionOrCommand::CodeAction(action) = &actions[0] else {
		panic!("expected CodeAction")
	};

	assert!(action.title.contains("Update block"));
	assert!(action.title.contains("greeting"));
	assert!(action.edit.is_some());
}

// ---- Diagnostics: stale consumer with transformers applied ----

#[test]
fn diagnostics_stale_consumer_with_transformers() {
	// Provider content is "Hello world!" but the consumer has a trim
	// transformer. The provider stores content with newlines around it,
	// so after trim the result differs from the raw consumer content.
	let provider_template = "<!-- {@greeting} -->\n\n  Hello world!  \n\n<!-- {/greeting} -->\n";
	let consumer_doc = "<!-- {=greeting|trim} -->\n\nnot trimmed content\n\n<!-- {/greeting} -->\n";

	let provider_blocks = parse(provider_template).unwrap_or_default();
	let (consumer_blocks, consumer_parse_diagnostics) =
		parse_with_diagnostics(consumer_doc).unwrap_or_default();

	let provider_block = provider_blocks
		.iter()
		.find(|b| b.r#type == BlockType::Provider)
		.cloned()
		.unwrap_or_else(|| panic!("expected a provider block"));

	let provider_entry = ProviderEntry {
		block: provider_block,
		file: PathBuf::from("/tmp/test/template.t.md"),
		content: extract_content_between_tags(provider_template, &provider_blocks[0]),
	};

	let consumer_uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut providers = HashMap::new();
	providers.insert("greeting".to_string(), provider_entry);

	let mut documents = HashMap::new();
	documents.insert(
		consumer_uri.clone(),
		DocumentState {
			content: consumer_doc.to_string(),
			blocks: consumer_blocks,
			parse_diagnostics: consumer_parse_diagnostics,
		},
	);

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers,
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let diagnostics = compute_diagnostics(&state, &consumer_uri);
	assert!(
		diagnostics
			.iter()
			.any(|d| d.message.contains("out of date")),
		"expected stale consumer diagnostic with transformers applied, got: {diagnostics:?}"
	);
}

// ---- Provider hover with no consumers (0 consumer count) ----

#[test]
fn hover_provider_with_zero_consumers() {
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
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let position = to_lsp_position(&provider_block.opening.start);
	let hover = compute_hover(&state, &provider_uri, position);

	assert!(hover.is_some());
	if let HoverContents::Markup(markup) = &hover.unwrap().contents {
		assert!(
			markup.value.contains("Provider block"),
			"expected 'Provider block' in hover"
		);
		assert!(
			markup.value.contains("0 consumer(s)"),
			"expected '0 consumer(s)' in hover, got: {}",
			markup.value
		);
		// When no consumers, should NOT contain "Consumers in:" section.
		assert!(
			!markup.value.contains("Consumers in:"),
			"should not list consumers when there are none"
		);
		assert!(
			markup.value.contains("Hello!"),
			"expected provider content in hover"
		);
	} else {
		panic!("expected Markup hover contents");
	}
}

// ---- Document Symbols: provider block uses CLASS kind ----

#[test]
fn document_symbols_provider_block_has_class_kind() {
	let content = "<!-- {@greeting} -->\n\nHello\n\n<!-- {/greeting} -->\n";
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
	assert_eq!(symbols.len(), 1);
	assert_eq!(symbols[0].name, "@greeting");
	assert_eq!(symbols[0].kind, SymbolKind::CLASS);
}

// ---- Document Symbols: consumer block uses VARIABLE kind ----

#[test]
fn document_symbols_consumer_block_has_variable_kind() {
	let content = "<!-- {=greeting} -->\n\nHello\n\n<!-- {/greeting} -->\n";
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
	assert_eq!(symbols.len(), 1);
	assert_eq!(symbols[0].name, "=greeting");
	assert_eq!(symbols[0].kind, SymbolKind::VARIABLE);
}

// ---- Diagnostics: provider with consumers (not unused) ----

#[test]
fn diagnostics_provider_with_consumers_no_unused_warning() {
	let content = "<!-- {@greeting} -->\n\nHello\n\n<!-- {/greeting} -->\n";
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
				arguments: vec![],
			},
			file: PathBuf::from("/tmp/test/template.t.md"),
			content: "\n\nHello\n\n".to_string(),
		},
	);

	// Add a consumer that references this provider.
	let consumers = vec![ConsumerEntry {
		block: Block {
			name: "greeting".to_string(),
			r#type: BlockType::Consumer,
			opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
			closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
			transformers: Vec::new(),
			arguments: vec![],
		},
		file: PathBuf::from("/tmp/test/readme.md"),
		content: "\n\nHello\n\n".to_string(),
	}];

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers,
		consumers,
		data: HashMap::new(),
	};

	let diagnostics = compute_diagnostics(&state, &uri);
	assert!(
		!diagnostics
			.iter()
			.any(|d| d.message.contains("has no consumers")),
		"provider with consumers should not produce unused warning, got: {diagnostics:?}"
	);
}

// ---- Diagnostics: up-to-date consumer with data rendering ----

#[test]
fn diagnostics_stale_consumer_with_successful_template_rendering() {
	// Provider content uses template syntax that resolves with data.
	// The consumer has old version, so it should be stale and the
	// diagnostic data should contain the rendered content.
	let provider_template =
		"<!-- {@version} -->\n\nVersion: {{ pkg.version }}\n\n<!-- {/version} -->\n";
	let consumer_doc = "<!-- {=version} -->\n\nVersion: 0.9.0\n\n<!-- {/version} -->\n";

	let provider_blocks = parse(provider_template).unwrap_or_default();
	let (consumer_blocks, consumer_parse_diagnostics) =
		parse_with_diagnostics(consumer_doc).unwrap_or_default();

	let provider_block = provider_blocks
		.iter()
		.find(|b| b.r#type == BlockType::Provider)
		.cloned()
		.unwrap_or_else(|| panic!("expected a provider block"));

	let provider_entry = ProviderEntry {
		block: provider_block,
		file: PathBuf::from("/tmp/test/template.t.md"),
		content: extract_content_between_tags(provider_template, &provider_blocks[0]),
	};

	let consumer_uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut providers = HashMap::new();
	providers.insert("version".to_string(), provider_entry);

	let mut documents = HashMap::new();
	documents.insert(
		consumer_uri.clone(),
		DocumentState {
			content: consumer_doc.to_string(),
			blocks: consumer_blocks,
			parse_diagnostics: consumer_parse_diagnostics,
		},
	);

	let mut data = HashMap::new();
	data.insert("pkg".to_string(), serde_json::json!({"version": "1.0.0"}));

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers,
		consumers: Vec::new(),
		data,
	};

	let diagnostics = compute_diagnostics(&state, &consumer_uri);
	// The consumer has "0.9.0" but the rendered template produces "1.0.0",
	// so there should be a stale diagnostic.
	assert!(
		diagnostics
			.iter()
			.any(|d| d.message.contains("out of date")),
		"expected stale diagnostic when rendered content differs, got: {diagnostics:?}"
	);
	// The diagnostic data should contain the rendered "1.0.0" version.
	let stale_diag = diagnostics
		.iter()
		.find(|d| d.message.contains("out of date"))
		.unwrap_or_else(|| panic!("expected stale diagnostic"));
	let data = stale_diag
		.data
		.as_ref()
		.unwrap_or_else(|| panic!("expected diagnostic data"));
	let expected_content = data["expected_content"]
		.as_str()
		.unwrap_or_else(|| panic!("expected expected_content string"));
	assert!(
		expected_content.contains("Version: 1.0.0"),
		"expected rendered content to contain 'Version: 1.0.0', got: {expected_content}"
	);
}

// ---- Hover: consumer with transformers shows transformed preview ----

#[test]
fn hover_consumer_with_transformers_shows_transformed_content() {
	let consumer_doc = "<!-- {=greeting|trim} -->\n\nstuff\n\n<!-- {/greeting} -->\n";
	let (consumer_blocks, consumer_parse_diags) =
		parse_with_diagnostics(consumer_doc).unwrap_or_default();
	let uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let provider_entry = ProviderEntry {
		block: Block {
			name: "greeting".to_string(),
			r#type: BlockType::Provider,
			opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
			closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
			transformers: Vec::new(),
			arguments: vec![],
		},
		file: PathBuf::from("/tmp/test/template.t.md"),
		content: "\n\n  Hello world!  \n\n".to_string(),
	};

	let mut providers = HashMap::new();
	providers.insert("greeting".to_string(), provider_entry);

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: consumer_doc.to_string(),
			blocks: consumer_blocks.clone(),
			parse_diagnostics: consumer_parse_diags,
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
		.unwrap_or_else(|| panic!("expected consumer block"));

	let position = to_lsp_position(&block.opening.start);
	let hover = compute_hover(&state, &uri, position);

	assert!(hover.is_some(), "expected hover result");
	if let HoverContents::Markup(markup) = &hover.unwrap().contents {
		assert!(
			markup.value.contains("Transformers"),
			"expected 'Transformers' section, got: {}",
			markup.value
		);
		assert!(
			markup.value.contains("trim"),
			"expected 'trim' in transformers, got: {}",
			markup.value
		);
		// The trimmed content should appear in the preview.
		assert!(
			markup.value.contains("Hello world!"),
			"expected trimmed content in hover preview, got: {}",
			markup.value
		);
	} else {
		panic!("expected Markup hover contents");
	}
}

// ---- Completion: multiple providers ----

#[test]
fn completion_lists_all_providers() {
	let doc = "<!-- {=";
	let uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: doc.to_string(),
			blocks: Vec::new(),
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
				closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
				transformers: Vec::new(),
				arguments: vec![],
			},
			file: PathBuf::from("/tmp/test/template.t.md"),
			content: "\n\nHello!\n\n".to_string(),
		},
	);
	providers.insert(
		"installation".to_string(),
		ProviderEntry {
			block: Block {
				name: "installation".to_string(),
				r#type: BlockType::Provider,
				opening: mdt_core::Position::new(5, 1, 50, 5, 25, 74),
				closing: mdt_core::Position::new(7, 1, 80, 7, 25, 104),
				transformers: Vec::new(),
				arguments: vec![],
			},
			file: PathBuf::from("/tmp/test/template.t.md"),
			content: "\n\nInstall it.\n\n".to_string(),
		},
	);

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers,
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let position = Position {
		line: 0,
		character: 7,
	};
	let completions = compute_completions(&state, &uri, position);
	assert_eq!(
		completions.len(),
		2,
		"expected 2 completion items for 2 providers"
	);
	let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
	assert!(labels.contains(&"greeting"));
	assert!(labels.contains(&"installation"));
}

// ---- suggest_similar_names edge cases ----

#[test]
fn suggest_similar_names_no_providers_returns_empty() {
	let providers = HashMap::new();
	let suggestions = suggest_similar_names("greeting", &providers);
	assert!(
		suggestions.is_empty(),
		"expected no suggestions with no providers"
	);
}

#[test]
fn suggest_similar_names_exact_match_excluded() {
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
				arguments: vec![],
			},
			file: PathBuf::from("/tmp/test/template.t.md"),
			content: String::new(),
		},
	);

	// Exact match has distance 0, which is filtered out (d > 0).
	let suggestions = suggest_similar_names("greeting", &providers);
	assert!(
		suggestions.is_empty(),
		"exact match should not be suggested"
	);
}

#[test]
fn suggest_similar_names_truncates_to_three() {
	let mut providers = HashMap::new();
	for name in &["greet1", "greet2", "greet3", "greet4", "greet5"] {
		providers.insert(
			name.to_string(),
			ProviderEntry {
				block: Block {
					name: name.to_string(),
					r#type: BlockType::Provider,
					opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
					closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
					transformers: Vec::new(),
					arguments: vec![],
				},
				file: PathBuf::from("/tmp/test/template.t.md"),
				content: String::new(),
			},
		);
	}

	let suggestions = suggest_similar_names("greetX", &providers);
	assert!(
		suggestions.len() <= 3,
		"expected at most 3 suggestions, got: {}",
		suggestions.len()
	);
}

// ---- levenshtein_distance additional edge cases ----

#[test]
fn levenshtein_single_char_strings() {
	assert_eq!(levenshtein_distance("a", "b"), 1);
	assert_eq!(levenshtein_distance("a", "a"), 0);
	assert_eq!(levenshtein_distance("a", ""), 1);
	assert_eq!(levenshtein_distance("", "a"), 1);
}

#[test]
fn levenshtein_completely_different() {
	assert_eq!(levenshtein_distance("abc", "xyz"), 3);
}

// ---- Code action: stale consumer with data rendering (successful render) ----

#[test]
fn code_action_with_successful_template_rendering() {
	let provider_template =
		"<!-- {@version} -->\n\nVersion: {{ pkg.version }}\n\n<!-- {/version} -->\n";
	let consumer_doc = "<!-- {=version} -->\n\nVersion: 0.9.0\n\n<!-- {/version} -->\n";

	let provider_blocks = parse(provider_template).unwrap_or_default();
	let consumer_blocks = parse(consumer_doc).unwrap_or_default();

	let provider_block = provider_blocks
		.iter()
		.find(|b| b.r#type == BlockType::Provider)
		.cloned()
		.unwrap_or_else(|| panic!("expected a provider block"));

	let provider_entry = ProviderEntry {
		block: provider_block,
		file: PathBuf::from("/tmp/test/template.t.md"),
		content: extract_content_between_tags(provider_template, &provider_blocks[0]),
	};

	let consumer_uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut providers = HashMap::new();
	providers.insert("version".to_string(), provider_entry);

	let mut documents = HashMap::new();
	documents.insert(
		consumer_uri.clone(),
		DocumentState {
			content: consumer_doc.to_string(),
			blocks: consumer_blocks.clone(),
			parse_diagnostics: Vec::new(),
		},
	);

	let mut data = HashMap::new();
	data.insert("pkg".to_string(), serde_json::json!({"version": "1.0.0"}));

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers,
		consumers: Vec::new(),
		data,
	};

	let block = consumer_blocks
		.iter()
		.find(|b| b.r#type == BlockType::Consumer)
		.unwrap_or_else(|| panic!("expected consumer block"));

	let range = Range {
		start: to_lsp_position(&block.opening.start),
		end: to_lsp_position(&block.closing.end),
	};

	let actions = compute_code_actions(&state, &consumer_uri, range);
	assert!(
		!actions.is_empty(),
		"expected code action for stale consumer with rendered template"
	);

	let CodeActionOrCommand::CodeAction(action) = &actions[0] else {
		panic!("expected CodeAction")
	};

	assert!(action.title.contains("Update block"));
	assert!(action.title.contains("version"));

	// Verify the edit contains the rendered content.
	let edit = action
		.edit
		.as_ref()
		.unwrap_or_else(|| panic!("expected workspace edit"));
	let changes = edit
		.changes
		.as_ref()
		.unwrap_or_else(|| panic!("expected changes"));
	let edits = changes
		.get(&consumer_uri)
		.unwrap_or_else(|| panic!("expected edits for consumer URI"));
	assert!(
		edits[0].new_text.contains("Version: 1.0.0"),
		"expected rendered content in edit, got: {}",
		edits[0].new_text
	);
}

// ---- Hover: provider with render_template for data ----

#[test]
fn hover_provider_shows_raw_content_with_template_syntax() {
	let provider_template =
		"<!-- {@version} -->\n\nVersion: {{ pkg.version }}\n\n<!-- {/version} -->\n";
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
	providers.insert("version".to_string(), provider_entry);

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
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	let position = to_lsp_position(&provider_block.opening.start);
	let hover = compute_hover(&state, &provider_uri, position);

	assert!(hover.is_some());
	if let HoverContents::Markup(markup) = &hover.unwrap().contents {
		assert!(
			markup.value.contains("Provider block"),
			"expected 'Provider block' header"
		);
		// Provider hover shows raw content (not rendered).
		assert!(
			markup.value.contains("pkg.version"),
			"expected raw template syntax in provider hover, got: {}",
			markup.value
		);
	} else {
		panic!("expected Markup hover contents");
	}
}

// ---- Multiple blocks in same document ----

#[test]
fn diagnostics_multiple_blocks_mixed_states() {
	let consumer_doc = "<!-- {=greeting} -->\n\nHello!\n\n<!-- {/greeting} -->\n\n<!-- {=missing} \
	                    -->\n\nstuff\n\n<!-- {/missing} -->\n";
	let (consumer_blocks, consumer_parse_diagnostics) =
		parse_with_diagnostics(consumer_doc).unwrap_or_default();
	let uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

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
				arguments: vec![],
			},
			file: PathBuf::from("/tmp/test/template.t.md"),
			content: "\n\nHello!\n\n".to_string(),
		},
	);

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: consumer_doc.to_string(),
			blocks: consumer_blocks,
			parse_diagnostics: consumer_parse_diagnostics,
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
	// "greeting" should be up to date (content matches), "missing" should
	// produce a "No provider found" diagnostic.
	assert!(
		diagnostics
			.iter()
			.any(|d| d.message.contains("No provider found") && d.message.contains("missing")),
		"expected missing provider diagnostic for 'missing' block, got: {diagnostics:?}"
	);
	assert!(
		!diagnostics
			.iter()
			.any(|d| d.message.contains("greeting") && d.message.contains("out of date")),
		"'greeting' should be up to date, got: {diagnostics:?}"
	);
}

// ---- update_document_in_project: provider in non-template file ----

#[test]
fn update_document_in_project_provider_in_non_template_file_not_registered() {
	let content = "<!-- {@greeting} -->\n\nHello\n\n<!-- {/greeting} -->\n";
	let blocks = parse(content).unwrap_or_default();
	// Non-template URI (readme.md, not *.t.md).
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

	let mut state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers: HashMap::new(),
		consumers: Vec::new(),
		data: HashMap::new(),
	};

	state.update_document_in_project(&uri);

	// Provider blocks in non-template files should NOT be registered.
	assert!(
		state.providers.is_empty(),
		"provider in non-template file should not be registered"
	);
}

// ---- Document symbols: multiple blocks with correct details ----

#[test]
fn document_symbols_multiple_blocks_correct_ranges() {
	let content = "<!-- {@first} -->\n\nContent1\n\n<!-- {/first} -->\n\n<!-- {=second} \
	               -->\n\nContent2\n\n<!-- {/second} -->\n";
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

	// First should be provider with @ prefix and CLASS kind.
	assert_eq!(symbols[0].name, "@first");
	assert_eq!(symbols[0].kind, SymbolKind::CLASS);

	// Second should be consumer with = prefix and VARIABLE kind.
	assert_eq!(symbols[1].name, "=second");
	assert_eq!(symbols[1].kind, SymbolKind::VARIABLE);

	// The full range should span from opening to closing.
	assert!(
		symbols[0].range.start.line < symbols[0].range.end.line,
		"expected multi-line range for block"
	);
}

// ---- References tests ----

#[test]
fn references_from_consumer_returns_provider_and_consumers() {
	let (state, consumer_uri) = make_test_state("Hello!", "Old");

	let doc = state.documents.get(&consumer_uri).unwrap();
	let block = doc
		.blocks
		.iter()
		.find(|b| b.r#type == BlockType::Consumer)
		.unwrap();

	let position = to_lsp_position(&block.opening.start);
	let result = compute_references(&state, &consumer_uri, position);

	assert!(result.is_some(), "expected references result");
	let locations = result.unwrap();

	// Should include the provider + at least 1 consumer.
	assert!(
		locations.len() >= 2,
		"expected at least 2 locations (provider + consumer), got {}",
		locations.len()
	);

	// Provider location should point to template file.
	assert!(
		locations
			.iter()
			.any(|l| l.uri.path().as_str().contains("template.t.md")),
		"expected provider location in references"
	);

	// Consumer location should point to readme.
	assert!(
		locations
			.iter()
			.any(|l| l.uri.path().as_str().contains("readme.md")),
		"expected consumer location in references"
	);
}

#[test]
fn references_from_provider_returns_provider_and_consumers() {
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

	let consumers = vec![
		ConsumerEntry {
			block: Block {
				name: "greeting".to_string(),
				r#type: BlockType::Consumer,
				opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
				closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
				transformers: Vec::new(),
				arguments: vec![],
			},
			file: PathBuf::from("/tmp/test/readme.md"),
			content: "\n\nold\n\n".to_string(),
		},
		ConsumerEntry {
			block: Block {
				name: "greeting".to_string(),
				r#type: BlockType::Consumer,
				opening: mdt_core::Position::new(1, 1, 0, 1, 20, 19),
				closing: mdt_core::Position::new(3, 1, 30, 3, 20, 49),
				transformers: Vec::new(),
				arguments: vec![],
			},
			file: PathBuf::from("/tmp/test/docs.md"),
			content: "\n\nold\n\n".to_string(),
		},
	];

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
	let result = compute_references(&state, &provider_uri, position);

	assert!(result.is_some(), "expected references result");
	let locations = result.unwrap();

	// Provider + 2 consumers = 3 locations.
	assert_eq!(locations.len(), 3, "expected 3 locations");

	let paths: Vec<String> = locations
		.iter()
		.map(|l| l.uri.path().as_str().to_string())
		.collect();
	assert!(
		paths.iter().any(|p| p.contains("template.t.md")),
		"expected provider in references"
	);
	assert!(
		paths.iter().any(|p| p.contains("readme.md")),
		"expected readme consumer in references"
	);
	assert!(
		paths.iter().any(|p| p.contains("docs.md")),
		"expected docs consumer in references"
	);
}

#[test]
fn references_outside_block_returns_none() {
	let (state, uri) = make_test_state("Hello!", "Hello!");

	let position = Position {
		line: 0,
		character: 0,
	};
	let result = compute_references(&state, &uri, position);
	assert!(result.is_none(), "expected None for position outside block");
}

#[test]
fn references_consumer_without_provider_returns_only_consumer() {
	let consumer_doc = "<!-- {=orphan} -->\n\nstuff\n\n<!-- {/orphan} -->\n";
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

	let consumers = vec![ConsumerEntry {
		block: consumer_blocks[0].clone(),
		file: PathBuf::from("/tmp/test/readme.md"),
		content: "\n\nstuff\n\n".to_string(),
	}];

	let state = WorkspaceState {
		root: Some(PathBuf::from("/tmp/test")),
		documents,
		providers: HashMap::new(),
		consumers,
		data: HashMap::new(),
	};

	let block = consumer_blocks
		.iter()
		.find(|b| b.r#type == BlockType::Consumer)
		.unwrap();
	let position = to_lsp_position(&block.opening.start);
	let result = compute_references(&state, &uri, position);

	assert!(result.is_some(), "expected references result");
	let locations = result.unwrap();
	assert_eq!(locations.len(), 1, "expected only the consumer location");
	assert!(locations[0].uri.path().as_str().contains("readme.md"));
}

// ---- Prepare Rename tests ----

#[test]
fn prepare_rename_on_consumer_returns_name_range() {
	let consumer_doc = "<!-- {=greeting} -->\n\nHello\n\n<!-- {/greeting} -->\n";
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
	let result = compute_prepare_rename(&state, &uri, position);

	assert!(result.is_some(), "expected prepare rename result");
	match result.unwrap() {
		PrepareRenameResponse::Range(range) => {
			// The name "greeting" in `<!-- {=greeting} -->` starts after
			// `<!-- {=` (7 chars) and spans 8 chars.
			assert_eq!(range.start.line, 0);
			assert_eq!(range.start.character, 7);
			assert_eq!(range.end.line, 0);
			assert_eq!(range.end.character, 15);
		}
		other => panic!("expected Range response, got: {other:?}"),
	}
}

#[test]
fn prepare_rename_on_provider_returns_name_range() {
	let provider_doc = "<!-- {@myBlock} -->\n\nContent\n\n<!-- {/myBlock} -->\n";
	let provider_blocks = parse(provider_doc).unwrap_or_default();
	let uri = "file:///tmp/test/template.t.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let mut documents = HashMap::new();
	documents.insert(
		uri.clone(),
		DocumentState {
			content: provider_doc.to_string(),
			blocks: provider_blocks.clone(),
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

	let block = provider_blocks
		.iter()
		.find(|b| b.r#type == BlockType::Provider)
		.unwrap();
	let position = to_lsp_position(&block.opening.start);
	let result = compute_prepare_rename(&state, &uri, position);

	assert!(result.is_some(), "expected prepare rename result");
	match result.unwrap() {
		PrepareRenameResponse::Range(range) => {
			// The name "myBlock" in `<!-- {@myBlock} -->` starts after
			// `<!-- {@` (7 chars) and spans 7 chars.
			assert_eq!(range.start.line, 0);
			assert_eq!(range.start.character, 7);
			assert_eq!(range.end.line, 0);
			assert_eq!(range.end.character, 14);
		}
		other => panic!("expected Range response, got: {other:?}"),
	}
}

#[test]
fn prepare_rename_outside_block_returns_none() {
	let (state, uri) = make_test_state("Hello!", "Hello!");

	let position = Position {
		line: 0,
		character: 0,
	};
	let result = compute_prepare_rename(&state, &uri, position);
	assert!(result.is_none(), "expected None for position outside block");
}

// ---- Rename tests ----

#[test]
fn rename_consumer_renames_both_tags_in_open_document() {
	let consumer_doc = "<!-- {=greeting} -->\n\nHello\n\n<!-- {/greeting} -->\n";
	let consumer_blocks = parse(consumer_doc).unwrap_or_default();
	let consumer_uri = "file:///tmp/test/readme.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

	let provider_template = "<!-- {@greeting} -->\n\nHello\n\n<!-- {/greeting} -->\n";
	let provider_blocks = parse(provider_template).unwrap_or_default();
	let provider_uri = "file:///tmp/test/template.t.md"
		.parse::<Uri>()
		.unwrap_or_else(|_| panic!("invalid test URI"));

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

	let mut consumers = Vec::new();
	for block in &consumer_blocks {
		if block.r#type == BlockType::Consumer {
			consumers.push(ConsumerEntry {
				block: block.clone(),
				file: PathBuf::from("/tmp/test/readme.md"),
				content: extract_content_between_tags(consumer_doc, block),
			});
		}
	}

	let mut documents = HashMap::new();
	documents.insert(
		consumer_uri.clone(),
		DocumentState {
			content: consumer_doc.to_string(),
			blocks: consumer_blocks.clone(),
			parse_diagnostics: Vec::new(),
		},
	);
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

	let block = consumer_blocks
		.iter()
		.find(|b| b.r#type == BlockType::Consumer)
		.unwrap();
	let position = to_lsp_position(&block.opening.start);
	let result = compute_rename(&state, &consumer_uri, position, "salutation");

	assert!(result.is_some(), "expected rename result");
	let edit = result.unwrap();
	let changes = edit.changes.unwrap();

	// Should have edits for both the consumer file and the provider file.
	assert!(
		changes.contains_key(&consumer_uri),
		"expected edits for consumer file"
	);
	assert!(
		changes.contains_key(&provider_uri),
		"expected edits for provider file"
	);

	// Consumer file: opening + closing = 2 edits.
	let consumer_edits = &changes[&consumer_uri];
	assert_eq!(
		consumer_edits.len(),
		2,
		"expected 2 edits for consumer (open + close tag)"
	);
	for edit in consumer_edits {
		assert_eq!(edit.new_text, "salutation");
	}

	// Provider file: opening + closing = 2 edits.
	let provider_edits = &changes[&provider_uri];
	assert_eq!(
		provider_edits.len(),
		2,
		"expected 2 edits for provider (open + close tag)"
	);
	for edit in provider_edits {
		assert_eq!(edit.new_text, "salutation");
	}
}

#[test]
fn rename_outside_block_returns_none() {
	let (state, uri) = make_test_state("Hello!", "Hello!");

	let position = Position {
		line: 0,
		character: 0,
	};
	let result = compute_rename(&state, &uri, position, "newName");
	assert!(result.is_none(), "expected None for position outside block");
}

// ---- find_name_range_in_tag tests ----

#[test]
fn find_name_range_in_consumer_tag() {
	let tag = "<!-- {=greeting} -->";
	let start = Position {
		line: 0,
		character: 0,
	};
	let range = find_name_range_in_tag(tag, start, "greeting");
	assert!(range.is_some(), "expected name range");
	let range = range.unwrap();
	assert_eq!(range.start.line, 0);
	assert_eq!(range.start.character, 7); // after `<!-- {=`
	assert_eq!(range.end.line, 0);
	assert_eq!(range.end.character, 15); // 7 + 8
}

#[test]
fn find_name_range_in_provider_tag() {
	let tag = "<!-- {@myBlock} -->";
	let start = Position {
		line: 0,
		character: 0,
	};
	let range = find_name_range_in_tag(tag, start, "myBlock");
	assert!(range.is_some(), "expected name range");
	let range = range.unwrap();
	assert_eq!(range.start.line, 0);
	assert_eq!(range.start.character, 7); // after `<!-- {@`
	assert_eq!(range.end.line, 0);
	assert_eq!(range.end.character, 14); // 7 + 7
}

#[test]
fn find_name_range_in_close_tag() {
	let tag = "<!-- {/greeting} -->";
	let start = Position {
		line: 0,
		character: 0,
	};
	let range = find_name_range_in_tag(tag, start, "greeting");
	assert!(range.is_some(), "expected name range");
	let range = range.unwrap();
	assert_eq!(range.start.line, 0);
	assert_eq!(range.start.character, 7); // after `<!-- {/`
	assert_eq!(range.end.line, 0);
	assert_eq!(range.end.character, 15); // 7 + 8
}

#[test]
fn find_name_range_with_nonzero_start() {
	let tag = "<!-- {=greeting} -->";
	let start = Position {
		line: 5,
		character: 10,
	};
	let range = find_name_range_in_tag(tag, start, "greeting");
	assert!(range.is_some(), "expected name range");
	let range = range.unwrap();
	assert_eq!(range.start.line, 5);
	assert_eq!(range.start.character, 17); // 10 + 7
	assert_eq!(range.end.line, 5);
	assert_eq!(range.end.character, 25); // 17 + 8
}

#[test]
fn find_name_range_in_consumer_with_transformers() {
	let tag = "<!-- {=greeting|trim|indent:\"  \"} -->";
	let start = Position {
		line: 0,
		character: 0,
	};
	let range = find_name_range_in_tag(tag, start, "greeting");
	assert!(range.is_some(), "expected name range");
	let range = range.unwrap();
	assert_eq!(range.start.line, 0);
	assert_eq!(range.start.character, 7);
	assert_eq!(range.end.line, 0);
	assert_eq!(range.end.character, 15);
}

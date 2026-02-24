use std::path::Path;
use std::path::PathBuf;

use rmcp::handler::server::wrapper::Parameters;

use super::*;

// ---------------------------------------------------------------------------
// Helper: extract text from the first Content item in a CallToolResult
// ---------------------------------------------------------------------------

fn extract_text(result: &CallToolResult) -> &str {
	result.content[0]
		.raw
		.as_text()
		.unwrap_or_else(|| panic!("expected text content"))
		.text
		.as_str()
}

// ---------------------------------------------------------------------------
// Helper: create a minimal mdt project in a temp directory
// ---------------------------------------------------------------------------

/// Create a project with a provider named `greeting` and a **stale** consumer.
fn create_stale_project(root: &Path) {
	let template = "\
<!-- {@greeting} -->

Hello from mdt!

<!-- {/greeting} -->
";
	let readme = "\
<!-- {=greeting} -->

Old stale content.

<!-- {/greeting} -->
";
	std::fs::write(root.join("template.t.md"), template)
		.unwrap_or_else(|e| panic!("write template: {e}"));
	std::fs::write(root.join("readme.md"), readme).unwrap_or_else(|e| panic!("write readme: {e}"));
}

/// Create a project with a provider named `greeting` and an **up-to-date**
/// consumer.
fn create_synced_project(root: &Path) {
	let template = "\
<!-- {@greeting} -->

Hello from mdt!

<!-- {/greeting} -->
";
	let readme = "\
<!-- {=greeting} -->

Hello from mdt!

<!-- {/greeting} -->
";
	std::fs::write(root.join("template.t.md"), template)
		.unwrap_or_else(|e| panic!("write template: {e}"));
	std::fs::write(root.join("readme.md"), readme).unwrap_or_else(|e| panic!("write readme: {e}"));
}

/// Create a project with multiple provider blocks.
fn create_multi_block_project(root: &Path) {
	let template = "\
<!-- {@greeting} -->

Hello from mdt!

<!-- {/greeting} -->

<!-- {@farewell} -->

Goodbye from mdt!

<!-- {/farewell} -->
";
	let readme = "\
<!-- {=greeting} -->

Hello from mdt!

<!-- {/greeting} -->

<!-- {=farewell} -->

Old farewell content.

<!-- {/farewell} -->
";
	std::fs::write(root.join("template.t.md"), template)
		.unwrap_or_else(|e| panic!("write template: {e}"));
	std::fs::write(root.join("readme.md"), readme).unwrap_or_else(|e| panic!("write readme: {e}"));
}

// ===========================================================================
// resolve_root
// ===========================================================================

#[test]
fn resolve_root_with_some_path() {
	let result = resolve_root(Some("/tmp/test_project"));
	assert_eq!(result, PathBuf::from("/tmp/test_project"));
}

#[test]
fn resolve_root_with_none_falls_back_to_cwd() {
	let result = resolve_root(None);
	let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
	assert_eq!(result, cwd);
}

// ===========================================================================
// make_relative
// ===========================================================================

#[test]
fn make_relative_inside_root() {
	let root = Path::new("/home/user/project");
	let full = Path::new("/home/user/project/src/main.rs");
	assert_eq!(make_relative(full, root), "src/main.rs");
}

#[test]
fn make_relative_outside_root_returns_full_path() {
	let root = Path::new("/home/user/project");
	let full = Path::new("/other/path/file.md");
	assert_eq!(make_relative(full, root), "/other/path/file.md");
}

#[test]
fn make_relative_same_as_root() {
	let root = Path::new("/home/user/project");
	let full = Path::new("/home/user/project");
	// strip_prefix on equal paths gives ""
	assert_eq!(make_relative(full, root), "");
}

// ===========================================================================
// scan_ctx
// ===========================================================================

#[test]
fn scan_ctx_on_empty_dir_succeeds() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let ctx = scan_ctx(tmp.path());
	assert!(ctx.is_ok(), "scan_ctx should succeed on empty directory");
}

#[test]
fn scan_ctx_on_project_finds_providers() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());
	let ctx = scan_ctx(tmp.path()).unwrap_or_else(|e| panic!("scan_ctx: {e}"));
	assert!(
		ctx.project.providers.contains_key("greeting"),
		"should find the greeting provider"
	);
}

// ===========================================================================
// MdtMcpServer::new / Default
// ===========================================================================

#[test]
fn server_new_creates_instance() {
	let _server = MdtMcpServer::new();
}

#[test]
fn server_default_creates_instance() {
	let _server = MdtMcpServer::default();
}

// ===========================================================================
// init
// ===========================================================================

#[tokio::test]
async fn init_creates_template_file() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let server = MdtMcpServer::new();

	let result = server
		.init(Parameters(InitParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
		}))
		.await
		.unwrap_or_else(|e| panic!("init: {e:?}"));

	let text = extract_text(&result);
	assert!(
		text.contains("Created template file"),
		"expected creation message, got: {text}"
	);
	assert!(
		tmp.path().join("template.t.md").exists(),
		"template.t.md should exist"
	);
}

#[tokio::test]
async fn init_reports_existing_template() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("template.t.md"), "existing content")
		.unwrap_or_else(|e| panic!("write: {e}"));

	let server = MdtMcpServer::new();
	let result = server
		.init(Parameters(InitParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
		}))
		.await
		.unwrap_or_else(|e| panic!("init: {e:?}"));

	let text = extract_text(&result);
	assert!(
		text.contains("already exists"),
		"expected 'already exists' message, got: {text}"
	);
}

// ===========================================================================
// check
// ===========================================================================

#[tokio::test]
async fn check_on_empty_project_reports_up_to_date() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let server = MdtMcpServer::new();

	let result = server
		.check(Parameters(PathParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
		}))
		.await
		.unwrap_or_else(|e| panic!("check: {e:?}"));

	let text = extract_text(&result);
	assert!(
		text.contains("up to date"),
		"expected up-to-date message, got: {text}"
	);
}

#[tokio::test]
async fn check_on_synced_project_reports_up_to_date() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_synced_project(tmp.path());

	let server = MdtMcpServer::new();
	let result = server
		.check(Parameters(PathParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
		}))
		.await
		.unwrap_or_else(|e| panic!("check: {e:?}"));

	let text = extract_text(&result);
	assert!(
		text.contains("up to date"),
		"expected up-to-date message, got: {text}"
	);
}

#[tokio::test]
async fn check_on_stale_project_reports_stale_blocks() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());

	let server = MdtMcpServer::new();
	let result = server
		.check(Parameters(PathParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
		}))
		.await
		.unwrap_or_else(|e| panic!("check: {e:?}"));

	let text = extract_text(&result);
	assert!(
		text.contains("stale"),
		"expected stale message, got: {text}"
	);
	assert!(
		text.contains("greeting"),
		"expected block name in message, got: {text}"
	);
}

// ===========================================================================
// update
// ===========================================================================

#[tokio::test]
async fn update_on_up_to_date_project_reports_no_changes() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_synced_project(tmp.path());

	let server = MdtMcpServer::new();
	let result = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: false,
		}))
		.await
		.unwrap_or_else(|e| panic!("update: {e:?}"));

	let text = extract_text(&result);
	assert!(
		text.contains("already up to date"),
		"expected no-changes message, got: {text}"
	);
}

#[tokio::test]
async fn update_on_stale_project_applies_changes() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());

	let server = MdtMcpServer::new();
	let result = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: false,
		}))
		.await
		.unwrap_or_else(|e| panic!("update: {e:?}"));

	let text = extract_text(&result);
	assert!(
		text.contains("Updated"),
		"expected update confirmation, got: {text}"
	);

	// Verify the file was actually written
	let readme_content = std::fs::read_to_string(tmp.path().join("readme.md"))
		.unwrap_or_else(|e| panic!("read readme: {e}"));
	assert!(
		readme_content.contains("Hello from mdt!"),
		"consumer should now have provider content"
	);
	assert!(
		!readme_content.contains("Old stale content"),
		"old stale content should be replaced"
	);
}

#[tokio::test]
async fn update_dry_run_does_not_write() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());

	let server = MdtMcpServer::new();
	let result = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: true,
		}))
		.await
		.unwrap_or_else(|e| panic!("update: {e:?}"));

	let text = extract_text(&result);
	assert!(
		text.contains("Dry run"),
		"expected dry-run message, got: {text}"
	);

	// Verify the file was NOT modified
	let readme_content = std::fs::read_to_string(tmp.path().join("readme.md"))
		.unwrap_or_else(|e| panic!("read readme: {e}"));
	assert!(
		readme_content.contains("Old stale content"),
		"dry run should not modify files"
	);
}

#[tokio::test]
async fn update_dry_run_lists_affected_files() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());

	let server = MdtMcpServer::new();
	let result = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: true,
		}))
		.await
		.unwrap_or_else(|e| panic!("update: {e:?}"));

	let text = extract_text(&result);
	assert!(
		text.contains("readme.md"),
		"dry run should list the affected file, got: {text}"
	);
}

// ===========================================================================
// list
// ===========================================================================

#[tokio::test]
async fn list_on_empty_project_returns_empty() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let server = MdtMcpServer::new();

	let result = server
		.list(Parameters(PathParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
		}))
		.await
		.unwrap_or_else(|e| panic!("list: {e:?}"));

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));
	assert_eq!(json["providers"], serde_json::json!([]));
	assert_eq!(json["consumers"], serde_json::json!([]));
}

#[tokio::test]
async fn list_on_project_with_blocks_returns_provider_and_consumer() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());

	let server = MdtMcpServer::new();
	let result = server
		.list(Parameters(PathParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
		}))
		.await
		.unwrap_or_else(|e| panic!("list: {e:?}"));

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	let providers = json["providers"]
		.as_array()
		.unwrap_or_else(|| panic!("providers should be array"));
	assert_eq!(providers.len(), 1);
	assert_eq!(providers[0]["name"], "greeting");

	let consumers = json["consumers"]
		.as_array()
		.unwrap_or_else(|| panic!("consumers should be array"));
	assert_eq!(consumers.len(), 1);
	assert_eq!(consumers[0]["name"], "greeting");
	assert_eq!(consumers[0]["is_stale"], true);
}

#[tokio::test]
async fn list_shows_synced_consumer_as_not_stale() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_synced_project(tmp.path());

	let server = MdtMcpServer::new();
	let result = server
		.list(Parameters(PathParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
		}))
		.await
		.unwrap_or_else(|e| panic!("list: {e:?}"));

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	let consumers = json["consumers"]
		.as_array()
		.unwrap_or_else(|| panic!("consumers should be array"));
	assert_eq!(consumers.len(), 1);
	assert_eq!(consumers[0]["is_stale"], false);
}

#[tokio::test]
async fn list_with_multiple_blocks_returns_sorted_providers() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_multi_block_project(tmp.path());

	let server = MdtMcpServer::new();
	let result = server
		.list(Parameters(PathParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
		}))
		.await
		.unwrap_or_else(|e| panic!("list: {e:?}"));

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	let providers = json["providers"]
		.as_array()
		.unwrap_or_else(|| panic!("providers should be array"));
	assert_eq!(providers.len(), 2);
	// Providers should be sorted alphabetically
	assert_eq!(providers[0]["name"], "farewell");
	assert_eq!(providers[1]["name"], "greeting");
}

// ===========================================================================
// get_block
// ===========================================================================

#[tokio::test]
async fn get_block_for_provider_returns_provider_info() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());

	let server = MdtMcpServer::new();
	let result = server
		.get_block(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "greeting".to_string(),
		}))
		.await
		.unwrap_or_else(|e| panic!("get_block: {e:?}"));

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	assert_eq!(json["type"], "provider");
	assert_eq!(json["name"], "greeting");
	assert_eq!(json["consumer_count"], 1);

	let rendered = json["rendered_content"]
		.as_str()
		.unwrap_or_else(|| panic!("rendered_content should be string"));
	assert!(
		rendered.contains("Hello from mdt!"),
		"rendered content should contain provider text"
	);
}

#[tokio::test]
async fn get_block_for_provider_lists_consumer_files() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());

	let server = MdtMcpServer::new();
	let result = server
		.get_block(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "greeting".to_string(),
		}))
		.await
		.unwrap_or_else(|e| panic!("get_block: {e:?}"));

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	let consumer_files = json["consumer_files"]
		.as_array()
		.unwrap_or_else(|| panic!("consumer_files should be array"));
	assert_eq!(consumer_files.len(), 1);
	assert!(
		consumer_files[0]
			.as_str()
			.unwrap_or_default()
			.contains("readme.md"),
		"consumer file should be readme.md"
	);
}

#[tokio::test]
async fn get_block_for_consumer_only_returns_consumer_entries() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	// Create a project where a consumer references a block that has no provider
	let readme = "\
<!-- {=orphan} -->

Some orphan content.

<!-- {/orphan} -->
";
	std::fs::write(tmp.path().join("readme.md"), readme).unwrap_or_else(|e| panic!("write: {e}"));
	// Need a template file for mdt to scan (even if empty of providers for this
	// block)
	std::fs::write(tmp.path().join("template.t.md"), "").unwrap_or_else(|e| panic!("write: {e}"));

	let server = MdtMcpServer::new();
	let result = server
		.get_block(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "orphan".to_string(),
		}))
		.await
		.unwrap_or_else(|e| panic!("get_block: {e:?}"));

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	// Should be an array of consumer entries
	let entries = json
		.as_array()
		.unwrap_or_else(|| panic!("expected array of consumer entries"));
	assert_eq!(entries.len(), 1);
	assert_eq!(entries[0]["type"], "consumer");
	assert_eq!(entries[0]["name"], "orphan");
}

#[tokio::test]
async fn get_block_for_nonexistent_returns_error() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	let server = MdtMcpServer::new();
	let result = server
		.get_block(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "nonexistent".to_string(),
		}))
		.await
		.unwrap_or_else(|e| panic!("get_block: {e:?}"));

	assert_eq!(
		result.is_error,
		Some(true),
		"result should be marked as error"
	);

	let text = extract_text(&result);
	assert!(
		text.contains("No block named"),
		"expected 'No block named' message, got: {text}"
	);
}

// ===========================================================================
// preview
// ===========================================================================

#[tokio::test]
async fn preview_for_existing_provider_returns_rendered_content() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());

	let server = MdtMcpServer::new();
	let result = server
		.preview(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "greeting".to_string(),
		}))
		.await
		.unwrap_or_else(|e| panic!("preview: {e:?}"));

	let text = extract_text(&result);
	assert!(
		text.contains("Provider `greeting`"),
		"expected provider heading, got: {text}"
	);
	assert!(
		text.contains("Hello from mdt!"),
		"expected rendered content, got: {text}"
	);
}

#[tokio::test]
async fn preview_shows_consumer_info() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());

	let server = MdtMcpServer::new();
	let result = server
		.preview(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "greeting".to_string(),
		}))
		.await
		.unwrap_or_else(|e| panic!("preview: {e:?}"));

	let text = extract_text(&result);
	assert!(
		text.contains("consumer(s)"),
		"expected consumer section, got: {text}"
	);
	assert!(
		text.contains("readme.md"),
		"expected consumer file listed, got: {text}"
	);
}

#[tokio::test]
async fn preview_for_nonexistent_provider_returns_error() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	let server = MdtMcpServer::new();
	let result = server
		.preview(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "nonexistent".to_string(),
		}))
		.await
		.unwrap_or_else(|e| panic!("preview: {e:?}"));

	assert_eq!(
		result.is_error,
		Some(true),
		"result should be marked as error"
	);

	let text = extract_text(&result);
	assert!(
		text.contains("No provider named"),
		"expected 'No provider named' message, got: {text}"
	);
}

#[tokio::test]
async fn preview_provider_without_consumers_omits_consumer_section() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	// Create a project with a provider but no consumer referencing it
	let template = "\
<!-- {@lonely} -->

Nobody references me.

<!-- {/lonely} -->
";
	std::fs::write(tmp.path().join("template.t.md"), template)
		.unwrap_or_else(|e| panic!("write: {e}"));

	let server = MdtMcpServer::new();
	let result = server
		.preview(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "lonely".to_string(),
		}))
		.await
		.unwrap_or_else(|e| panic!("preview: {e:?}"));

	let text = extract_text(&result);
	assert!(
		text.contains("Provider `lonely`"),
		"expected provider heading, got: {text}"
	);
	assert!(
		!text.contains("consumer(s)"),
		"should not contain consumer section when there are no consumers"
	);
}

// ===========================================================================
// check: missing provider detection
// ===========================================================================

#[tokio::test]
async fn check_detects_missing_providers() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	// Create a consumer referencing a provider that doesn't exist
	let template = "\
<!-- {@existing} -->

content

<!-- {/existing} -->
";
	let readme = "\
<!-- {=missing_block} -->

placeholder

<!-- {/missing_block} -->
";
	std::fs::write(tmp.path().join("template.t.md"), template)
		.unwrap_or_else(|e| panic!("write template: {e}"));
	std::fs::write(tmp.path().join("readme.md"), readme)
		.unwrap_or_else(|e| panic!("write readme: {e}"));

	let server = MdtMcpServer::new();
	let result = server
		.check(Parameters(PathParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
		}))
		.await
		.unwrap_or_else(|e| panic!("check: {e:?}"));

	let text = extract_text(&result);
	assert!(
		text.contains("missing providers"),
		"expected missing providers message, got: {text}"
	);
	assert!(
		text.contains("missing_block"),
		"expected missing block name in message, got: {text}"
	);
}

// ===========================================================================
// list: consumer_count tracking
// ===========================================================================

#[tokio::test]
async fn list_shows_correct_consumer_count() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_multi_block_project(tmp.path());

	let server = MdtMcpServer::new();
	let result = server
		.list(Parameters(PathParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
		}))
		.await
		.unwrap_or_else(|e| panic!("list: {e:?}"));

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	let providers = json["providers"]
		.as_array()
		.unwrap_or_else(|| panic!("providers should be array"));

	for provider in providers {
		assert_eq!(
			provider["consumer_count"], 1,
			"each provider should have exactly one consumer"
		);
	}
}

// ===========================================================================
// list: summary field
// ===========================================================================

#[tokio::test]
async fn list_includes_summary() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_multi_block_project(tmp.path());

	let server = MdtMcpServer::new();
	let result = server
		.list(Parameters(PathParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
		}))
		.await
		.unwrap_or_else(|e| panic!("list: {e:?}"));

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	let summary = json["summary"]
		.as_str()
		.unwrap_or_else(|| panic!("summary should be string"));
	assert!(
		summary.contains("2 provider(s)"),
		"expected 2 providers in summary, got: {summary}"
	);
	assert!(
		summary.contains("2 consumer(s)"),
		"expected 2 consumers in summary, got: {summary}"
	);
}

// ===========================================================================
// update: multiple blocks
// ===========================================================================

#[tokio::test]
async fn update_fixes_multiple_stale_blocks() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_multi_block_project(tmp.path());

	let server = MdtMcpServer::new();
	let result = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: false,
		}))
		.await
		.unwrap_or_else(|e| panic!("update: {e:?}"));

	let text = extract_text(&result);
	assert!(
		text.contains("Updated"),
		"expected update confirmation, got: {text}"
	);

	// Verify the stale block was updated
	let readme = std::fs::read_to_string(tmp.path().join("readme.md"))
		.unwrap_or_else(|e| panic!("read readme: {e}"));
	assert!(
		readme.contains("Goodbye from mdt!"),
		"farewell block should be updated"
	);
	assert!(
		readme.contains("Hello from mdt!"),
		"greeting block should remain"
	);
}

// ===========================================================================
// init: created file contains expected content
// ===========================================================================

#[tokio::test]
async fn init_creates_file_with_provider_block() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let server = MdtMcpServer::new();

	server
		.init(Parameters(InitParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
		}))
		.await
		.unwrap_or_else(|e| panic!("init: {e:?}"));

	let content = std::fs::read_to_string(tmp.path().join("template.t.md"))
		.unwrap_or_else(|e| panic!("read template: {e}"));
	assert!(
		content.contains("{@greeting}"),
		"template should contain a provider block"
	);
	assert!(
		content.contains("{/greeting}"),
		"template should contain a closing tag"
	);
}

// ===========================================================================
// get_info
// ===========================================================================

#[test]
fn get_info_returns_server_info() {
	let server = MdtMcpServer::new();
	let info = server.get_info();
	// Should have instructions
	assert!(
		info.instructions.is_some(),
		"get_info should return instructions"
	);
	let instructions = info
		.instructions
		.unwrap_or_else(|| panic!("expected instructions"));
	assert!(
		instructions.contains("mdt"),
		"instructions should mention mdt, got: {instructions}"
	);
	// Should have tool capabilities enabled
	assert!(
		info.capabilities.tools.is_some(),
		"capabilities should have tools enabled"
	);
}

// ===========================================================================
// check: stale consumers with missing providers combined
// ===========================================================================

#[tokio::test]
async fn check_reports_both_stale_and_missing() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	let template = "\
<!-- {@greeting} -->

Hello from mdt!

<!-- {/greeting} -->
";
	let readme = "\
<!-- {=greeting} -->

Old stale content.

<!-- {/greeting} -->

<!-- {=nonexistent} -->

placeholder

<!-- {/nonexistent} -->
";
	std::fs::write(tmp.path().join("template.t.md"), template)
		.unwrap_or_else(|e| panic!("write template: {e}"));
	std::fs::write(tmp.path().join("readme.md"), readme)
		.unwrap_or_else(|e| panic!("write readme: {e}"));

	let server = MdtMcpServer::new();
	let result = server
		.check(Parameters(PathParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
		}))
		.await
		.unwrap_or_else(|e| panic!("check: {e:?}"));

	let text = extract_text(&result);
	// Should mention stale blocks
	assert!(
		text.contains("stale"),
		"expected stale message, got: {text}"
	);
	// Should mention missing providers
	assert!(
		text.contains("missing providers"),
		"expected missing providers message, got: {text}"
	);
	assert!(
		text.contains("nonexistent"),
		"expected nonexistent in message, got: {text}"
	);
}

// ===========================================================================
// update: actual write (not dry_run) with verification
// ===========================================================================

#[tokio::test]
async fn update_writes_files_and_reports_count() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_multi_block_project(tmp.path());

	let server = MdtMcpServer::new();
	let result = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: false,
		}))
		.await
		.unwrap_or_else(|e| panic!("update: {e:?}"));

	let text = extract_text(&result);
	assert!(
		text.contains("Updated"),
		"expected Updated message, got: {text}"
	);
	assert!(
		text.contains("file(s)"),
		"expected file count in message, got: {text}"
	);

	// Verify second run is a no-op
	let result2 = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: false,
		}))
		.await
		.unwrap_or_else(|e| panic!("update: {e:?}"));

	let text2 = extract_text(&result2);
	assert!(
		text2.contains("already up to date"),
		"expected no-changes message on second run, got: {text2}"
	);
}

// ===========================================================================
// get_block: consumer with stale content and provider present
// ===========================================================================

#[tokio::test]
async fn get_block_for_provider_shows_stale_consumers() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());

	let server = MdtMcpServer::new();
	let result = server
		.get_block(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "greeting".to_string(),
		}))
		.await
		.unwrap_or_else(|e| panic!("get_block: {e:?}"));

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	assert_eq!(json["type"], "provider");
	assert_eq!(json["name"], "greeting");
	assert_eq!(json["consumer_count"], 1);
	// rendered_content should contain the provider text
	let rendered = json["rendered_content"]
		.as_str()
		.unwrap_or_else(|| panic!("rendered_content should be string"));
	assert!(
		rendered.contains("Hello from mdt!"),
		"expected provider text in rendered_content"
	);
}

// ===========================================================================
// list: with stale consumers shows correct staleness
// ===========================================================================

#[tokio::test]
async fn list_with_stale_and_synced_consumers() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	// Create project with one stale block (farewell) and one synced (greeting)
	create_multi_block_project(tmp.path());

	let server = MdtMcpServer::new();
	let result = server
		.list(Parameters(PathParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
		}))
		.await
		.unwrap_or_else(|e| panic!("list: {e:?}"));

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	let consumers = json["consumers"]
		.as_array()
		.unwrap_or_else(|| panic!("consumers should be array"));

	// greeting is synced, farewell is stale
	let greeting_consumer = consumers
		.iter()
		.find(|c| c["name"] == "greeting")
		.unwrap_or_else(|| panic!("expected greeting consumer"));
	assert_eq!(
		greeting_consumer["is_stale"], false,
		"greeting should be synced"
	);

	let farewell_consumer = consumers
		.iter()
		.find(|c| c["name"] == "farewell")
		.unwrap_or_else(|| panic!("expected farewell consumer"));
	assert_eq!(
		farewell_consumer["is_stale"], true,
		"farewell should be stale"
	);
}

// ===========================================================================
// resolve_root with None
// ===========================================================================

#[test]
fn resolve_root_none_returns_cwd() {
	let result = resolve_root(None);
	// Should fall back to current directory
	assert!(
		result.is_absolute() || result == PathBuf::from("."),
		"resolve_root(None) should return an absolute path or '.', got: {result:?}"
	);
}

// ===========================================================================
// check: project with data interpolation in templates
// ===========================================================================

#[tokio::test]
async fn check_with_template_data_interpolation() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npkg = \"package.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("package.json"),
		r#"{"name": "my-tool", "version": "2.0.0"}"#,
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@install} -->\n\nnpm install {{ pkg.name }}@{{ pkg.version }}\n\n<!-- {/install} \
		 -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=install} -->\n\nnpm install my-tool@1.0.0\n\n<!-- {/install} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let server = MdtMcpServer::new();
	let result = server
		.check(Parameters(PathParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
		}))
		.await
		.unwrap_or_else(|e| panic!("check: {e:?}"));

	let text = extract_text(&result);
	assert!(
		text.contains("stale"),
		"expected stale message for outdated version, got: {text}"
	);
}

// ===========================================================================
// update: with template data and actual write
// ===========================================================================

#[tokio::test]
async fn update_with_data_interpolation_writes_rendered_content() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npkg = \"package.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("package.json"),
		r#"{"name": "my-tool", "version": "2.0.0"}"#,
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@install} -->\n\nnpm install {{ pkg.name }}@{{ pkg.version }}\n\n<!-- {/install} \
		 -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=install} -->\n\nold\n\n<!-- {/install} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let server = MdtMcpServer::new();
	let result = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: false,
		}))
		.await
		.unwrap_or_else(|e| panic!("update: {e:?}"));

	let text = extract_text(&result);
	assert!(
		text.contains("Updated"),
		"expected Updated message, got: {text}"
	);

	// Verify file was written with rendered content
	let readme_content = std::fs::read_to_string(tmp.path().join("readme.md"))
		.unwrap_or_else(|e| panic!("read readme: {e}"));
	assert!(
		readme_content.contains("npm install my-tool@2.0.0"),
		"readme should contain rendered template content, got: {readme_content}"
	);
}

// ===========================================================================
// get_block: consumer entries when provider exists (stale check with rendering)
// ===========================================================================

#[tokio::test]
async fn get_block_consumer_with_provider_and_data() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npkg = \"package.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("package.json"), r#"{"version": "5.0.0"}"#)
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@ver} -->\n\nv{{ pkg.version }}\n\n<!-- {/ver} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=ver} -->\n\nv4.0.0\n\n<!-- {/ver} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let server = MdtMcpServer::new();
	let result = server
		.get_block(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "ver".to_string(),
		}))
		.await
		.unwrap_or_else(|e| panic!("get_block: {e:?}"));

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	assert_eq!(json["type"], "provider");
	let rendered = json["rendered_content"]
		.as_str()
		.unwrap_or_else(|| panic!("rendered_content should be string"));
	assert!(
		rendered.contains("v5.0.0"),
		"rendered content should contain interpolated version, got: {rendered}"
	);
}

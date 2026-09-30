use std::path::Path;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::ContentBlock;

use super::*;

// ---------------------------------------------------------------------------
// Helper: extract text from the first Content item in a CallToolResult
// ---------------------------------------------------------------------------

fn extract_text(result: &CallToolResult) -> &str {
	let content = result
		.content
		.first()
		.unwrap_or_else(|| panic!("expected content"));
	let ContentBlock::Text(text) = content else {
		panic!("expected text content");
	};

	text.text.as_str()
}

fn extract_json(result: &CallToolResult) -> serde_json::Value {
	if let Some(value) = result.structured_content.clone() {
		return value;
	}
	serde_json::from_str(extract_text(result))
		.unwrap_or_else(|e| panic!("invalid JSON result: {e}"))
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

fn create_formatter_only_stale_project(root: &Path) {
	std::fs::write(
		root.join("mdt.toml"),
		r#"[[formatters]]
command = "/usr/bin/perl -0pe 's/Draft title/Published title/g'"
patterns = ["**/*.md"]
"#,
	)
	.unwrap_or_else(|e| panic!("write config: {e}"));
	std::fs::write(
		root.join("template.t.md"),
		"<!-- {@body} -->\n\nBody content.\n\n<!-- {/body} -->\n",
	)
	.unwrap_or_else(|e| panic!("write template: {e}"));
	std::fs::write(
		root.join("readme.md"),
		"# Draft title\n\n<!-- {=body} -->\n\nBody content.\n\n<!-- {/body} -->\n",
	)
	.unwrap_or_else(|e| panic!("write readme: {e}"));
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

fn create_warning_project(root: &Path) {
	std::fs::write(root.join("mdt.toml"), "[data]\npkg = \"package.json\"\n")
		.unwrap_or_else(|e| panic!("write config: {e}"));
	std::fs::write(
		root.join("package.json"),
		r#"{"name": "my-lib", "version": "1.0.0"}"#,
	)
	.unwrap_or_else(|e| panic!("write package: {e}"));
	std::fs::write(
		root.join("template.t.md"),
		"<!-- {@install} -->\n\nnpm install {{ pkgg.name }}\n\n<!-- {/install} -->\n",
	)
	.unwrap_or_else(|e| panic!("write template: {e}"));
	std::fs::write(
		root.join("readme.md"),
		"<!-- {=install} -->\n\nnpm install \n\n<!-- {/install} -->\n",
	)
	.unwrap_or_else(|e| panic!("write readme: {e}"));
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
	let server = MdtMcpServer::with_base_root(tmp.path());

	let result = server
		.init(Parameters(InitParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["ok"], true);
	assert_eq!(json["action"], "init");
	assert_eq!(json["root"], ".");
	assert_eq!(json["created_root"], false);
	assert_eq!(
		json["config"],
		serde_json::json!({ "status": "created", "file": "mdt.toml" })
	);
	assert_eq!(
		json["sample"],
		serde_json::json!({
			"status": "created_with_readme",
			"template": ".templates/template.t.md",
			"readme": "readme.md",
		})
	);
	assert_eq!(
		json["gitignore"],
		serde_json::json!({ "status": "not_applicable" })
	);
	assert_eq!(
		json["written_files"],
		serde_json::json!(["mdt.toml", ".templates/template.t.md", "readme.md"])
	);
	assert!(
		tmp.path().join(".templates/template.t.md").exists(),
		".templates/template.t.md should exist"
	);
	assert!(
		tmp.path().join("mdt.toml").exists(),
		"mdt.toml should exist"
	);
}

#[tokio::test]
async fn init_matches_the_cli_and_leaves_a_passing_project() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	server.init(Parameters(InitParam::default())).await;
	let config = std::fs::read_to_string(tmp.path().join("mdt.toml"))
		.unwrap_or_else(|e| panic!("read config: {e}"));
	let check = extract_json(&server.check(Parameters(CheckParam::default())).await);

	// The annotated starter config comes from `mdt_core::init`, the same one
	// `mdt init` writes, not a hand-rolled copy.
	assert!(
		config.len() > 1000,
		"expected the annotated config: {config}"
	);
	assert_eq!(
		check["ok"], true,
		"init must leave a passing project: {check}"
	);
}

#[tokio::test]
async fn init_reports_existing_template() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::create_dir_all(tmp.path().join(".templates")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	std::fs::write(
		tmp.path().join(".templates/template.t.md"),
		"existing content",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.init(Parameters(InitParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["ok"], true);
	assert_eq!(
		json["sample"],
		serde_json::json!({ "status": "template_exists", "template": ".templates/template.t.md" })
	);
	assert_eq!(json["config"]["status"], "created");
	assert_eq!(json["written_files"], serde_json::json!(["mdt.toml"]));
	assert_eq!(
		std::fs::read_to_string(tmp.path().join(".templates/template.t.md"))
			.unwrap_or_else(|e| panic!("read: {e}")),
		"existing content",
		"init must not overwrite an existing template"
	);
}

#[tokio::test]
async fn init_twice_writes_nothing_the_second_time() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	server.init(Parameters(InitParam::default())).await;
	let json = extract_json(&server.init(Parameters(InitParam::default())).await);

	assert_eq!(json["ok"], true);
	assert_eq!(json["written_files"], serde_json::json!([]));
	assert_eq!(json["config"]["status"], "exists");
	assert!(
		json["summary"]
			.as_str()
			.is_some_and(|summary| summary.contains("nothing was written")),
		"got: {json}"
	);
}

#[tokio::test]
async fn init_creates_a_missing_subdirectory_and_reports_relative_paths() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	let json = extract_json(
		&server
			.init(Parameters(InitParam {
				path: Some("packages/new".to_string()),
			}))
			.await,
	);

	assert_eq!(json["ok"], true, "got: {json}");
	assert_eq!(json["root"], "packages/new");
	assert_eq!(json["created_root"], true);
	assert_eq!(json["config"]["file"], "mdt.toml");
	assert!(tmp.path().join("packages/new/mdt.toml").exists());
}

#[tokio::test]
async fn init_adds_the_cache_directory_to_gitignore_in_git_repositories() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::create_dir(tmp.path().join(".git")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	let json = extract_json(&server.init(Parameters(InitParam::default())).await);

	assert_eq!(
		json["gitignore"],
		serde_json::json!({ "status": "created", "file": ".gitignore" })
	);
}

#[tokio::test]
async fn init_with_an_invalid_existing_config_is_an_error_result() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data\nbad toml")
		.unwrap_or_else(|e| panic!("write: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	let result = server.init(Parameters(InitParam::default())).await;

	assert_eq!(result.is_error, Some(true));
	let json = extract_json(&result);
	assert_eq!(json["action"], "init");
	assert_eq!(json["error"]["code"], "mdt::config_parse");
}

#[tokio::test]
async fn init_rejects_a_path_that_is_a_file() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("notes.md"), "# notes\n")
		.unwrap_or_else(|e| panic!("write: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	let result = server
		.init(Parameters(InitParam {
			path: Some("notes.md".to_string()),
		}))
		.await;

	assert_eq!(result.is_error, Some(true));
	assert_eq!(
		extract_json(&result)["error"]["code"],
		"mdt::path_not_directory"
	);
}

// ===========================================================================
// check
// ===========================================================================

#[tokio::test]
async fn check_on_empty_project_reports_up_to_date() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	let result = server
		.check(Parameters(CheckParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..CheckParam::default()
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["ok"], true);
	assert_eq!(json["action"], "check");
	assert_eq!(json["stale"], serde_json::json!([]));
	assert_eq!(json["render_errors"], serde_json::json!([]));
}

#[tokio::test]
async fn check_on_synced_project_reports_up_to_date() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_synced_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.check(Parameters(CheckParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..CheckParam::default()
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["ok"], true);
	assert_eq!(json["stale"], serde_json::json!([]));
	assert_eq!(json["render_errors"], serde_json::json!([]));
}

#[tokio::test]
async fn check_on_stale_project_reports_stale_blocks() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.check(Parameters(CheckParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..CheckParam::default()
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["ok"], false);
	assert_eq!(json["action"], "check");
	assert_eq!(json["stale"][0]["block_name"], "greeting");
	assert_eq!(json["stale"][0]["file"], "readme.md");
}

#[tokio::test]
async fn check_reports_formatter_only_stale_files() {
	if cfg!(windows) {
		return;
	}

	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_formatter_only_stale_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.check(Parameters(CheckParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..CheckParam::default()
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["ok"], false);
	assert_eq!(json["stale"], serde_json::json!([]));
	assert_eq!(json["stale_files"][0], "readme.md");
}

// ===========================================================================
// update
// ===========================================================================

#[tokio::test]
async fn update_on_up_to_date_project_reports_no_changes() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_synced_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: false,
			..UpdateParam::default()
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["ok"], true);
	assert_eq!(json["action"], "update");
	assert_eq!(json["updated_count"], 0);
	assert_eq!(json["dry_run"], false);
}

#[tokio::test]
async fn update_on_stale_project_applies_changes() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: false,
			..UpdateParam::default()
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["ok"], true);
	assert_eq!(json["action"], "update");
	assert_eq!(json["updated_count"], 1);
	assert_eq!(json["updated_files"][0], "readme.md");

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
async fn update_formatter_only_stale_project_normalizes_file() {
	if cfg!(windows) {
		return;
	}

	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_formatter_only_stale_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: false,
			..UpdateParam::default()
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["ok"], true);
	assert_eq!(json["updated_count"], 0);
	assert_eq!(json["updated_files"][0], "readme.md");
	assert!(
		json["summary"]
			.as_str()
			.unwrap_or_else(|| panic!("summary string"))
			.contains("Normalized 1 file(s) via formatter integration.")
	);
	let readme_content = std::fs::read_to_string(tmp.path().join("readme.md"))
		.unwrap_or_else(|e| panic!("read readme: {e}"));
	assert!(readme_content.contains("Published title"));
}

#[tokio::test]
async fn update_dry_run_does_not_write() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: true,
			..UpdateParam::default()
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["ok"], true);
	assert_eq!(json["action"], "update");
	assert_eq!(json["dry_run"], true);
	assert_eq!(json["updated_count"], 1);

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

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: true,
			..UpdateParam::default()
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["updated_files"][0], "readme.md");
}

#[tokio::test]
async fn update_includes_template_warnings_in_json() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_warning_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: true,
			..UpdateParam::default()
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["warnings"][0]["block_name"], "install");
	assert_eq!(json["warnings"][0]["undefined_variables"][0], "pkgg.name");
}

// ===========================================================================
// list
// ===========================================================================

#[tokio::test]
async fn list_on_empty_project_returns_empty() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	let result = server
		.list(Parameters(ListParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..ListParam::default()
		}))
		.await;

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

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.list(Parameters(ListParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..ListParam::default()
		}))
		.await;

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

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.list(Parameters(ListParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..ListParam::default()
		}))
		.await;

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

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.list(Parameters(ListParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..ListParam::default()
		}))
		.await;

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

#[tokio::test]
async fn find_reuse_suggests_similar_blocks_and_file_types() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	let template = "\
<!-- {@greeting} -->

Hello from mdt!

<!-- {/greeting} -->

<!-- {@goodbye} -->

Bye!

<!-- {/goodbye} -->
";
	let readme = "\
<!-- {=greeting} -->

Old markdown content.

<!-- {/greeting} -->
";
	let source = "\
//! <!-- {=greeting} -->
//!
//! Old source content.
//!
//! <!-- {/greeting} -->
";

	std::fs::create_dir_all(tmp.path().join("src"))
		.unwrap_or_else(|e| panic!("create src dir: {e}"));
	std::fs::write(tmp.path().join("template.t.md"), template)
		.unwrap_or_else(|e| panic!("write template: {e}"));
	std::fs::write(tmp.path().join("readme.md"), readme)
		.unwrap_or_else(|e| panic!("write readme: {e}"));
	std::fs::write(tmp.path().join("src/lib.rs"), source)
		.unwrap_or_else(|e| panic!("write source: {e}"));

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.find_reuse(Parameters(ReuseParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: Some("greting".to_string()),
			limit: 5,
			..ReuseParam::default()
		}))
		.await;

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	assert!(
		json["guidance"]
			.as_str()
			.is_some_and(|guidance| guidance.contains("reusing an existing provider")),
		"expected reuse guidance, got: {text}"
	);

	let candidates = json["candidates"]
		.as_array()
		.unwrap_or_else(|| panic!("candidates should be array"));
	assert!(!candidates.is_empty(), "expected at least one candidate");
	assert_eq!(candidates[0]["name"], "greeting");
	assert_eq!(candidates[0]["consumer_count"], 2);
	assert!(
		candidates[0]["markdown_files"]
			.as_array()
			.unwrap_or_else(|| panic!("markdown_files should be array"))
			.iter()
			.any(|value| value == "readme.md"),
		"expected readme.md in markdown_files, got: {text}"
	);
	assert!(
		candidates[0]["code_files"]
			.as_array()
			.unwrap_or_else(|| panic!("code_files should be array"))
			.iter()
			.any(|value| value == "src/lib.rs"),
		"expected src/lib.rs in code_files, got: {text}"
	);
}

#[tokio::test]
async fn find_reuse_sorts_by_consumer_count_without_query() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	let template = "\
<!-- {@popular} -->

Popular block

<!-- {/popular} -->

<!-- {@rare} -->

Rare block

<!-- {/rare} -->
";
	let readme = "\
<!-- {=popular} -->

Old content.

<!-- {/popular} -->

<!-- {=rare} -->

Old content.

<!-- {/rare} -->
";
	let changelog = "\
<!-- {=popular} -->

Old changelog content.

<!-- {/popular} -->
";

	std::fs::write(tmp.path().join("template.t.md"), template)
		.unwrap_or_else(|e| panic!("write template: {e}"));
	std::fs::write(tmp.path().join("readme.md"), readme)
		.unwrap_or_else(|e| panic!("write readme: {e}"));
	std::fs::write(tmp.path().join("changelog.md"), changelog)
		.unwrap_or_else(|e| panic!("write changelog: {e}"));

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.find_reuse(Parameters(ReuseParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: None,
			limit: 2,
			..ReuseParam::default()
		}))
		.await;

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));
	let candidates = json["candidates"]
		.as_array()
		.unwrap_or_else(|| panic!("candidates should be array"));

	assert_eq!(candidates.len(), 2);
	assert_eq!(candidates[0]["name"], "popular");
	assert_eq!(candidates[0]["consumer_count"], 2);
	assert_eq!(candidates[1]["name"], "rare");
	assert_eq!(candidates[1]["consumer_count"], 1);
}

// ===========================================================================
// get_block
// ===========================================================================

#[tokio::test]
async fn get_block_for_provider_returns_provider_info() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.get_block(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "greeting".to_string(),
		}))
		.await;

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	assert!(json["provider"].is_object(), "got: {json}");
	assert_eq!(json["block_name"], "greeting");
	assert_eq!(json["provider"]["consumer_count"], 1);

	let rendered = json["provider"]["rendered_with_project_data"]
		.as_str()
		.unwrap_or_else(|| panic!("rendered_with_project_data should be string"));
	assert!(
		rendered.contains("Hello from mdt!"),
		"rendered content should contain provider text"
	);
}

#[tokio::test]
async fn get_block_for_provider_lists_consumer_files() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.get_block(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "greeting".to_string(),
		}))
		.await;

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	let consumer_files: Vec<_> = json["consumers"]
		.as_array()
		.unwrap_or_else(|| panic!("consumers should be array"))
		.iter()
		.map(|consumer| consumer["file"].clone())
		.collect();
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

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.get_block(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "orphan".to_string(),
		}))
		.await;

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	// One object shape: no provider, the orphan consumer listed under `consumers`.
	assert_eq!(
		json["ok"], false,
		"orphan consumers cannot be synced: {json}"
	);
	assert_eq!(json["action"], "get_block");
	assert_eq!(json["block_name"], "orphan");
	assert!(json["provider"].is_null(), "got: {json}");
	let entries = json["consumers"]
		.as_array()
		.unwrap_or_else(|| panic!("expected array of consumer entries"));
	assert_eq!(entries.len(), 1);
	assert_eq!(entries[0]["type"], "consumer");
	assert_eq!(entries[0]["name"], "orphan");
	assert_eq!(entries[0]["status"], "orphan");
	assert_eq!(entries[0]["line"], 1);
	assert_eq!(
		entries[0]["current_content"],
		"\n\nSome orphan content.\n\n"
	);
}

#[tokio::test]
async fn get_block_for_nonexistent_returns_error() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.get_block(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "nonexistent".to_string(),
		}))
		.await;

	assert_eq!(
		result.is_error,
		Some(true),
		"result should be marked as error"
	);
	let json = extract_json(&result);
	assert_eq!(json["ok"], false);
	assert!(
		json["summary"]
			.as_str()
			.is_some_and(|summary| summary.contains("No block named")),
		"expected 'No block named' message, got: {json}"
	);
}

// ===========================================================================
// preview
// ===========================================================================

#[tokio::test]
async fn preview_for_existing_provider_returns_rendered_content() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.preview(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "greeting".to_string(),
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["ok"], true);
	assert_eq!(json["action"], "preview");
	assert_eq!(json["provider"]["name"], "greeting");
	assert!(
		json["provider"]["rendered_with_project_data"]
			.as_str()
			.is_some_and(|value| value.contains("Hello from mdt!")),
		"expected rendered provider content, got: {json}"
	);
}

#[tokio::test]
async fn preview_shows_consumer_info() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.preview(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "greeting".to_string(),
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["consumers"][0]["file"], "readme.md");
	assert_eq!(json["consumers"][0]["is_stale"], true);
	assert!(
		json["consumers"][0]["rendered_content"]
			.as_str()
			.is_some_and(|value| value.contains("Hello from mdt!")),
		"expected rendered consumer preview, got: {json}"
	);
}

#[tokio::test]
async fn preview_for_nonexistent_provider_returns_error() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.preview(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "nonexistent".to_string(),
		}))
		.await;

	assert_eq!(
		result.is_error,
		Some(true),
		"result should be marked as error"
	);
	let json = extract_json(&result);
	assert_eq!(json["ok"], false);
	assert!(
		json["summary"]
			.as_str()
			.is_some_and(|summary| summary.contains("No provider named")),
		"expected missing-provider error, got: {json}"
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

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.preview(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "lonely".to_string(),
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["provider"]["name"], "lonely");
	assert_eq!(json["consumers"], serde_json::json!([]));
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

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.check(Parameters(CheckParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..CheckParam::default()
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["ok"], false);
	assert_eq!(json["missing_provider_names"][0], "missing_block");
}

#[tokio::test]
async fn check_includes_template_warnings_in_json() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_warning_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.check(Parameters(CheckParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..CheckParam::default()
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["warnings"][0]["block_name"], "install");
	assert_eq!(json["warnings"][0]["undefined_variables"][0], "pkgg.name");
}

// ===========================================================================
// list: consumer_count tracking
// ===========================================================================

#[tokio::test]
async fn list_shows_correct_consumer_count() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_multi_block_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.list(Parameters(ListParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..ListParam::default()
		}))
		.await;

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

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.list(Parameters(ListParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..ListParam::default()
		}))
		.await;

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

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: false,
			..UpdateParam::default()
		}))
		.await;

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
	let server = MdtMcpServer::with_base_root(tmp.path());

	server
		.init(Parameters(InitParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
		}))
		.await;

	let content = std::fs::read_to_string(tmp.path().join(".templates/template.t.md"))
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
	assert!(
		instructions.contains("mdt_find_reuse"),
		"instructions should encourage reuse discovery, got: {instructions}"
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

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.check(Parameters(CheckParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..CheckParam::default()
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["ok"], false);
	assert_eq!(json["stale"][0]["block_name"], "greeting");
	assert_eq!(json["missing_provider_names"][0], "nonexistent");
}

// ===========================================================================
// update: actual write (not dry_run) with verification
// ===========================================================================

#[tokio::test]
async fn update_writes_files_and_reports_count() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_multi_block_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: false,
			..UpdateParam::default()
		}))
		.await;

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
			..UpdateParam::default()
		}))
		.await;

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

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.get_block(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "greeting".to_string(),
		}))
		.await;

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	assert!(json["provider"].is_object(), "got: {json}");
	assert_eq!(json["block_name"], "greeting");
	assert_eq!(json["provider"]["consumer_count"], 1);
	// rendered_content should contain the provider text
	let rendered = json["provider"]["rendered_with_project_data"]
		.as_str()
		.unwrap_or_else(|| panic!("rendered_with_project_data should be string"));
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

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.list(Parameters(ListParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..ListParam::default()
		}))
		.await;

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

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.check(Parameters(CheckParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..CheckParam::default()
		}))
		.await;

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

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: false,
			..UpdateParam::default()
		}))
		.await;

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

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.get_block(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "ver".to_string(),
		}))
		.await;

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	assert!(json["provider"].is_object(), "got: {json}");
	let rendered = json["provider"]["rendered_with_project_data"]
		.as_str()
		.unwrap_or_else(|| panic!("rendered_with_project_data should be string"));
	assert!(
		rendered.contains("v5.0.0"),
		"rendered content should contain interpolated version, got: {rendered}"
	);
}

// ===========================================================================
// Missing and non-directory project paths
// ===========================================================================

/// Call every tool except `mdt_init` with `path`, labelled by action.
async fn call_project_tools(
	server: &MdtMcpServer,
	path: &str,
) -> Vec<(&'static str, CallToolResult)> {
	let path = Some(path.to_string());
	let block = || {
		Parameters(BlockParam {
			path: path.clone(),
			block_name: "greeting".to_string(),
		})
	};
	vec![
		(
			"check",
			server
				.check(Parameters(CheckParam {
					path: path.clone(),
					..CheckParam::default()
				}))
				.await,
		),
		(
			"update",
			server
				.update(Parameters(UpdateParam {
					path: path.clone(),
					..UpdateParam::default()
				}))
				.await,
		),
		(
			"list",
			server
				.list(Parameters(ListParam {
					path: path.clone(),
					..ListParam::default()
				}))
				.await,
		),
		(
			"find_reuse",
			server
				.find_reuse(Parameters(ReuseParam {
					path: path.clone(),
					..ReuseParam::default()
				}))
				.await,
		),
		("get_block", server.get_block(block()).await),
		("preview", server.preview(block()).await),
	]
}

#[tokio::test]
async fn tools_report_a_missing_path_as_an_error_result() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_synced_project(tmp.path());
	let server = MdtMcpServer::with_base_root(tmp.path());

	for (action, result) in call_project_tools(&server, "missing").await {
		let json = extract_json(&result);
		assert_eq!(result.is_error, Some(true), "{action}: {json}");
		assert_eq!(json["ok"], false, "{action}: {json}");
		assert_eq!(json["action"], action);
		assert_eq!(json["error"]["code"], "mdt::path_not_found", "{action}");
		assert_eq!(json["summary"], "path `missing` does not exist", "{action}");
	}
}

#[tokio::test]
async fn tools_report_a_file_path_as_an_error_result() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_synced_project(tmp.path());
	let server = MdtMcpServer::with_base_root(tmp.path());

	for (action, result) in call_project_tools(&server, "readme.md").await {
		let json = extract_json(&result);
		assert_eq!(result.is_error, Some(true), "{action}: {json}");
		assert_eq!(json["error"]["code"], "mdt::path_not_directory", "{action}");
		assert_eq!(
			json["summary"], "path `readme.md` is not a directory",
			"{action}"
		);
	}
}

#[tokio::test]
async fn tools_report_a_missing_server_root_as_an_error_result() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path().join("gone"));

	let result = server.check(Parameters(CheckParam::default())).await;

	assert_eq!(result.is_error, Some(true));
	assert_eq!(
		extract_json(&result)["error"]["code"],
		"mdt::path_not_found"
	);
}

// ===========================================================================
// check: project with only providers and no consumers
// ===========================================================================

#[tokio::test]
async fn check_project_providers_only_no_consumers() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	let template = "\
<!-- {@greeting} -->

Hello from mdt!

<!-- {/greeting} -->
";
	std::fs::write(tmp.path().join("template.t.md"), template)
		.unwrap_or_else(|e| panic!("write template: {e}"));

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.check(Parameters(CheckParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..CheckParam::default()
		}))
		.await;

	let text = extract_text(&result);
	assert!(
		text.contains("up to date"),
		"project with only providers should be up to date, got: {text}"
	);
}

// ===========================================================================
// list: project with only providers shows empty consumers
// ===========================================================================

#[tokio::test]
async fn list_project_with_only_providers() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	let template = "\
<!-- {@greeting} -->

Hello from mdt!

<!-- {/greeting} -->
";
	std::fs::write(tmp.path().join("template.t.md"), template)
		.unwrap_or_else(|e| panic!("write template: {e}"));

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.list(Parameters(ListParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..ListParam::default()
		}))
		.await;

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	let providers = json["providers"]
		.as_array()
		.unwrap_or_else(|| panic!("providers should be array"));
	assert_eq!(providers.len(), 1);
	assert_eq!(providers[0]["name"], "greeting");
	assert_eq!(providers[0]["consumer_count"], 0);

	let consumers = json["consumers"]
		.as_array()
		.unwrap_or_else(|| panic!("consumers should be array"));
	assert!(consumers.is_empty(), "should have no consumers");
}

// ===========================================================================
// list: project with only consumers (orphans) shows empty providers
// ===========================================================================

#[tokio::test]
async fn list_project_with_only_consumers() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	let readme = "\
<!-- {=orphan} -->

Some content.

<!-- {/orphan} -->
";
	std::fs::write(tmp.path().join("readme.md"), readme)
		.unwrap_or_else(|e| panic!("write readme: {e}"));
	// Need a template file for scanning (even if empty).
	std::fs::write(tmp.path().join("template.t.md"), "")
		.unwrap_or_else(|e| panic!("write template: {e}"));

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.list(Parameters(ListParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..ListParam::default()
		}))
		.await;

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	let providers = json["providers"]
		.as_array()
		.unwrap_or_else(|| panic!("providers should be array"));
	assert!(providers.is_empty(), "should have no providers");

	let consumers = json["consumers"]
		.as_array()
		.unwrap_or_else(|| panic!("consumers should be array"));
	assert_eq!(consumers.len(), 1);
	assert_eq!(consumers[0]["name"], "orphan");
	// Consumer with missing provider is not stale (no provider to compare
	// against).
	assert_eq!(consumers[0]["is_stale"], false);
}

// ===========================================================================
// list: consumer transformers are listed
// ===========================================================================

#[tokio::test]
async fn list_shows_consumer_transformers() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	let template = "\
<!-- {@greeting} -->

Hello from mdt!

<!-- {/greeting} -->
";
	let readme = "\
<!-- {=greeting|trim|indent:\"  \"} -->

  Hello from mdt!

<!-- {/greeting} -->
";
	std::fs::write(tmp.path().join("template.t.md"), template)
		.unwrap_or_else(|e| panic!("write template: {e}"));
	std::fs::write(tmp.path().join("readme.md"), readme)
		.unwrap_or_else(|e| panic!("write readme: {e}"));

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.list(Parameters(ListParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..ListParam::default()
		}))
		.await;

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	let consumers = json["consumers"]
		.as_array()
		.unwrap_or_else(|| panic!("consumers should be array"));
	assert_eq!(consumers.len(), 1);

	let transformers = consumers[0]["transformers"]
		.as_array()
		.unwrap_or_else(|| panic!("transformers should be array"));
	assert!(
		transformers.iter().any(|t| t.as_str() == Some("trim")),
		"expected trim transformer"
	);
	assert!(
		transformers.iter().any(|t| t.as_str() == Some("indent")),
		"expected indent transformer"
	);
}

// ===========================================================================
// update: dry_run reports block count
// ===========================================================================

#[tokio::test]
async fn update_dry_run_reports_block_and_file_count() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_multi_block_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: true,
			..UpdateParam::default()
		}))
		.await;

	let text = extract_text(&result);
	assert!(
		text.contains("Dry run"),
		"expected dry run prefix, got: {text}"
	);
	assert!(
		text.contains("block(s)"),
		"expected block count in dry run output, got: {text}"
	);
	assert!(
		text.contains("file(s)"),
		"expected file count in dry run output, got: {text}"
	);
}

// ===========================================================================
// update: on empty project reports no changes
// ===========================================================================

#[tokio::test]
async fn update_empty_project_reports_no_changes() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: false,
			..UpdateParam::default()
		}))
		.await;

	let text = extract_text(&result);
	assert!(
		text.contains("already up to date"),
		"empty project should report no changes, got: {text}"
	);
}

// ===========================================================================
// update: dry_run on synced project reports no changes
// ===========================================================================

#[tokio::test]
async fn update_dry_run_synced_project_no_changes() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_synced_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: true,
			..UpdateParam::default()
		}))
		.await;

	let text = extract_text(&result);
	assert!(
		text.contains("already up to date"),
		"synced project dry run should report no changes, got: {text}"
	);
}

// ===========================================================================
// get_block: provider with no consumers lists empty consumer_files
// ===========================================================================

#[tokio::test]
async fn get_block_provider_with_no_consumers() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	let template = "\
<!-- {@lonely} -->

Nobody references me.

<!-- {/lonely} -->
";
	std::fs::write(tmp.path().join("template.t.md"), template)
		.unwrap_or_else(|e| panic!("write: {e}"));

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.get_block(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "lonely".to_string(),
		}))
		.await;

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	assert!(json["provider"].is_object(), "got: {json}");
	assert_eq!(json["block_name"], "lonely");
	assert_eq!(json["provider"]["consumer_count"], 0);
	let consumer_files: Vec<_> = json["consumers"]
		.as_array()
		.unwrap_or_else(|| panic!("consumers should be array"))
		.iter()
		.map(|consumer| consumer["file"].clone())
		.collect();
	assert!(consumer_files.is_empty(), "should have no consumer files");
}

// ===========================================================================
// get_block: provider raw_content vs rendered_content differ with data
// ===========================================================================

#[tokio::test]
async fn get_block_provider_raw_vs_rendered_content() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npkg = \"package.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("package.json"),
		r#"{"name": "my-lib", "version": "3.0.0"}"#,
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@install} -->\n\nnpm install {{ pkg.name }}@{{ pkg.version }}\n\n<!-- {/install} \
		 -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.get_block(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "install".to_string(),
		}))
		.await;

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	let raw = json["provider"]["raw_content"]
		.as_str()
		.unwrap_or_else(|| panic!("raw_content should be string"));
	let rendered = json["provider"]["rendered_with_project_data"]
		.as_str()
		.unwrap_or_else(|| panic!("rendered_with_project_data should be string"));

	// raw_content should still have template syntax.
	assert!(
		raw.contains("{{ pkg.name }}"),
		"raw_content should contain template syntax, got: {raw}"
	);
	// rendered_content should have interpolated values.
	assert!(
		rendered.contains("npm install my-lib@3.0.0"),
		"rendered_content should contain interpolated values, got: {rendered}"
	);
}

// ===========================================================================
// preview: with data interpolation
// ===========================================================================

#[tokio::test]
async fn preview_with_data_interpolation() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npkg = \"package.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("package.json"), r#"{"version": "7.0.0"}"#)
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@ver} -->\n\nv{{ pkg.version }}\n\n<!-- {/ver} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=ver} -->\n\nv6.0.0\n\n<!-- {/ver} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.preview(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "ver".to_string(),
		}))
		.await;

	let text = extract_text(&result);
	assert!(
		text.contains("v7.0.0"),
		"preview should show rendered content with interpolation, got: {text}"
	);
	assert!(
		text.contains("consumer(s)"),
		"preview should show consumer section, got: {text}"
	);
}

// ===========================================================================
// preview: with transformers applied to consumers
// ===========================================================================

#[tokio::test]
async fn preview_shows_transformed_consumer_content() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	let template = "\
<!-- {@greeting} -->

Hello from mdt!

<!-- {/greeting} -->
";
	let readme = "\
<!-- {=greeting|trim} -->

Hello from mdt!

<!-- {/greeting} -->
";
	std::fs::write(tmp.path().join("template.t.md"), template)
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("readme.md"), readme).unwrap_or_else(|e| panic!("write: {e}"));

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.preview(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "greeting".to_string(),
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["consumers"][0]["transformers"][0], "trim");
}

// ===========================================================================
// check: stale project reports stale block names in output
// ===========================================================================

#[tokio::test]
async fn check_stale_project_reports_block_name_and_file() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.check(Parameters(CheckParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..CheckParam::default()
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["stale"][0]["block_name"], "greeting");
	assert_eq!(json["stale"][0]["file"], "readme.md");
	assert!(
		json["summary"]
			.as_str()
			.is_some_and(|summary| summary.contains("stale consumer block")),
		"check output should include summary, got: {json}"
	);
}

// ===========================================================================
// update: idempotent (second update on same project is no-op)
// ===========================================================================

#[tokio::test]
async fn update_is_idempotent() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());

	// First update.
	let result1 = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: false,
			..UpdateParam::default()
		}))
		.await;
	assert!(
		extract_text(&result1).contains("Updated"),
		"first update should make changes"
	);

	// Read the file content after first update.
	let content_after_first = std::fs::read_to_string(tmp.path().join("readme.md"))
		.unwrap_or_else(|e| panic!("read: {e}"));

	// Second update.
	let result2 = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: false,
			..UpdateParam::default()
		}))
		.await;
	assert!(
		extract_text(&result2).contains("already up to date"),
		"second update should be no-op"
	);

	// Content should be unchanged.
	let content_after_second = std::fs::read_to_string(tmp.path().join("readme.md"))
		.unwrap_or_else(|e| panic!("read: {e}"));
	assert_eq!(
		content_after_first, content_after_second,
		"content should be unchanged after idempotent update"
	);
}

// ===========================================================================
// init: path with nested directories
// ===========================================================================

#[tokio::test]
async fn init_in_nested_directory() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let nested = tmp.path().join("a").join("b").join("c");
	std::fs::create_dir_all(&nested).unwrap_or_else(|e| panic!("mkdir: {e}"));

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.init(Parameters(InitParam {
			path: Some(nested.to_string_lossy().to_string()),
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["root"], "a/b/c", "got: {json}");
	assert_eq!(json["sample"]["template"], ".templates/template.t.md");
	assert!(
		json["summary"]
			.as_str()
			.is_some_and(|summary| summary.starts_with("Initialized mdt in `a/b/c`")),
		"expected creation message, got: {json}"
	);
	assert!(
		nested.join(".templates/template.t.md").exists(),
		"template should be created in nested dir"
	);
	assert!(
		nested.join("mdt.toml").exists(),
		"mdt.toml should be created"
	);
}

// ===========================================================================
// check: multiple stale blocks reports count
// ===========================================================================

#[tokio::test]
async fn check_multiple_stale_blocks_reports_count() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_multi_block_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.check(Parameters(CheckParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..CheckParam::default()
		}))
		.await;

	let text = extract_text(&result);
	// multi_block_project has farewell as stale (greeting is synced).
	assert!(
		text.contains("stale"),
		"expected stale blocks in output, got: {text}"
	);
	assert!(
		text.contains("farewell"),
		"expected farewell block name, got: {text}"
	);
}

// ===========================================================================
// get_block: multiple consumers for same block
// ===========================================================================

#[tokio::test]
async fn get_block_provider_with_multiple_consumers() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	let template = "\
<!-- {@greeting} -->

Hello from mdt!

<!-- {/greeting} -->
";
	let readme = "\
<!-- {=greeting} -->

Old content 1.

<!-- {/greeting} -->
";
	let docs = "\
<!-- {=greeting} -->

Old content 2.

<!-- {/greeting} -->
";
	std::fs::write(tmp.path().join("template.t.md"), template)
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("readme.md"), readme).unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("docs.md"), docs).unwrap_or_else(|e| panic!("write: {e}"));

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.get_block(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "greeting".to_string(),
		}))
		.await;

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	assert!(json["provider"].is_object(), "got: {json}");
	assert_eq!(json["provider"]["consumer_count"], 2);
	let consumer_files: Vec<_> = json["consumers"]
		.as_array()
		.unwrap_or_else(|| panic!("consumers should be array"))
		.iter()
		.map(|consumer| consumer["file"].clone())
		.collect();
	assert_eq!(consumer_files.len(), 2);
}

// ===========================================================================
// update: with multiple files being updated
// ===========================================================================

#[tokio::test]
async fn update_multiple_files() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	let template = "\
<!-- {@greeting} -->

Hello from mdt!

<!-- {/greeting} -->
";
	let readme = "\
<!-- {=greeting} -->

Old readme content.

<!-- {/greeting} -->
";
	let docs = "\
<!-- {=greeting} -->

Old docs content.

<!-- {/greeting} -->
";
	std::fs::write(tmp.path().join("template.t.md"), template)
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("readme.md"), readme).unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("docs.md"), docs).unwrap_or_else(|e| panic!("write: {e}"));

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: false,
			..UpdateParam::default()
		}))
		.await;

	let text = extract_text(&result);
	assert!(text.contains("Updated"), "expected update, got: {text}");
	assert!(
		text.contains("2 file(s)"),
		"expected 2 files updated, got: {text}"
	);

	// Verify both files were updated.
	let readme_content = std::fs::read_to_string(tmp.path().join("readme.md"))
		.unwrap_or_else(|e| panic!("read: {e}"));
	let docs_content =
		std::fs::read_to_string(tmp.path().join("docs.md")).unwrap_or_else(|e| panic!("read: {e}"));

	assert!(
		readme_content.contains("Hello from mdt!"),
		"readme should be updated"
	);
	assert!(
		docs_content.contains("Hello from mdt!"),
		"docs should be updated"
	);
}

// ===========================================================================
// list: summary format is correct
// ===========================================================================

#[tokio::test]
async fn list_summary_format() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.list(Parameters(ListParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..ListParam::default()
		}))
		.await;

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	let summary = json["summary"]
		.as_str()
		.unwrap_or_else(|| panic!("summary should be string"));
	assert!(
		summary.contains("1 provider(s)"),
		"expected '1 provider(s)' in summary, got: {summary}"
	);
	assert!(
		summary.contains("1 consumer(s)"),
		"expected '1 consumer(s)' in summary, got: {summary}"
	);
}

// ===========================================================================
// list: provider content is trimmed in output
// ===========================================================================

#[tokio::test]
async fn list_provider_content_is_trimmed() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.list(Parameters(ListParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			include_content: true,
			..ListParam::default()
		}))
		.await;

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	let providers = json["providers"]
		.as_array()
		.unwrap_or_else(|| panic!("providers should be array"));
	let content = providers[0]["content"]
		.as_str()
		.unwrap_or_else(|| panic!("content should be string"));
	// Content should be trimmed (no leading/trailing whitespace).
	assert_eq!(
		content,
		content.trim(),
		"provider content should be trimmed in list output"
	);
}

// ===========================================================================
// list: provider file paths are relative
// ===========================================================================

#[tokio::test]
async fn list_uses_relative_file_paths() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.list(Parameters(ListParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..ListParam::default()
		}))
		.await;

	let text = extract_text(&result);
	let json: serde_json::Value =
		serde_json::from_str(text).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

	let providers = json["providers"]
		.as_array()
		.unwrap_or_else(|| panic!("providers should be array"));
	let file = providers[0]["file"]
		.as_str()
		.unwrap_or_else(|| panic!("file should be string"));
	assert_eq!(
		file, "template.t.md",
		"provider file path should be relative"
	);

	let consumers = json["consumers"]
		.as_array()
		.unwrap_or_else(|| panic!("consumers should be array"));
	let consumer_file = consumers[0]["file"]
		.as_str()
		.unwrap_or_else(|| panic!("file should be string"));
	assert_eq!(
		consumer_file, "readme.md",
		"consumer file path should be relative"
	);
}

// ===========================================================================
// preview: with multiple consumers shows all
// ===========================================================================

#[tokio::test]
async fn preview_with_multiple_consumers() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

	let template = "\
<!-- {@greeting} -->

Hello from mdt!

<!-- {/greeting} -->
";
	let readme = "\
<!-- {=greeting} -->

Old readme.

<!-- {/greeting} -->
";
	let docs = "\
<!-- {=greeting|trim} -->

Hello from mdt!

<!-- {/greeting} -->
";
	std::fs::write(tmp.path().join("template.t.md"), template)
		.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("readme.md"), readme).unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(tmp.path().join("docs.md"), docs).unwrap_or_else(|e| panic!("write: {e}"));

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.preview(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "greeting".to_string(),
		}))
		.await;

	let text = extract_text(&result);
	assert!(
		text.contains("2 consumer(s)"),
		"expected 2 consumers, got: {text}"
	);
	assert!(
		text.contains("readme.md"),
		"expected readme.md in output, got: {text}"
	);
	assert!(
		text.contains("docs.md"),
		"expected docs.md in output, got: {text}"
	);
}

// ===========================================================================
// Tool-level failures are isError results, not JSON-RPC errors
// ===========================================================================

#[tokio::test]
async fn tools_report_an_invalid_config_as_an_error_result() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data\nbad toml")
		.unwrap_or_else(|e| panic!("write: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	for (action, result) in call_project_tools(&server, ".").await {
		let json = extract_json(&result);
		assert_eq!(result.is_error, Some(true), "{action}: {json}");
		assert_eq!(json["ok"], false, "{action}");
		assert_eq!(json["action"], action);
		assert_eq!(json["error"]["code"], "mdt::config_parse", "{action}");
		assert_eq!(json["summary"], json["error"]["message"], "{action}");
		assert!(
			json["error"]["help"]
				.as_str()
				.is_some_and(|help| help.contains("mdt.toml")),
			"{action}: expected the diagnostic's help text, got: {json}"
		);
	}
}

#[tokio::test]
async fn check_reports_a_missing_data_file_as_an_error_result() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npkg = \"nonexistent.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@ver} -->\n\nv{{ pkg.version }}\n\n<!-- {/ver} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	let result = server.check(Parameters(CheckParam::default())).await;

	assert_eq!(result.is_error, Some(true));
	let json = extract_json(&result);
	assert_eq!(json["error"]["code"], "mdt::data_file", "got: {json}");
	assert!(
		json["error"]["message"]
			.as_str()
			.is_some_and(|message| message.contains("nonexistent.json")),
		"got: {json}"
	);
}

#[tokio::test]
async fn check_reports_duplicate_providers_as_an_error_result() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_synced_project(tmp.path());
	std::fs::write(
		tmp.path().join("other.t.md"),
		"<!-- {@greeting} -->\n\nAgain.\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	let result = server.check(Parameters(CheckParam::default())).await;

	assert_eq!(result.is_error, Some(true));
	assert_eq!(
		extract_json(&result)["error"]["code"],
		"mdt::duplicate_provider"
	);
}

#[tokio::test]
async fn update_reports_a_failing_formatter_as_an_error_result() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[[formatters]]\ncommand = \"exit 3\"\npatterns = [\"**/*.md\"]\n",
	)
	.unwrap_or_else(|e| panic!("write config: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	let result = server.update(Parameters(UpdateParam::default())).await;

	assert_eq!(result.is_error, Some(true));
	let json = extract_json(&result);
	assert_eq!(json["action"], "update");
	assert_eq!(json["error"]["code"], "mdt::formatter", "got: {json}");
	let readme = std::fs::read_to_string(tmp.path().join("readme.md"))
		.unwrap_or_else(|e| panic!("read readme: {e}"));
	assert!(
		readme.contains("Old stale content."),
		"a failed update must not write"
	);
}

// ===========================================================================
// Block arguments helpers
// ===========================================================================

/// Create a project where the provider declares a parameter (`crate_name`)
/// and the consumer passes an argument value (`mdt_core`).  The consumer
/// body is stale — it does NOT match the rendered provider content.
fn create_stale_args_project(root: &Path) {
	let template = "\
<!-- {@badges:\"crate_name\"} -->

[![crates.io](https://img.shields.io/crates/v/{{ \
	                crate_name }})]

<!-- {/badges} -->
";
	let readme = "\
<!-- {=badges:\"mdt_core\"} -->

Old stale content.

<!-- {/badges} -->
";
	std::fs::write(root.join("template.t.md"), template)
		.unwrap_or_else(|e| panic!("write template: {e}"));
	std::fs::write(root.join("readme.md"), readme).unwrap_or_else(|e| panic!("write readme: {e}"));
}

/// Create a project where the consumer passes too many arguments relative
/// to the provider's parameter count (count mismatch).
fn create_args_mismatch_project(root: &Path) {
	let template = "\
<!-- {@badges:\"crate_name\"} -->

Content for {{ crate_name }}.

<!-- {/badges} -->
";
	// Consumer passes 2 arguments, but provider declares only 1 parameter.
	let readme = "\
<!-- {=badges:\"mdt_core\":\"extra\"} -->

Old content.

<!-- {/badges} -->
";
	std::fs::write(root.join("template.t.md"), template)
		.unwrap_or_else(|e| panic!("write template: {e}"));
	std::fs::write(root.join("readme.md"), readme).unwrap_or_else(|e| panic!("write readme: {e}"));
}

// ===========================================================================
// check: block arguments
// ===========================================================================

#[tokio::test]
async fn check_with_block_arguments_detects_stale() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_args_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.check(Parameters(CheckParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..CheckParam::default()
		}))
		.await;

	let text = extract_text(&result);
	assert!(
		text.contains("stale"),
		"expected stale message, got: {text}"
	);
	assert!(
		text.contains("badges"),
		"expected block name in message, got: {text}"
	);
}

#[tokio::test]
async fn check_reports_argument_count_mismatch() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_args_mismatch_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.check(Parameters(CheckParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			..CheckParam::default()
		}))
		.await;

	let text = extract_text(&result);
	assert!(
		text.contains("argument count mismatch"),
		"expected argument count mismatch message, got: {text}"
	);
}

// ===========================================================================
// update: block arguments
// ===========================================================================

#[tokio::test]
async fn update_with_block_arguments_applies_changes() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_args_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.update(Parameters(UpdateParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			dry_run: false,
			..UpdateParam::default()
		}))
		.await;

	let text = extract_text(&result);
	assert!(
		text.contains("Updated"),
		"expected update confirmation, got: {text}"
	);

	// Verify the file was written with the rendered argument value.
	let readme_content = std::fs::read_to_string(tmp.path().join("readme.md"))
		.unwrap_or_else(|e| panic!("read readme: {e}"));
	assert!(
		readme_content.contains("mdt_core"),
		"consumer should contain rendered argument value 'mdt_core'"
	);
	assert!(
		!readme_content.contains("{{ crate_name }}"),
		"template variable should be interpolated"
	);
	assert!(
		!readme_content.contains("Old stale content"),
		"old stale content should be replaced"
	);
}

// ===========================================================================
// preview: block arguments
// ===========================================================================

#[tokio::test]
async fn preview_with_block_arguments_shows_provider_template() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_args_project(tmp.path());

	let server = MdtMcpServer::with_base_root(tmp.path());
	let result = server
		.preview(Parameters(BlockParam {
			path: Some(tmp.path().to_string_lossy().to_string()),
			block_name: "badges".to_string(),
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["provider"]["name"], "badges");
	assert_eq!(json["consumers"][0]["file"], "readme.md");
	assert!(
		json["consumers"][0]["rendered_content"]
			.as_str()
			.is_some_and(|value| value.contains("mdt_core")),
		"expected argument-aware consumer preview, got: {json}"
	);
}

// ===========================================================================
// path confinement
// ===========================================================================

#[tokio::test]
async fn tools_reject_paths_outside_server_root() {
	let base = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let outside = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(outside.path());

	let server = MdtMcpServer::with_base_root(base.path());
	let outside_path = outside.path().to_string_lossy().to_string();

	for (action, result) in call_project_tools(&server, &outside_path).await {
		let json = extract_json(&result);
		assert_eq!(result.is_error, Some(true), "{action}: {json}");
		assert_eq!(json["error"]["code"], "mdt::path_outside_root", "{action}");
		assert!(
			json["summary"]
				.as_str()
				.is_some_and(|summary| summary.contains("outside the mdt MCP server root")),
			"{action}: expected confinement error, got: {json}"
		);
	}
	let init = server
		.init(Parameters(InitParam {
			path: Some(outside_path),
		}))
		.await;
	assert_eq!(
		extract_json(&init)["error"]["code"],
		"mdt::path_outside_root"
	);
	assert!(
		!outside.path().join("mdt.toml").exists(),
		"init wrote outside the server root"
	);
}

#[tokio::test]
async fn tools_reject_parent_escape_paths() {
	let base = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let nested = base.path().join("docs");
	std::fs::create_dir_all(&nested).unwrap_or_else(|e| panic!("mkdir: {e}"));

	let server = MdtMcpServer::with_base_root(&nested);
	let result = server
		.init(Parameters(InitParam {
			path: Some("../elsewhere".to_string()),
		}))
		.await;

	assert_eq!(
		extract_json(&result)["error"]["code"],
		"mdt::path_outside_root"
	);
	assert!(!base.path().join("elsewhere").exists());
}

#[tokio::test]
async fn tools_accept_parent_segments_that_stay_inside_the_root() {
	let base = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::create_dir_all(base.path().join("docs")).unwrap_or_else(|e| panic!("mkdir: {e}"));
	create_synced_project(base.path());

	let server = MdtMcpServer::with_base_root(base.path());
	let result = server
		.check(Parameters(CheckParam {
			path: Some("docs/../.".to_string()),
			..CheckParam::default()
		}))
		.await;

	assert_eq!(extract_json(&result)["ok"], true);
}

#[cfg(unix)]
#[tokio::test]
async fn tools_reject_symlinks_that_leave_the_root() {
	let base = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let outside = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(outside.path());
	std::os::unix::fs::symlink(outside.path(), base.path().join("esc"))
		.unwrap_or_else(|e| panic!("symlink: {e}"));
	let server = MdtMcpServer::with_base_root(base.path());

	for (action, result) in call_project_tools(&server, "esc").await {
		assert_eq!(
			extract_json(&result)["error"]["code"],
			"mdt::path_outside_root",
			"{action}"
		);
	}
	let readme = std::fs::read_to_string(outside.path().join("readme.md"))
		.unwrap_or_else(|e| panic!("read readme: {e}"));
	assert!(
		readme.contains("Old stale content."),
		"update wrote outside"
	);
}

#[cfg(unix)]
#[tokio::test]
async fn init_rejects_a_dangling_symlink_to_a_path_outside_the_root() {
	let base = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let outside = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let target = outside.path().join("not-yet");
	std::os::unix::fs::symlink(&target, base.path().join("dangling"))
		.unwrap_or_else(|e| panic!("symlink: {e}"));
	let server = MdtMcpServer::with_base_root(base.path());

	let result = server
		.init(Parameters(InitParam {
			path: Some("dangling/sub".to_string()),
		}))
		.await;

	assert_eq!(result.is_error, Some(true));
	assert_eq!(
		extract_json(&result)["error"]["code"],
		"mdt::path_unresolvable"
	);
	assert!(
		!target.exists(),
		"init created a directory outside the root"
	);
}

#[tokio::test]
async fn tools_accept_subdirectory_inside_server_root() {
	let base = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let nested = base.path().join("docs");
	std::fs::create_dir_all(&nested).unwrap_or_else(|e| panic!("mkdir: {e}"));
	create_synced_project(&nested);

	let server = MdtMcpServer::with_base_root(base.path());
	let result = server
		.check(Parameters(CheckParam {
			path: Some(nested.to_string_lossy().to_string()),
			..CheckParam::default()
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["ok"], true);
}

#[tokio::test]
async fn tools_default_to_server_root() {
	let base = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_synced_project(base.path());

	let server = MdtMcpServer::with_base_root(base.path());
	let result = server.check(Parameters(CheckParam::default())).await;

	let json = extract_json(&result);
	assert_eq!(json["ok"], true);
}

// ===========================================================================
// Regressions from the MCP evaluation
// ===========================================================================

fn create_padded_synced_project(root: &Path) {
	std::fs::write(root.join("mdt.toml"), "[padding]\nbefore = 1\nafter = 1\n")
		.unwrap_or_else(|e| panic!("write config: {e}"));
	std::fs::write(
		root.join("template.t.md"),
		"<!-- {@greeting} -->\nHello from mdt!\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write template: {e}"));
	std::fs::write(
		root.join("readme.md"),
		"<!-- {=greeting} -->\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write readme: {e}"));
	let ctx =
		mdt_core::project::scan_project_with_config(root).unwrap_or_else(|e| panic!("scan: {e}"));
	let updates = mdt_core::compute_updates(&ctx).unwrap_or_else(|e| panic!("updates: {e}"));
	mdt_core::write_updates(&updates).unwrap_or_else(|e| panic!("write updates: {e}"));
	let ctx =
		mdt_core::project::scan_project_with_config(root).unwrap_or_else(|e| panic!("rescan: {e}"));
	let check = mdt_core::check_project(&ctx).unwrap_or_else(|e| panic!("check: {e}"));
	assert!(check.is_ok(), "fixture must pass `mdt check`");
}

#[tokio::test]
async fn regression_list_agrees_with_check_under_padding() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_padded_synced_project(tmp.path());
	let server = MdtMcpServer::with_base_root(tmp.path());

	let result = server.list(Parameters(ListParam::default())).await;

	let json = extract_json(&result);
	assert_eq!(json["consumers"][0]["is_stale"], false, "got: {json}");
}

#[tokio::test]
async fn regression_check_reports_validation_errors() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_synced_project(tmp.path());
	std::fs::write(
		tmp.path().join("notes.md"),
		"<!-- {=greeting} -->\n\nnever closed\n",
	)
	.unwrap_or_else(|e| panic!("write notes: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	let result = server.check(Parameters(CheckParam::default())).await;

	let json = extract_json(&result);
	assert_eq!(json["ok"], false, "got: {json}");
	assert_eq!(
		json["diagnostics"][0]["kind"], "unclosed_block",
		"got: {json}"
	);
}

#[tokio::test]
async fn regression_check_reports_orphans() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_synced_project(tmp.path());
	std::fs::write(
		tmp.path().join("notes.md"),
		"<!-- {=greetin} -->\n\nx\n\n<!-- {/greetin} -->\n",
	)
	.unwrap_or_else(|e| panic!("write notes: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	let result = server.check(Parameters(CheckParam::default())).await;

	let json = extract_json(&result);
	assert_eq!(json["orphans"][0]["block_name"], "greetin", "got: {json}");
	assert_eq!(
		json["orphans"][0]["suggestions"][0], "greeting",
		"got: {json}"
	);
}

#[tokio::test]
async fn regression_init_next_steps_use_single_braces() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("README.md"), "# Existing\n")
		.unwrap_or_else(|e| panic!("write readme: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	let result = server.init(Parameters(InitParam { path: None })).await;

	let text = serde_json::to_string(&extract_json(&result)["next_steps"])
		.unwrap_or_else(|e| panic!("serialize: {e}"));
	assert!(!text.contains("{{"), "doubled braces in next steps: {text}");
	assert!(text.contains("<!-- {=greeting} -->"), "got: {text}");
}

#[cfg(unix)]
#[tokio::test]
async fn regression_init_rejects_symlink_escape_to_missing_path() {
	let base = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let outside = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::os::unix::fs::symlink(outside.path(), base.path().join("esc"))
		.unwrap_or_else(|e| panic!("symlink: {e}"));
	let server = MdtMcpServer::with_base_root(base.path());

	let _ = server
		.init(Parameters(InitParam {
			path: Some("esc/sub".to_string()),
		}))
		.await;

	assert!(
		!outside.path().join("sub").exists(),
		"init wrote outside the server root"
	);
}

#[tokio::test]
async fn regression_update_reports_render_errors() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npkg = \"package.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write config: {e}"));
	std::fs::write(tmp.path().join("package.json"), r#"{"name": "x"}"#)
		.unwrap_or_else(|e| panic!("write package: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@broken} -->\n\n{{ pkg.name \n\n<!-- {/broken} -->\n",
	)
	.unwrap_or_else(|e| panic!("write template: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=broken} -->\n\nold\n\n<!-- {/broken} -->\n",
	)
	.unwrap_or_else(|e| panic!("write readme: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	let result = server
		.update(Parameters(UpdateParam {
			path: None,
			dry_run: true,
			..UpdateParam::default()
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["ok"], false, "got: {json}");
	assert_eq!(
		json["render_errors"][0]["block_name"], "broken",
		"got: {json}"
	);
}

#[tokio::test]
async fn regression_get_block_consumer_lookup_returns_object() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=orphan} -->\n\nx\n\n<!-- {/orphan} -->\n",
	)
	.unwrap_or_else(|e| panic!("write readme: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	let result = server
		.get_block(Parameters(BlockParam {
			path: None,
			block_name: "orphan".to_string(),
		}))
		.await;

	assert!(
		result
			.structured_content
			.as_ref()
			.is_some_and(serde_json::Value::is_object),
		"structured content must be an object: {:?}",
		result.structured_content
	);
}

#[tokio::test]
async fn regression_invalid_config_is_an_error_result() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(tmp.path().join("mdt.toml"), "[data\nbad toml")
		.unwrap_or_else(|e| panic!("write: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	let result = server.check(Parameters(CheckParam::default())).await;

	assert_eq!(result.is_error, Some(true));
	assert_eq!(extract_json(&result)["error"]["code"], "mdt::config_parse");
}

#[tokio::test]
async fn regression_missing_path_is_an_error_result() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	let result = server
		.check(Parameters(CheckParam {
			path: Some("does-not-exist".to_string()),
			..CheckParam::default()
		}))
		.await;

	assert_eq!(
		result.is_error,
		Some(true),
		"got: {:?}",
		extract_json(&result)
	);
}

#[test]
fn regression_server_info_names_mdt() {
	let info = MdtMcpServer::new().get_info();
	assert_eq!(info.server_info.name, "mdt");
	assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn regression_find_reuse_ranks_normalized_names_and_drops_unrelated() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@install} -->\na\n<!-- {/install} -->\n<!-- {@installGuide} -->\nb\n<!-- \
		 {/installGuide} -->\n<!-- {@badges} -->\nc\n<!-- {/badges} -->\n",
	)
	.unwrap_or_else(|e| panic!("write template: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	let result = server
		.find_reuse(Parameters(ReuseParam {
			path: None,
			block_name: Some("install-guide".to_string()),
			limit: 5,
			..ReuseParam::default()
		}))
		.await;

	let json = extract_json(&result);
	assert_eq!(json["candidates"][0]["name"], "installGuide", "got: {json}");
	let names: Vec<_> = json["candidates"]
		.as_array()
		.unwrap_or_else(|| panic!("candidates"))
		.iter()
		.map(|candidate| candidate["name"].clone())
		.collect();
	assert!(!names.contains(&serde_json::json!("badges")), "got: {json}");
}

#[tokio::test]
async fn regression_list_omits_provider_content_by_default() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_synced_project(tmp.path());
	let server = MdtMcpServer::with_base_root(tmp.path());

	let result = server.list(Parameters(ListParam::default())).await;

	let json = extract_json(&result);
	assert!(json["providers"][0].get("content").is_none(), "got: {json}");
}

#[test]
fn regression_tool_schemas_and_annotations() {
	let tools = MdtMcpServer::new().tool_router.list_all();
	let reuse = tools
		.iter()
		.find(|tool| tool.name == "mdt_find_reuse")
		.unwrap_or_else(|| panic!("mdt_find_reuse"));
	let limit = &reuse.input_schema["properties"]["limit"];
	assert_eq!(limit["minimum"], 1, "limit schema: {limit}");
	assert_eq!(limit["maximum"], 20, "limit schema: {limit}");
	let check = tools
		.iter()
		.find(|tool| tool.name == "mdt_check")
		.unwrap_or_else(|| panic!("mdt_check"));
	assert_eq!(
		check.annotations.as_ref().and_then(|a| a.read_only_hint),
		Some(true)
	);
}

// ===========================================================================
// Server startup
// ===========================================================================

#[test]
fn init_tracing_tolerates_an_existing_global_subscriber() {
	// `MDT_LOG=info mdt mcp`: the CLI installs its subscriber first.
	let _cli_subscriber = fmt().with_writer(std::io::sink).try_init();

	init_tracing();
}

#[tokio::test]
async fn serve_in_identifies_as_mdt_and_serves_the_given_root() {
	use tokio::io::AsyncBufReadExt as _;
	use tokio::io::AsyncWriteExt as _;

	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_stale_project(tmp.path());
	let (client, server_io) = tokio::io::duplex(1 << 16);
	let server = tokio::spawn(serve_in(
		tmp.path().to_path_buf(),
		tokio::io::split(server_io),
	));
	let (client_read, mut client_write) = tokio::io::split(client);
	let mut lines = tokio::io::BufReader::new(client_read).lines();

	let messages = [
		serde_json::json!({
			"jsonrpc": "2.0", "id": 1, "method": "initialize",
			"params": {
				"protocolVersion": "2025-06-18",
				"capabilities": {},
				"clientInfo": { "name": "test", "version": "0" },
			},
		}),
		serde_json::json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
		serde_json::json!({
			"jsonrpc": "2.0", "id": 2, "method": "tools/call",
			"params": { "name": "mdt_check", "arguments": {} },
		}),
	];
	let mut responses = Vec::new();
	for message in messages {
		let mut line = message.to_string();
		line.push('\n');
		client_write
			.write_all(line.as_bytes())
			.await
			.unwrap_or_else(|e| panic!("write: {e}"));
		if message.get("id").is_none() {
			continue;
		}
		let response = tokio::time::timeout(std::time::Duration::from_secs(10), lines.next_line())
			.await
			.unwrap_or_else(|e| panic!("timed out waiting for a response: {e}"))
			.unwrap_or_else(|e| panic!("read: {e}"))
			.unwrap_or_else(|| panic!("server closed the stream"));
		responses.push(
			serde_json::from_str::<serde_json::Value>(&response)
				.unwrap_or_else(|e| panic!("invalid JSON-RPC response: {e}")),
		);
	}
	client_write
		.shutdown()
		.await
		.unwrap_or_else(|e| panic!("shutdown: {e}"));
	tokio::time::timeout(std::time::Duration::from_secs(10), server)
		.await
		.unwrap_or_else(|e| panic!("server did not stop after the client left: {e}"))
		.unwrap_or_else(|e| panic!("server task failed: {e}"));

	let server_info = &responses[0]["result"]["serverInfo"];
	assert_eq!(server_info["name"], "mdt");
	assert_eq!(server_info["version"], env!("CARGO_PKG_VERSION"));
	let check = &responses[1]["result"]["structuredContent"];
	assert_eq!(check["ok"], false, "got: {check}");
	assert_eq!(check["stale"][0]["file"], "readme.md", "got: {check}");
}

#[test]
fn server_instructions_point_agents_at_the_skill() {
	let instructions = MdtMcpServer::new()
		.get_info()
		.instructions
		.unwrap_or_else(|| panic!("expected instructions"));

	assert!(instructions.contains("`mdt skill`"), "got: {instructions}");
	assert!(instructions.contains("--reference"), "got: {instructions}");
}

#[test]
fn tool_annotations_mark_only_update_and_init_as_writing() {
	let tools = MdtMcpServer::new().tool_router.list_all();
	let read_only = |name: &str| {
		tools
			.iter()
			.find(|tool| tool.name == name)
			.unwrap_or_else(|| panic!("missing tool {name}"))
			.annotations
			.as_ref()
			.and_then(|annotations| annotations.read_only_hint)
	};

	for name in [
		"mdt_check",
		"mdt_list",
		"mdt_find_reuse",
		"mdt_get_block",
		"mdt_preview",
	] {
		assert_eq!(read_only(name), Some(true), "{name}");
	}
	for name in ["mdt_update", "mdt_init"] {
		assert_eq!(read_only(name), Some(false), "{name}");
	}
}

#[test]
fn tool_schemas_expose_validation_and_content_options() {
	let tools = MdtMcpServer::new().tool_router.list_all();
	let properties = |name: &str| {
		tools
			.iter()
			.find(|tool| tool.name == name)
			.unwrap_or_else(|| panic!("missing tool {name}"))
			.input_schema["properties"]
			.clone()
	};

	for name in ["mdt_check", "mdt_update", "mdt_list"] {
		let properties = properties(name);
		for option in [
			"path",
			"ignore_unclosed_blocks",
			"ignore_unused_blocks",
			"ignore_invalid_names",
			"ignore_invalid_transformers",
		] {
			assert!(
				properties.get(option).is_some(),
				"{name} is missing `{option}`: {properties}"
			);
		}
	}
	assert!(properties("mdt_list").get("include_content").is_some());
	assert!(properties("mdt_find_reuse").get("content_query").is_some());
}

// ===========================================================================
// Response envelope
// ===========================================================================

#[tokio::test]
async fn every_tool_response_carries_ok_action_and_summary() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_synced_project(tmp.path());
	let server = MdtMcpServer::with_base_root(tmp.path());

	let mut results = call_project_tools(&server, ".").await;
	results.push(("init", server.init(Parameters(InitParam::default())).await));

	for (action, result) in results {
		let json = extract_json(&result);
		assert!(json.is_object(), "{action}: {json}");
		assert!(json["ok"].is_boolean(), "{action}: {json}");
		assert_eq!(json["action"], action);
		assert!(json["summary"].is_string(), "{action}: {json}");
		assert_eq!(
			serde_json::from_str::<serde_json::Value>(extract_text(&result))
				.unwrap_or_else(|e| panic!("{action}: text is not JSON: {e}")),
			json,
			"{action}: text and structured content must match"
		);
	}
}

// ===========================================================================
// Validation diagnostics
// ===========================================================================

fn create_unclosed_block_project(root: &Path) {
	create_stale_project(root);
	std::fs::write(
		root.join("notes.md"),
		"<!-- {=greeting} -->\n\nnever closed\n",
	)
	.unwrap_or_else(|e| panic!("write notes: {e}"));
}

#[tokio::test]
async fn check_reports_diagnostic_details() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_unclosed_block_project(tmp.path());
	let server = MdtMcpServer::with_base_root(tmp.path());

	let json = extract_json(&server.check(Parameters(CheckParam::default())).await);

	assert_eq!(
		json["diagnostics"],
		serde_json::json!([{
			"kind": "unclosed_block",
			"severity": "error",
			"file": "notes.md",
			"line": 1,
			"column": 1,
			"message": "missing closing tag for block `greeting`",
		}])
	);
	assert!(
		json["summary"]
			.as_str()
			.is_some_and(|summary| summary.contains("1 validation error(s)")),
		"got: {json}"
	);
}

#[tokio::test]
async fn check_ignore_flags_silence_diagnostics() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_synced_project(tmp.path());
	std::fs::write(
		tmp.path().join("notes.md"),
		"<!-- {=greeting} -->\n\nnever closed\n",
	)
	.unwrap_or_else(|e| panic!("write notes: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	let json = extract_json(
		&server
			.check(Parameters(CheckParam {
				validation: ValidationParam {
					ignore_unclosed_blocks: true,
					..ValidationParam::default()
				},
				..CheckParam::default()
			}))
			.await,
	);

	assert_eq!(json["ok"], true, "got: {json}");
	assert_eq!(json["diagnostics"], serde_json::json!([]));
}

#[tokio::test]
async fn unused_providers_are_warnings_that_keep_check_green() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_synced_project(tmp.path());
	std::fs::write(
		tmp.path().join("extra.t.md"),
		"<!-- {@lonely} -->\n\nNobody uses me.\n\n<!-- {/lonely} -->\n",
	)
	.unwrap_or_else(|e| panic!("write: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	let json = extract_json(&server.check(Parameters(CheckParam::default())).await);
	let ignored = extract_json(
		&server
			.check(Parameters(CheckParam {
				validation: ValidationParam {
					ignore_unused_blocks: true,
					..ValidationParam::default()
				},
				..CheckParam::default()
			}))
			.await,
	);

	assert_eq!(json["ok"], true, "got: {json}");
	assert_eq!(json["diagnostics"][0]["kind"], "unused_provider");
	assert_eq!(json["diagnostics"][0]["severity"], "warning");
	assert_eq!(ignored["diagnostics"], serde_json::json!([]));
}

#[tokio::test]
async fn update_refuses_to_write_when_validation_fails() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_unclosed_block_project(tmp.path());
	let server = MdtMcpServer::with_base_root(tmp.path());

	let result = server.update(Parameters(UpdateParam::default())).await;

	assert_eq!(result.is_error, Some(true));
	let json = extract_json(&result);
	assert_eq!(json["ok"], false);
	assert_eq!(json["error"]["code"], "mdt::validation");
	assert_eq!(json["updated_count"], 0);
	assert_eq!(json["updated_files"], serde_json::json!([]));
	assert_eq!(json["diagnostics"][0]["kind"], "unclosed_block");
	let readme = std::fs::read_to_string(tmp.path().join("readme.md"))
		.unwrap_or_else(|e| panic!("read readme: {e}"));
	assert!(
		readme.contains("Old stale content."),
		"update must not write"
	);
}

#[tokio::test]
async fn update_writes_when_the_validation_error_is_ignored() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_unclosed_block_project(tmp.path());
	let server = MdtMcpServer::with_base_root(tmp.path());

	let json = extract_json(
		&server
			.update(Parameters(UpdateParam {
				validation: ValidationParam {
					ignore_unclosed_blocks: true,
					..ValidationParam::default()
				},
				..UpdateParam::default()
			}))
			.await,
	);

	assert_eq!(json["ok"], true, "got: {json}");
	assert_eq!(json["updated_files"], serde_json::json!(["readme.md"]));
}

#[tokio::test]
async fn list_reports_validation_errors() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_unclosed_block_project(tmp.path());
	let server = MdtMcpServer::with_base_root(tmp.path());

	let json = extract_json(&server.list(Parameters(ListParam::default())).await);

	assert_eq!(json["ok"], false);
	assert_eq!(json["action"], "list");
	assert_eq!(json["diagnostics"][0]["severity"], "error");
	assert!(
		json["summary"]
			.as_str()
			.is_some_and(|summary| summary.contains("1 validation error(s)")),
		"got: {json}"
	);
}

// ===========================================================================
// Render errors
// ===========================================================================

/// A project where `broken` fails to render and `greeting` is stale.
fn create_render_error_project(root: &Path) {
	std::fs::write(root.join("mdt.toml"), "[data]\npkg = \"package.json\"\n")
		.unwrap_or_else(|e| panic!("write config: {e}"));
	std::fs::write(root.join("package.json"), r#"{"name": "x"}"#)
		.unwrap_or_else(|e| panic!("write package: {e}"));
	std::fs::write(
		root.join("template.t.md"),
		"<!-- {@broken} -->\n\n{{ pkg.name \n\n<!-- {/broken} -->\n\n<!-- {@greeting} \
		 -->\n\nHello from mdt!\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write template: {e}"));
	std::fs::write(
		root.join("readme.md"),
		"<!-- {=broken} -->\n\nold\n\n<!-- {/broken} -->\n\n<!-- {=greeting} -->\n\nold\n\n<!-- \
		 {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write readme: {e}"));
}

#[tokio::test]
async fn update_syncs_what_renders_and_reports_the_rest() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_render_error_project(tmp.path());
	let server = MdtMcpServer::with_base_root(tmp.path());

	let result = server.update(Parameters(UpdateParam::default())).await;

	assert_ne!(result.is_error, Some(true), "render errors are a result");
	let json = extract_json(&result);
	assert_eq!(json["ok"], false);
	assert_eq!(json["updated_count"], 1);
	let error = &json["render_errors"][0];
	assert_eq!(error["block_name"], "broken");
	assert_eq!(error["file"], "readme.md");
	assert_eq!(error["line"], 1);
	assert_eq!(error["column"], 1);
	let message = error["message"].as_str().unwrap_or_default();
	assert!(
		!message.contains("template rendering failed"),
		"no doubled prefix: {message}"
	);
	let readme = std::fs::read_to_string(tmp.path().join("readme.md"))
		.unwrap_or_else(|e| panic!("read readme: {e}"));
	assert!(readme.contains("Hello from mdt!"), "greeting is synced");
	assert!(readme.contains("\nold\n"), "broken is left unchanged");
}

#[tokio::test]
async fn get_block_reports_a_provider_that_fails_to_render() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_render_error_project(tmp.path());
	let server = MdtMcpServer::with_base_root(tmp.path());

	let json = extract_json(
		&server
			.get_block(Parameters(BlockParam {
				path: None,
				block_name: "broken".to_string(),
			}))
			.await,
	);

	assert_eq!(json["ok"], false, "got: {json}");
	assert!(json["provider"]["rendered_with_project_data"].is_null());
	assert!(json["provider"]["render_error"].is_string(), "got: {json}");
	assert_eq!(json["consumers"][0]["status"], "render_error");
	assert!(json["consumers"][0]["render_error"].is_string());
}

#[tokio::test]
async fn preview_reports_a_provider_that_fails_to_render() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_render_error_project(tmp.path());
	let server = MdtMcpServer::with_base_root(tmp.path());

	let json = extract_json(
		&server
			.preview(Parameters(BlockParam {
				path: None,
				block_name: "broken".to_string(),
			}))
			.await,
	);

	assert_eq!(json["ok"], false, "got: {json}");
	assert!(json["provider"]["render_error"].is_string(), "got: {json}");
	let consumer = &json["consumers"][0];
	assert!(consumer["rendered_content"].is_null(), "got: {json}");
	assert_eq!(consumer["status"], "render_error");
	assert_eq!(consumer["is_stale"], false);
}

// ===========================================================================
// Staleness agrees with `mdt check`
// ===========================================================================

#[tokio::test]
async fn get_block_and_preview_agree_with_check_under_padding() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_padded_synced_project(tmp.path());
	let server = MdtMcpServer::with_base_root(tmp.path());
	let block = || {
		Parameters(BlockParam {
			path: None,
			block_name: "greeting".to_string(),
		})
	};

	let get_block = extract_json(&server.get_block(block()).await);
	let preview = extract_json(&server.preview(block()).await);

	assert_eq!(get_block["ok"], true, "got: {get_block}");
	assert_eq!(get_block["consumers"][0]["status"], "current");
	assert_eq!(preview["ok"], true, "got: {preview}");
	let consumer = &preview["consumers"][0];
	assert_eq!(consumer["status"], "current");
	assert_eq!(consumer["is_stale"], false);
	assert_eq!(
		consumer["rendered_content"], consumer["current_content"],
		"preview must render exactly what `mdt update` wrote"
	);
}

#[tokio::test]
async fn list_reports_consumer_type_location_and_status() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("mdt.toml"),
		"[data]\npkg = \"package.json\"\n",
	)
	.unwrap_or_else(|e| panic!("write config: {e}"));
	std::fs::write(tmp.path().join("package.json"), r#"{"version": "2.0.0"}"#)
		.unwrap_or_else(|e| panic!("write package: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@greeting} -->\n\nHello from mdt!\n\n<!-- {/greeting} -->\n",
	)
	.unwrap_or_else(|e| panic!("write template: {e}"));
	std::fs::write(
		tmp.path().join("readme.md"),
		"# Title\n\nVersion <!-- {~version:\"{{ pkg.version }}\"} -->1.0.0<!-- {/version} \
		 -->\n\n<!-- {=greeting} -->\n\nHello from mdt!\n\n<!-- {/greeting} -->\n\n<!-- \
		 {=missing} -->\n\nx\n\n<!-- {/missing} -->\n",
	)
	.unwrap_or_else(|e| panic!("write readme: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	let json = extract_json(&server.list(Parameters(ListParam::default())).await);

	let consumers = &json["consumers"];
	assert_eq!(consumers[0]["type"], "inline", "got: {json}");
	assert_eq!(consumers[0]["name"], "version");
	assert_eq!(consumers[0]["line"], 3);
	assert_eq!(consumers[0]["column"], 9);
	assert_eq!(consumers[0]["status"], "stale");
	assert_eq!(consumers[0]["is_stale"], true);
	assert_eq!(consumers[1]["type"], "consumer");
	assert_eq!(consumers[1]["status"], "current");
	assert_eq!(consumers[2]["name"], "missing");
	assert_eq!(consumers[2]["status"], "orphan");
	assert_eq!(json["providers"][0]["line"], 1);
	assert!(
		json["summary"]
			.as_str()
			.is_some_and(|summary| summary.contains("1 stale")),
		"got: {json}"
	);
}

// ===========================================================================
// Reuse ranking
// ===========================================================================

fn provider_names(json: &serde_json::Value) -> Vec<String> {
	json["candidates"]
		.as_array()
		.unwrap_or_else(|| panic!("candidates should be array: {json}"))
		.iter()
		.map(|candidate| candidate["name"].as_str().unwrap_or_default().to_string())
		.collect()
}

#[tokio::test]
async fn find_reuse_ranks_by_match_kind() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let template: String = [
		"badges",
		"instalGuide",
		"fullInstallGuide",
		"installGuideExtended",
		"installGuide",
		"install-guide",
	]
	.iter()
	.map(|name| format!("<!-- {{@{name}}} -->\n{name}\n<!-- {{/{name}}} -->\n"))
	.collect::<Vec<_>>()
	.concat();
	std::fs::write(tmp.path().join("template.t.md"), template)
		.unwrap_or_else(|e| panic!("write template: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	let json = extract_json(
		&server
			.find_reuse(Parameters(ReuseParam {
				block_name: Some("install-guide".to_string()),
				limit: 20,
				..ReuseParam::default()
			}))
			.await,
	);

	assert_eq!(
		provider_names(&json),
		[
			"install-guide",
			"installGuide",
			"installGuideExtended",
			"fullInstallGuide",
			"instalGuide",
		]
	);
	let kinds: Vec<_> = json["candidates"]
		.as_array()
		.unwrap_or_else(|| panic!("candidates"))
		.iter()
		.map(|candidate| candidate["match"].clone())
		.collect();
	assert_eq!(
		kinds,
		["exact", "normalized", "prefix", "substring", "similar"]
	);
	assert_eq!(json["ok"], true);
	assert_eq!(json["action"], "find_reuse");
}

#[tokio::test]
async fn find_reuse_matches_provider_content() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@setup} -->\nRun npm install mdt.\n<!-- {/setup} -->\n<!-- {@other} \
		 -->\nUnrelated.\n<!-- {/other} -->\n",
	)
	.unwrap_or_else(|e| panic!("write template: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());

	let json = extract_json(
		&server
			.find_reuse(Parameters(ReuseParam {
				content_query: Some("NPM INSTALL".to_string()),
				..ReuseParam::default()
			}))
			.await,
	);

	assert_eq!(provider_names(&json), ["setup"]);
	assert_eq!(json["candidates"][0]["match"], "content");
	assert_eq!(json["content_query"], "NPM INSTALL");
}

#[tokio::test]
async fn find_reuse_without_a_match_suggests_a_new_provider() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	create_synced_project(tmp.path());
	let server = MdtMcpServer::with_base_root(tmp.path());

	let json = extract_json(
		&server
			.find_reuse(Parameters(ReuseParam {
				block_name: Some("changelogFooter".to_string()),
				..ReuseParam::default()
			}))
			.await,
	);

	assert_eq!(json["candidates"], serde_json::json!([]));
	assert!(
		json["summary"]
			.as_str()
			.is_some_and(|summary| summary.contains("create a new provider")),
		"got: {json}"
	);
}

#[tokio::test]
async fn find_reuse_clamps_limit_to_the_schema_range() {
	let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
	let template: String = (0..25)
		.map(|index| format!("<!-- {{@block{index}}} -->\nx\n<!-- {{/block{index}}} -->\n"))
		.collect::<Vec<_>>()
		.concat();
	std::fs::write(tmp.path().join("template.t.md"), template)
		.unwrap_or_else(|e| panic!("write template: {e}"));
	let server = MdtMcpServer::with_base_root(tmp.path());
	let candidates = |limit| {
		let server = &server;
		async move {
			extract_json(
				&server
					.find_reuse(Parameters(ReuseParam {
						limit,
						..ReuseParam::default()
					}))
					.await,
			)["candidates"]
				.as_array()
				.map_or(0, Vec::len)
		}
	};

	assert_eq!(candidates(0).await, 1);
	assert_eq!(candidates(100).await, 20);
}

#[test]
fn match_name_normalizes_case_and_separators() {
	use crate::reuse::MatchKind;
	use crate::reuse::match_name;

	let kind = |query, name| match_name(query, name).map(|matched| matched.kind);
	assert_eq!(kind("install", "install"), Some(MatchKind::Exact));
	assert_eq!(
		kind("install_guide", "InstallGuide"),
		Some(MatchKind::Normalized)
	);
	assert_eq!(kind("install", "installGuide"), Some(MatchKind::Prefix));
	assert_eq!(kind("guide", "installGuide"), Some(MatchKind::Substring));
	assert_eq!(kind("installGuide", "guide"), Some(MatchKind::Substring));
	assert_eq!(kind("greting", "greeting"), Some(MatchKind::Similar));
	assert_eq!(kind("greeting", "badges"), None);
	assert_eq!(
		kind("-", "badges"),
		None,
		"a separator-only query matches nothing"
	);
}

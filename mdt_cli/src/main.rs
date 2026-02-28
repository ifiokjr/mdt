use std::collections::BTreeSet;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::process;
use std::sync::mpsc;
use std::time::Duration;

use clap::Parser;
use mdt_cli::Commands;
use mdt_cli::DoctorOutputFormat;
use mdt_cli::InfoOutputFormat;
use mdt_cli::MdtCli;
use mdt_cli::OutputFormat;
use mdt_core::BlockType;
use mdt_core::MdtConfig;
use mdt_core::MdtError;
use mdt_core::TemplateWarning;
use mdt_core::check_project;
use mdt_core::compute_updates;
use mdt_core::project::ConsumerEntry;
use mdt_core::project::DiagnosticKind;
use mdt_core::project::ProjectContext;
use mdt_core::project::ProjectDiagnostic;
use mdt_core::project::ProviderEntry;
use mdt_core::project::ScanOptions;
use mdt_core::project::ValidationOptions;
use mdt_core::project::inspect_project_cache;
use mdt_core::project::scan_project_with_config;
use mdt_core::write_updates;
use owo_colors::OwoColorize;
use similar::ChangeTag;
use similar::TextDiff;

static USE_COLOR: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

fn color_enabled() -> bool {
	USE_COLOR.load(std::sync::atomic::Ordering::Relaxed)
}

/// Apply ANSI color codes only when color is enabled.
macro_rules! colored {
	($text:expr,red) => {
		if color_enabled() {
			format!("{}", $text.red())
		} else {
			format!("{}", $text)
		}
	};
	($text:expr,green) => {
		if color_enabled() {
			format!("{}", $text.green())
		} else {
			format!("{}", $text)
		}
	};
	($text:expr,yellow) => {
		if color_enabled() {
			format!("{}", $text.yellow())
		} else {
			format!("{}", $text)
		}
	};
	($text:expr,bold) => {
		if color_enabled() {
			format!("{}", $text.bold())
		} else {
			format!("{}", $text)
		}
	};
}

fn main() {
	let args = MdtCli::parse();

	// Respect NO_COLOR env var and --no-color flag.
	let use_color = !args.no_color && std::env::var_os("NO_COLOR").is_none();
	if !use_color {
		USE_COLOR.store(false, std::sync::atomic::Ordering::Relaxed);
	}

	// Install miette's fancy handler for rich error diagnostics.
	miette::set_hook(Box::new(move |_| {
		Box::new(
			miette::MietteHandlerOpts::new()
				.color(use_color)
				.unicode(use_color)
				.build(),
		)
	}))
	.ok();

	let result = match args.command {
		Some(Commands::Init) => run_init(&args),
		Some(Commands::Check {
			diff,
			format,
			watch,
		}) => run_check(&args, diff, format, watch),
		Some(Commands::Update { dry_run, watch }) => run_update(&args, dry_run, watch),
		Some(Commands::List) => run_list(&args),
		Some(Commands::Info { format }) => run_info(&args, format),
		Some(Commands::Doctor { format }) => run_doctor(&args, format),
		Some(Commands::Lsp) => run_lsp(),
		Some(Commands::Mcp) => run_mcp(),
		None => {
			eprintln!("No subcommand specified. Run `mdt --help` for usage.");
			process::exit(1);
		}
	};

	if let Err(e) = result {
		// Try to render through miette for rich diagnostics with help text
		// and error codes.
		match e.downcast::<MdtError>() {
			Ok(mdt_err) => {
				let report: miette::Report = (*mdt_err).into();
				eprintln!("{report:?}");
			}
			Err(e) => {
				eprintln!("{} {e}", colored!("error:", red));
			}
		}
		process::exit(2);
	}
}

fn resolve_root(args: &MdtCli) -> PathBuf {
	args.path
		.clone()
		.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn print_section(title: &str) {
	println!();
	println!("{}", colored!(title, bold));
}

fn print_field(label: &str, value: impl std::fmt::Display) {
	println!("{label:<28} {value}");
}

fn ratio_percent_string(numerator: u64, denominator: u64) -> String {
	if denominator == 0 {
		return "n/a".to_string();
	}

	let ratio = (numerator as f64 / denominator as f64) * 100.0;
	format!("{ratio:.1}%")
}

fn cache_hash_mode_hint(hash_verification_enabled: bool) -> String {
	if hash_verification_enabled {
		"unset `MDT_CACHE_VERIFY_HASH` to compare performance if cache reparses look too high"
			.to_string()
	} else {
		"set `MDT_CACHE_VERIFY_HASH=1` to validate cache keys with content hashes while \
		 troubleshooting"
			.to_string()
	}
}

fn run_init(args: &MdtCli) -> Result<(), Box<dyn std::error::Error>> {
	let root = resolve_root(args);
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

	if template_exists {
		println!("Template file already exists: {}", template_path.display());
	} else {
		let sample_content = "<!-- {@greeting} -->\n\nHello from mdt! This is a provider \
		                      block.\n\n<!-- {/greeting} -->\n";

		if let Some(parent) = template_path.parent() {
			std::fs::create_dir_all(parent)?;
		}
		std::fs::write(&template_path, sample_content)?;
		println!("Created template file: {}", template_path.display());
	}

	if config_exists {
		// Skip silently if config already exists.
	} else {
		let sample_config =
			"# mdt configuration\n# See \
			 https://ifiokjr.github.io/mdt/reference/configuration.html for full reference.\n\n# \
			 Map data files to template namespaces.\n# Values from these files are available in \
			 provider blocks as {{ namespace.key }}.\n# [data]\n# pkg = \"package.json\"\n# cargo \
			 = \"Cargo.toml\"\n# version = { command = \"cat VERSION\", format = \"text\", watch \
			 = [\"VERSION\"] }\n\n# Control blank lines between tags and content in source \
			 files.\n# Recommended when using formatters (rustfmt, prettier, etc.).\n# \
			 [padding]\n# before = 0\n# after = 0\n";

		std::fs::write(&config_path, sample_config)?;
		println!("Created mdt.toml");
	}

	if !template_exists {
		println!();
		println!("Next steps:");
		println!(
			"  1. Edit {} to define your template blocks",
			template_path.display()
		);
		println!("  2. Add consumer tags in your markdown files:");
		println!("     <!-- {{=greeting}} -->");
		println!("     <!-- {{/greeting}} -->");
		println!("  3. Run `mdt update` to sync content");
	}

	Ok(())
}

fn validation_options(args: &MdtCli) -> ValidationOptions {
	ValidationOptions {
		ignore_unclosed_blocks: args.ignore_unclosed_blocks,
		ignore_unused_blocks: args.ignore_unused_blocks,
		ignore_invalid_names: args.ignore_invalid_names,
		ignore_invalid_transformers: args.ignore_invalid_transformers,
	}
}

#[derive(Debug, Default)]
struct ConfigSummary {
	path: Option<PathBuf>,
	data_sources: Vec<DataSourceSummary>,
	template_dirs: Vec<PathBuf>,
}

#[derive(Debug)]
struct DataSourceSummary {
	namespace: String,
	location: String,
	kind: String,
	format: String,
	explicit_format: bool,
}

fn data_source_format(source: &mdt_core::DataSource) -> (String, bool) {
	if let Some(explicit) = source
		.format()
		.map(str::trim)
		.filter(|value| !value.is_empty())
	{
		return (explicit.to_ascii_lowercase(), true);
	}

	let inferred = match source {
		mdt_core::DataSource::Path(path) => {
			path.extension()
				.and_then(|ext| ext.to_str())
				.unwrap_or("unknown")
				.to_ascii_lowercase()
		}
		mdt_core::DataSource::Typed(typed) => {
			typed
				.path
				.extension()
				.and_then(|ext| ext.to_str())
				.unwrap_or("unknown")
				.to_ascii_lowercase()
		}
		mdt_core::DataSource::Script(_) => "text".to_string(),
		_ => "unknown".to_string(),
	};

	(inferred, false)
}

fn data_source_summary_fields(source: &mdt_core::DataSource) -> (String, String) {
	match source {
		mdt_core::DataSource::Path(path) => (path.display().to_string(), "file".to_string()),
		mdt_core::DataSource::Typed(typed) => {
			(typed.path.display().to_string(), "file".to_string())
		}
		mdt_core::DataSource::Script(script) => {
			(
				format!("script: {}", script.command),
				if script.watch.is_empty() {
					"script".to_string()
				} else {
					format!("script (watch: {})", script.watch.len())
				},
			)
		}
		_ => ("unknown".to_string(), "unknown".to_string()),
	}
}

fn load_config_summary(root: &Path) -> Result<ConfigSummary, Box<dyn std::error::Error>> {
	let config_path = MdtConfig::resolve_path(root);
	let config = MdtConfig::load(root)?;

	let Some(config) = config else {
		return Ok(ConfigSummary::default());
	};

	let mut data_sources: Vec<_> = config
		.data
		.into_iter()
		.map(|(namespace, source)| {
			let (format, explicit_format) = data_source_format(&source);
			let (location, kind) = data_source_summary_fields(&source);
			DataSourceSummary {
				namespace,
				location,
				kind,
				format,
				explicit_format,
			}
		})
		.collect();
	data_sources.sort_by(|a, b| {
		a.namespace
			.cmp(&b.namespace)
			.then_with(|| a.location.cmp(&b.location))
	});

	let mut template_dirs = config.templates.paths;
	template_dirs.sort();
	template_dirs.dedup();

	Ok(ConfigSummary {
		path: config_path,
		data_sources,
		template_dirs,
	})
}

fn normalize_dir_hint(path: &Path) -> String {
	let mut hint = path.display().to_string();
	if !hint.ends_with('/') {
		hint.push('/');
	}
	hint
}

fn template_directory_hints(template_dirs: &[PathBuf]) -> Vec<String> {
	let mut hints = BTreeSet::new();
	for dir in template_dirs {
		hints.insert(normalize_dir_hint(dir));
	}
	for canonical in [
		".templates/",
		"templates/",
		"docs/templates/",
		"shared/templates/",
	] {
		hints.insert(canonical.to_string());
	}
	hints.into_iter().collect()
}

fn count_orphan_consumers(
	providers: &std::collections::HashMap<String, ProviderEntry>,
	consumers: &[ConsumerEntry],
) -> usize {
	consumers
		.iter()
		.filter(|consumer| consumer.block.r#type == BlockType::Consumer)
		.filter(|consumer| !providers.contains_key(&consumer.block.name))
		.count()
}

fn count_unused_providers(
	providers: &std::collections::HashMap<String, ProviderEntry>,
	consumers: &[ConsumerEntry],
) -> usize {
	let referenced: HashSet<&str> = consumers
		.iter()
		.filter(|consumer| consumer.block.r#type == BlockType::Consumer)
		.map(|consumer| consumer.block.name.as_str())
		.collect();
	providers
		.keys()
		.filter(|name| !referenced.contains(name.as_str()))
		.count()
}

fn scan_and_warn(args: &MdtCli) -> Result<ProjectContext, Box<dyn std::error::Error>> {
	let root = resolve_root(args);
	let ctx = scan_project_with_config(&root)?;
	let options = validation_options(args);

	if args.verbose {
		println!(
			"Scanned project: {} provider(s), {} consumer(s)",
			ctx.project.providers.len(),
			ctx.project.consumers.len()
		);

		if !ctx.project.providers.is_empty() {
			println!("  Providers:");
			let mut names: Vec<_> = ctx.project.providers.keys().collect();
			names.sort();
			for name in names {
				let entry = &ctx.project.providers[name];
				println!("    @{name} ({})", entry.file.display());
			}
		}
	}

	// Report diagnostics
	let mut has_errors = false;
	for diag in &ctx.project.diagnostics {
		let rel = make_relative(&diag.file, &root);
		if diag.is_error(&options) {
			let report = diagnostic_to_report(diag, &rel, true);
			eprintln!("{report:?}");
			has_errors = true;
		} else if args.verbose {
			let report = diagnostic_to_report(diag, &rel, false);
			eprintln!("{report:?}");
		}
	}

	if has_errors {
		return Err("validation errors found".into());
	}

	// Warn about consumers referencing non-existent providers.
	let mut missing_providers = ctx.find_missing_providers();
	missing_providers.sort();
	for name in missing_providers {
		eprintln!(
			"{} consumer block `{name}` has no matching provider",
			colored!("warning:", yellow)
		);
	}

	Ok(ctx)
}

fn run_check(
	args: &MdtCli,
	show_diff: bool,
	format: OutputFormat,
	watch: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	// Run the initial check.
	let is_stale = run_check_once(args, show_diff, format)?;

	if !watch {
		if is_stale {
			process::exit(1);
		}
		return Ok(());
	}

	// Watch mode
	println!("\nWatching for file changes... (press Ctrl+C to stop)");

	let root = resolve_root(args);
	let (tx, rx) = mpsc::channel();

	let mut watcher =
		notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
			if let Ok(event) = res {
				if matches!(
					event.kind,
					notify::EventKind::Modify(_) | notify::EventKind::Create(_)
				) {
					let _ = tx.send(());
				}
			}
		})?;

	use notify::Watcher;
	watcher.watch(&root, notify::RecursiveMode::Recursive)?;

	loop {
		rx.recv()?;
		// Debounce: drain additional events within 200ms.
		while rx.recv_timeout(Duration::from_millis(200)).is_ok() {}

		println!("\nFile change detected, checking...");
		if let Err(e) = run_check_once(args, show_diff, format) {
			eprintln!("{} {e}", colored!("error:", red));
		}
	}
}

/// Run a single check and return whether any consumers are stale (true = stale).
fn run_check_once(
	args: &MdtCli,
	show_diff: bool,
	format: OutputFormat,
) -> Result<bool, Box<dyn std::error::Error>> {
	let ctx = scan_and_warn(args)?;
	let root = resolve_root(args);
	let result = check_project(&ctx)?;

	// Always print template variable warnings (they don't affect exit code).
	if !result.warnings.is_empty() {
		print_template_warnings(&result.warnings, &root);
	}

	if result.is_ok() {
		match format {
			OutputFormat::Json => {
				println!("{{\"ok\":true,\"stale\":[]}}");
			}
			OutputFormat::Github => {
				println!("All consumer blocks are up to date.");
			}
			OutputFormat::Text => {
				println!("Check passed: all consumer blocks are up to date.");
			}
		}
		return Ok(false);
	}

	match format {
		OutputFormat::Json => {
			let stale_entries: Vec<serde_json::Value> = result
				.stale
				.iter()
				.map(|entry| {
					let rel = make_relative(&entry.file, &root);
					serde_json::json!({
						"file": rel,
						"block": entry.block_name,
						"line": entry.line,
						"column": entry.column,
					})
				})
				.collect();
			let error_entries: Vec<serde_json::Value> = result
				.render_errors
				.iter()
				.map(|err| {
					let rel = make_relative(&err.file, &root);
					serde_json::json!({
						"file": rel,
						"block": err.block_name,
						"line": err.line,
						"column": err.column,
						"message": err.message,
					})
				})
				.collect();
			let output = serde_json::json!({
				"ok": false,
				"stale": stale_entries,
				"errors": error_entries,
			});
			println!("{output}");
		}
		OutputFormat::Github => {
			for err in &result.render_errors {
				let rel = make_relative(&err.file, &root);
				println!(
					"::error file={rel},line={},col={}::Template render failed for block `{}`: {}",
					err.line, err.column, err.block_name, err.message
				);
			}
			for entry in &result.stale {
				let rel = make_relative(&entry.file, &root);
				println!(
					"::warning file={rel},line={},col={}::Consumer block `{}` is out of date",
					entry.line, entry.column, entry.block_name
				);
			}
			eprintln!("{}", check_summary(&result));
		}
		OutputFormat::Text => {
			eprintln!("Check failed.");
			eprintln!("  render errors: {}", result.render_errors.len());
			eprintln!("  stale consumers: {}", result.stale.len());

			let sorted_errors = sorted_render_errors(&result, &root);
			if !sorted_errors.is_empty() {
				eprintln!();
				eprintln!("Render errors:");
				for err in sorted_errors {
					let rel = make_relative(&err.file, &root);
					eprintln!(
						"  block `{}` at {rel}:{}:{}: {}",
						err.block_name, err.line, err.column, err.message
					);
				}
			}

			let sorted_stale = sorted_stale_entries(&result, &root);
			if !sorted_stale.is_empty() {
				eprintln!();
				eprintln!("Stale consumers:");
				for entry in sorted_stale {
					let rel = make_relative(&entry.file, &root);
					eprintln!(
						"  block `{}` at {rel}:{}:{}",
						entry.block_name, entry.line, entry.column
					);

					if show_diff {
						print_diff(&entry.current_content, &entry.expected_content);
					}
				}
			}

			eprintln!();
			eprintln!("{}", check_summary(&result));
		}
	}

	Ok(true)
}

fn check_summary(result: &mdt_core::CheckResult) -> String {
	let mut parts = Vec::new();
	if !result.render_errors.is_empty() {
		parts.push(format!("{} render error(s)", result.render_errors.len()));
	}
	if !result.stale.is_empty() {
		parts.push(format!(
			"{} consumer block(s) are out of date",
			result.stale.len()
		));
	}
	format!("{}. Run `mdt update` to fix.", parts.join(" and "))
}

fn sorted_stale_entries<'a>(
	result: &'a mdt_core::CheckResult,
	root: &Path,
) -> Vec<&'a mdt_core::StaleEntry> {
	let mut stale_entries: Vec<_> = result.stale.iter().collect();
	stale_entries.sort_by(|a, b| {
		make_relative(&a.file, root)
			.cmp(&make_relative(&b.file, root))
			.then_with(|| a.line.cmp(&b.line))
			.then_with(|| a.column.cmp(&b.column))
			.then_with(|| a.block_name.cmp(&b.block_name))
	});
	stale_entries
}

fn sorted_render_errors<'a>(
	result: &'a mdt_core::CheckResult,
	root: &Path,
) -> Vec<&'a mdt_core::RenderError> {
	let mut render_errors: Vec<_> = result.render_errors.iter().collect();
	render_errors.sort_by(|a, b| {
		make_relative(&a.file, root)
			.cmp(&make_relative(&b.file, root))
			.then_with(|| a.line.cmp(&b.line))
			.then_with(|| a.column.cmp(&b.column))
			.then_with(|| a.block_name.cmp(&b.block_name))
	});
	render_errors
}

fn run_update(args: &MdtCli, dry_run: bool, watch: bool) -> Result<(), Box<dyn std::error::Error>> {
	// Run the initial update.
	run_update_once(args, dry_run)?;

	if !watch || dry_run {
		return Ok(());
	}

	// Watch mode
	println!("\nWatching for file changes... (press Ctrl+C to stop)");

	let root = resolve_root(args);
	let (tx, rx) = mpsc::channel();

	let mut watcher =
		notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
			if let Ok(event) = res {
				if matches!(
					event.kind,
					notify::EventKind::Modify(_) | notify::EventKind::Create(_)
				) {
					let _ = tx.send(());
				}
			}
		})?;

	use notify::Watcher;
	watcher.watch(&root, notify::RecursiveMode::Recursive)?;

	loop {
		rx.recv()?;
		// Debounce: drain additional events within 200ms.
		while rx.recv_timeout(Duration::from_millis(200)).is_ok() {}

		println!("\nFile change detected, updating...");
		if let Err(e) = run_update_once(args, false) {
			eprintln!("{} {e}", colored!("error:", red));
		}
	}
}

fn run_update_once(args: &MdtCli, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
	let ctx = scan_and_warn(args)?;
	let root = resolve_root(args);
	let updates = compute_updates(&ctx)?;

	// Print template variable warnings (they don't prevent updates).
	if !updates.warnings.is_empty() {
		print_template_warnings(&updates.warnings, &root);
	}

	if updates.updated_count == 0 {
		println!("All consumer blocks are already up to date.");
		return Ok(());
	}

	if dry_run {
		println!(
			"Dry run: would update {} block(s) in {} file(s):",
			updates.updated_count,
			updates.updated_files.len()
		);
		let mut paths: Vec<_> = updates.updated_files.keys().collect();
		paths.sort();
		for path in paths {
			let rel = make_relative(path, &root);
			println!("  {rel}");
		}
	} else {
		write_updates(&updates)?;
		println!(
			"Updated {} block(s) in {} file(s).",
			updates.updated_count,
			updates.updated_files.len()
		);

		if args.verbose {
			let mut paths: Vec<_> = updates.updated_files.keys().collect();
			paths.sort();
			for path in paths {
				let rel = make_relative(path, &root);
				println!("  {rel}");
			}
		}
	}

	Ok(())
}

fn run_list(args: &MdtCli) -> Result<(), Box<dyn std::error::Error>> {
	let ctx = scan_and_warn(args)?;
	let root = resolve_root(args);

	if ctx.project.providers.is_empty() && ctx.project.consumers.is_empty() {
		println!("No provider or consumer blocks found.");
		return Ok(());
	}

	// Providers
	if !ctx.project.providers.is_empty() {
		println!("{}", colored!("Providers:", bold));
		let mut names: Vec<_> = ctx.project.providers.keys().collect();
		names.sort();
		for name in names {
			let entry = &ctx.project.providers[name];
			let rel = make_relative(&entry.file, &root);
			let consumer_count = ctx
				.project
				.consumers
				.iter()
				.filter(|consumer| consumer.block.r#type == BlockType::Consumer)
				.filter(|c| c.block.name == *name)
				.count();
			println!("  @{name} {rel} ({consumer_count} consumer(s))");
		}
	}

	// Consumers
	if !ctx.project.consumers.is_empty() {
		if !ctx.project.providers.is_empty() {
			println!();
		}
		println!("{}", colored!("Consumers:", bold));
		for consumer in &ctx.project.consumers {
			let rel = make_relative(&consumer.file, &root);
			let (sigil, status) = match consumer.block.r#type {
				BlockType::Consumer => {
					let has_provider = ctx.project.providers.contains_key(&consumer.block.name);
					let status = if has_provider { "linked" } else { "orphan" };
					("=", status)
				}
				BlockType::Inline => ("~", "inline"),
				BlockType::Provider => ("@", "provider"),
				_ => ("?", "unknown"),
			};
			let transformers = if consumer.block.transformers.is_empty() {
				String::new()
			} else {
				let names: Vec<String> = consumer
					.block
					.transformers
					.iter()
					.map(|t| t.r#type.to_string())
					.collect();
				format!(" |{}", names.join("|"))
			};
			println!(
				"  {sigil}{} {rel}{transformers} [{status}]",
				consumer.block.name
			);
		}
	}

	// Summary
	println!(
		"\n{} provider(s), {} consumer(s)",
		ctx.project.providers.len(),
		ctx.project.consumers.len()
	);

	Ok(())
}

#[derive(serde::Serialize)]
struct InfoProjectSection {
	root: String,
	resolved_config: String,
}

#[derive(serde::Serialize)]
struct InfoBlocksSection {
	providers: usize,
	consumers: usize,
	orphan_consumers: usize,
	unused_providers: usize,
}

#[derive(serde::Serialize)]
struct InfoDataSourceSection {
	namespace: String,
	location: String,
	kind: String,
	format: String,
	explicit_format: bool,
}

#[derive(serde::Serialize)]
struct InfoDataSection {
	namespace_count: usize,
	namespaces: Vec<InfoDataSourceSection>,
}

#[derive(serde::Serialize)]
struct InfoTemplatesSection {
	file_count: usize,
	configured_dirs: Vec<String>,
	canonical_hints: Vec<String>,
	discovered_files: Vec<String>,
}

#[derive(serde::Serialize)]
struct InfoDiagnosticsSection {
	total: usize,
	errors: usize,
	warnings: usize,
	missing_provider_count: usize,
	missing_provider_names: Vec<String>,
}

#[derive(serde::Serialize)]
struct InfoCacheLastScanSection {
	timestamp_unix_ms: u64,
	full_project_hit: bool,
	reused_files: u64,
	reparsed_files: u64,
	total_files: u64,
}

#[derive(serde::Serialize)]
struct InfoCacheSection {
	path: String,
	exists: bool,
	readable: bool,
	valid: bool,
	schema_version: Option<u32>,
	schema_supported: bool,
	project_key_matches: bool,
	hash_verification_enabled: bool,
	scan_count: u64,
	full_project_hit_count: u64,
	full_project_hit_rate: String,
	reused_file_count_total: u64,
	reparsed_file_count_total: u64,
	file_reuse_rate: String,
	last_scan: Option<InfoCacheLastScanSection>,
}

#[derive(serde::Serialize)]
struct InfoReport {
	project: InfoProjectSection,
	blocks: InfoBlocksSection,
	data: InfoDataSection,
	templates: InfoTemplatesSection,
	diagnostics: InfoDiagnosticsSection,
	cache: InfoCacheSection,
}

fn run_info(args: &MdtCli, format: InfoOutputFormat) -> Result<(), Box<dyn std::error::Error>> {
	let root = resolve_root(args);
	let config = load_config_summary(&root)?;
	let loaded_config = MdtConfig::load(&root)?;
	let scan_options = ScanOptions::from_config(loaded_config.as_ref());
	let ctx = scan_project_with_config(&root)?;
	let options = validation_options(args);

	let provider_count = ctx.project.providers.len();
	let consumer_count = ctx.project.consumers.len();
	let orphan_consumer_count =
		count_orphan_consumers(&ctx.project.providers, &ctx.project.consumers);
	let unused_provider_count =
		count_unused_providers(&ctx.project.providers, &ctx.project.consumers);

	let template_files: Vec<String> = ctx
		.project
		.providers
		.values()
		.map(|entry| make_relative(&entry.file, &root))
		.collect::<BTreeSet<_>>()
		.into_iter()
		.collect();

	let diagnostics_total = ctx.project.diagnostics.len();
	let diagnostics_errors = ctx
		.project
		.diagnostics
		.iter()
		.filter(|diag| diag.is_error(&options))
		.count();
	let diagnostics_warnings = diagnostics_total.saturating_sub(diagnostics_errors);

	let mut missing_providers = ctx.find_missing_providers();
	missing_providers.sort();

	let cache_inspection = inspect_project_cache(&root, &scan_options);
	let telemetry = cache_inspection.telemetry.as_ref();
	let scan_count = telemetry.map_or(0, |metrics| metrics.scan_count);
	let full_project_hit_count = telemetry.map_or(0, |metrics| metrics.full_project_hit_count);
	let reused_file_count_total = telemetry.map_or(0, |metrics| metrics.reused_file_count_total);
	let reparsed_file_count_total =
		telemetry.map_or(0, |metrics| metrics.reparsed_file_count_total);
	let full_project_hit_rate = ratio_percent_string(full_project_hit_count, scan_count);
	let file_reuse_rate = ratio_percent_string(
		reused_file_count_total,
		reused_file_count_total.saturating_add(reparsed_file_count_total),
	);
	let last_scan = telemetry.and_then(|metrics| {
		metrics.last_scan.as_ref().map(|scan| {
			InfoCacheLastScanSection {
				timestamp_unix_ms: scan.timestamp_unix_ms,
				full_project_hit: scan.full_project_hit,
				reused_files: scan.reused_files,
				reparsed_files: scan.reparsed_files,
				total_files: scan.total_files,
			}
		})
	});

	let template_hints = template_directory_hints(&config.template_dirs);
	let configured_template_dirs: Vec<String> = config
		.template_dirs
		.iter()
		.map(|path| path.display().to_string())
		.collect();
	let configured_template_dirs_display = if configured_template_dirs.is_empty() {
		"default scan (*.t.md)".to_string()
	} else {
		configured_template_dirs.join(", ")
	};

	let resolved_config = config
		.path
		.as_ref()
		.map_or_else(|| "none".to_string(), |path| path.display().to_string());

	let data_sources: Vec<InfoDataSourceSection> = config
		.data_sources
		.iter()
		.map(|source| {
			InfoDataSourceSection {
				namespace: source.namespace.clone(),
				location: source.location.clone(),
				kind: source.kind.clone(),
				format: source.format.clone(),
				explicit_format: source.explicit_format,
			}
		})
		.collect();

	let report = InfoReport {
		project: InfoProjectSection {
			root: root.display().to_string(),
			resolved_config,
		},
		blocks: InfoBlocksSection {
			providers: provider_count,
			consumers: consumer_count,
			orphan_consumers: orphan_consumer_count,
			unused_providers: unused_provider_count,
		},
		data: InfoDataSection {
			namespace_count: data_sources.len(),
			namespaces: data_sources,
		},
		templates: InfoTemplatesSection {
			file_count: template_files.len(),
			configured_dirs: configured_template_dirs,
			canonical_hints: template_hints,
			discovered_files: template_files,
		},
		diagnostics: InfoDiagnosticsSection {
			total: diagnostics_total,
			errors: diagnostics_errors,
			warnings: diagnostics_warnings,
			missing_provider_count: missing_providers.len(),
			missing_provider_names: missing_providers,
		},
		cache: InfoCacheSection {
			path: cache_inspection.path.display().to_string(),
			exists: cache_inspection.exists,
			readable: cache_inspection.readable,
			valid: cache_inspection.valid,
			schema_version: cache_inspection.schema_version,
			schema_supported: cache_inspection.schema_supported,
			project_key_matches: cache_inspection.project_key_matches,
			hash_verification_enabled: cache_inspection.hash_verification_enabled,
			scan_count,
			full_project_hit_count,
			full_project_hit_rate,
			reused_file_count_total,
			reparsed_file_count_total,
			file_reuse_rate,
			last_scan,
		},
	};

	match format {
		InfoOutputFormat::Json => {
			println!("{}", serde_json::to_string_pretty(&report)?);
		}
		InfoOutputFormat::Text => {
			println!("{}", colored!("mdt info", bold));

			print_section("Project");
			print_field("Project root", &report.project.root);
			print_field("Resolved config", &report.project.resolved_config);

			print_section("Blocks");
			print_field("Providers", report.blocks.providers);
			print_field("Consumers", report.blocks.consumers);
			print_field("Orphan consumers", report.blocks.orphan_consumers);
			print_field("Unused providers", report.blocks.unused_providers);

			print_section("Data");
			print_field("Namespaces", report.data.namespace_count);
			if report.data.namespaces.is_empty() {
				print_field("Source files", "none");
			} else {
				for source in &report.data.namespaces {
					println!(
						"{:<28} {} [{}] -> {}",
						"source", source.namespace, source.kind, source.location
					);
				}
			}

			print_section("Templates");
			print_field("Template files", report.templates.file_count);
			print_field("Configured dirs", configured_template_dirs_display);
			print_field(
				"Canonical hints",
				report.templates.canonical_hints.join(", "),
			);
			if report.templates.discovered_files.is_empty() {
				print_field("Discovered files", "none");
			} else {
				for file in &report.templates.discovered_files {
					println!("{:<28} {file}", "template file");
				}
			}

			print_section("Diagnostics");
			print_field("Total", report.diagnostics.total);
			print_field("Errors", report.diagnostics.errors);
			print_field("Warnings", report.diagnostics.warnings);
			print_field(
				"Missing providers",
				report.diagnostics.missing_provider_count,
			);
			if report.diagnostics.missing_provider_names.is_empty() {
				print_field("Missing names", "none");
			} else {
				print_field(
					"Missing names",
					report.diagnostics.missing_provider_names.join(", "),
				);
			}

			print_section("Cache");
			print_field("Artifact path", &report.cache.path);
			let cache_status = if !report.cache.exists {
				"missing".to_string()
			} else if !report.cache.readable {
				"unreadable".to_string()
			} else if !report.cache.valid {
				"invalid".to_string()
			} else {
				"ok".to_string()
			};
			print_field("Artifact status", cache_status);
			let schema_display = report.cache.schema_version.map_or_else(
				|| "unknown".to_string(),
				|schema| {
					if report.cache.schema_supported {
						format!("{schema} (supported)")
					} else {
						format!("{schema} (unsupported)")
					}
				},
			);
			print_field("Schema version", schema_display);
			print_field(
				"Project key match",
				if report.cache.project_key_matches {
					"yes"
				} else {
					"no"
				},
			);
			print_field(
				"Hash verification",
				if report.cache.hash_verification_enabled {
					"enabled"
				} else {
					"disabled"
				},
			);
			print_field("Scans recorded", report.cache.scan_count);
			print_field(
				"Full project hits",
				format!(
					"{} ({})",
					report.cache.full_project_hit_count, report.cache.full_project_hit_rate
				),
			);
			print_field(
				"File reuse totals",
				format!(
					"{} reused / {} reparsed ({})",
					report.cache.reused_file_count_total,
					report.cache.reparsed_file_count_total,
					report.cache.file_reuse_rate
				),
			);
			if let Some(last_scan) = &report.cache.last_scan {
				print_field(
					"Last scan mode",
					if last_scan.full_project_hit {
						"full cache hit"
					} else {
						"incremental reuse"
					},
				);
				print_field(
					"Last scan files",
					format!(
						"{} reused / {} reparsed / {} total",
						last_scan.reused_files, last_scan.reparsed_files, last_scan.total_files
					),
				);
				print_field("Last scan unix ms", last_scan.timestamp_unix_ms);
			} else {
				print_field("Last scan", "none");
			}
		}
	}

	Ok(())
}

#[derive(Debug, Clone, Copy, serde::Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
enum DoctorStatus {
	Pass,
	Warn,
	Fail,
	Skip,
}

impl DoctorStatus {
	fn tag(self) -> &'static str {
		match self {
			Self::Pass => "PASS",
			Self::Warn => "WARN",
			Self::Fail => "FAIL",
			Self::Skip => "SKIP",
		}
	}

	fn colored_tag(self) -> String {
		match self {
			Self::Pass => colored!(self.tag(), green),
			Self::Warn => colored!(self.tag(), yellow),
			Self::Fail => colored!(self.tag(), red),
			Self::Skip => self.tag().to_string(),
		}
	}
}

#[derive(Debug, serde::Serialize)]
struct DoctorCheck {
	id: &'static str,
	title: &'static str,
	status: DoctorStatus,
	message: String,
	hint: Option<String>,
}

#[derive(Debug, Default, serde::Serialize)]
struct DoctorSummary {
	pass: usize,
	warn: usize,
	fail: usize,
	skip: usize,
}

#[derive(Debug, serde::Serialize)]
struct DoctorReport {
	ok: bool,
	summary: DoctorSummary,
	checks: Vec<DoctorCheck>,
}

fn add_doctor_check(
	checks: &mut Vec<DoctorCheck>,
	id: &'static str,
	title: &'static str,
	status: DoctorStatus,
	message: impl Into<String>,
	hint: Option<String>,
) {
	checks.push(DoctorCheck {
		id,
		title,
		status,
		message: message.into(),
		hint,
	});
}

fn is_canonical_template_dir(path: &Path) -> bool {
	path.components()
		.next()
		.is_some_and(|component| component.as_os_str() == ".templates")
}

fn run_doctor(args: &MdtCli, format: DoctorOutputFormat) -> Result<(), Box<dyn std::error::Error>> {
	let root = resolve_root(args);
	let mut checks = Vec::new();
	let options = validation_options(args);

	let config_path = MdtConfig::resolve_path(&root);
	if let Some(path) = &config_path {
		add_doctor_check(
			&mut checks,
			"config_discovery",
			"Config Discovery",
			DoctorStatus::Pass,
			format!("resolved config at {}", path.display()),
			None,
		);
	} else {
		add_doctor_check(
			&mut checks,
			"config_discovery",
			"Config Discovery",
			DoctorStatus::Warn,
			"no config file found (using defaults)",
			Some(
				"create `mdt.toml`, `.mdt.toml`, or `.config/mdt.toml` to define data and scan \
				 rules"
					.to_string(),
			),
		);
	}

	let config = match MdtConfig::load(&root) {
		Ok(config) => config,
		Err(error) => {
			add_doctor_check(
				&mut checks,
				"config_parse",
				"Config Parse",
				DoctorStatus::Fail,
				format!("failed to parse config: {error}"),
				Some(
					"fix TOML syntax and section structure in the discovered config file"
						.to_string(),
				),
			);
			None
		}
	};

	match &config {
		Some(config) if config.data.is_empty() => {
			add_doctor_check(
				&mut checks,
				"data_sources",
				"Data Sources",
				DoctorStatus::Pass,
				"no data namespaces configured".to_string(),
				None,
			);
		}
		Some(config) => {
			match config.load_data(&root) {
				Ok(loaded_data) => {
					add_doctor_check(
						&mut checks,
						"data_sources",
						"Data Sources",
						DoctorStatus::Pass,
						format!("loaded {} namespace(s) successfully", loaded_data.len()),
						None,
					);
				}
				Err(error) => {
					add_doctor_check(
						&mut checks,
						"data_sources",
						"Data Sources",
						DoctorStatus::Fail,
						format!("failed to load configured data sources: {error}"),
						Some(
							"verify data file paths, script commands, formats, and parse validity \
							 for each [data] namespace"
								.to_string(),
						),
					);
				}
			}
		}
		None => {
			add_doctor_check(
				&mut checks,
				"data_sources",
				"Data Sources",
				DoctorStatus::Skip,
				"skipped because no valid config was loaded".to_string(),
				Some("add a config file to enable explicit data source validation".to_string()),
			);
		}
	}

	let template_paths: Vec<PathBuf> = config
		.as_ref()
		.map(|cfg| cfg.templates.paths.clone())
		.unwrap_or_default();

	if template_paths
		.iter()
		.any(|path| is_canonical_template_dir(path))
	{
		add_doctor_check(
			&mut checks,
			"template_layout",
			"Template Layout",
			DoctorStatus::Pass,
			"using canonical `.templates/` layout".to_string(),
			None,
		);
	} else if root.join(".templates").is_dir() {
		add_doctor_check(
			&mut checks,
			"template_layout",
			"Template Layout",
			DoctorStatus::Pass,
			"found `.templates/` directory".to_string(),
			None,
		);
	} else if !template_paths.is_empty() {
		let configured = template_paths
			.iter()
			.map(|path| path.display().to_string())
			.collect::<Vec<_>>()
			.join(", ");
		add_doctor_check(
			&mut checks,
			"template_layout",
			"Template Layout",
			DoctorStatus::Warn,
			format!("configured template directories: {configured}"),
			Some("prefer `.templates/` as the canonical location for template files".to_string()),
		);
	} else if root.join("templates").is_dir() {
		add_doctor_check(
			&mut checks,
			"template_layout",
			"Template Layout",
			DoctorStatus::Warn,
			"using legacy `templates/` directory".to_string(),
			Some("consider moving templates to `.templates/` for consistency".to_string()),
		);
	} else {
		add_doctor_check(
			&mut checks,
			"template_layout",
			"Template Layout",
			DoctorStatus::Pass,
			"using default template discovery (`*.t.md`)".to_string(),
			None,
		);
	}

	let scan_options = ScanOptions::from_config(config.as_ref());
	let scan_result = scan_project_with_config(&root);
	match scan_result {
		Ok(ctx) => {
			add_doctor_check(
				&mut checks,
				"duplicate_providers",
				"Duplicate Providers",
				DoctorStatus::Pass,
				"provider names are unique".to_string(),
				None,
			);

			let mut missing_providers = ctx.find_missing_providers();
			missing_providers.sort();
			if missing_providers.is_empty() {
				add_doctor_check(
					&mut checks,
					"missing_providers",
					"Missing Providers",
					DoctorStatus::Pass,
					"all consumer blocks resolve to providers".to_string(),
					None,
				);
			} else {
				add_doctor_check(
					&mut checks,
					"missing_providers",
					"Missing Providers",
					DoctorStatus::Fail,
					format!(
						"{} missing provider name(s): {}",
						missing_providers.len(),
						missing_providers.join(", ")
					),
					Some(
						"define the missing provider blocks in template files or rename orphan \
						 consumers"
							.to_string(),
					),
				);
			}

			let orphan_count =
				count_orphan_consumers(&ctx.project.providers, &ctx.project.consumers);
			if orphan_count == 0 {
				add_doctor_check(
					&mut checks,
					"orphan_consumers",
					"Orphan Consumers",
					DoctorStatus::Pass,
					"no orphan consumer blocks found".to_string(),
					None,
				);
			} else {
				add_doctor_check(
					&mut checks,
					"orphan_consumers",
					"Orphan Consumers",
					DoctorStatus::Fail,
					format!("found {orphan_count} orphan consumer block(s)"),
					Some(
						"add matching provider blocks or remove stale consumer references"
							.to_string(),
					),
				);
			}

			let unused_provider_count =
				count_unused_providers(&ctx.project.providers, &ctx.project.consumers);
			if unused_provider_count == 0 {
				add_doctor_check(
					&mut checks,
					"unused_providers",
					"Unused Providers",
					DoctorStatus::Pass,
					"all providers have at least one consumer".to_string(),
					None,
				);
			} else {
				add_doctor_check(
					&mut checks,
					"unused_providers",
					"Unused Providers",
					DoctorStatus::Warn,
					format!("found {unused_provider_count} unused provider block(s)"),
					Some(
						"reuse existing providers from consumer blocks or remove dead templates"
							.to_string(),
					),
				);
			}

			let diagnostics_errors = ctx
				.project
				.diagnostics
				.iter()
				.filter(|diag| diag.is_error(&options))
				.count();
			let diagnostics_warnings = ctx
				.project
				.diagnostics
				.len()
				.saturating_sub(diagnostics_errors);

			if diagnostics_errors == 0 && diagnostics_warnings == 0 {
				add_doctor_check(
					&mut checks,
					"parser_diagnostics",
					"Parser Diagnostics",
					DoctorStatus::Pass,
					"no parser diagnostics found".to_string(),
					None,
				);
			} else if diagnostics_errors > 0 {
				add_doctor_check(
					&mut checks,
					"parser_diagnostics",
					"Parser Diagnostics",
					DoctorStatus::Fail,
					format!("{diagnostics_errors} error(s), {diagnostics_warnings} warning(s)"),
					Some(
						"fix malformed blocks and invalid transformers reported by `mdt check`"
							.to_string(),
					),
				);
			} else {
				add_doctor_check(
					&mut checks,
					"parser_diagnostics",
					"Parser Diagnostics",
					DoctorStatus::Warn,
					format!("0 error(s), {diagnostics_warnings} warning(s)"),
					Some("review warnings to keep template hygiene strong over time".to_string()),
				);
			}
		}
		Err(error) => {
			match error {
				MdtError::DuplicateProvider {
					name,
					first_file,
					second_file,
				} => {
					add_doctor_check(
						&mut checks,
						"duplicate_providers",
						"Duplicate Providers",
						DoctorStatus::Fail,
						format!(
							"provider `{name}` is declared in `{first_file}` and `{second_file}`"
						),
						Some(
							"rename one provider to a unique name; provider names must be \
							 globally unique"
								.to_string(),
						),
					);
				}
				other => {
					add_doctor_check(
						&mut checks,
						"project_scan",
						"Project Scan",
						DoctorStatus::Fail,
						format!("project scan failed: {other}"),
						Some("fix scan/config errors first, then rerun `mdt doctor`".to_string()),
					);
				}
			}

			for (id, title) in [
				("missing_providers", "Missing Providers"),
				("orphan_consumers", "Orphan Consumers"),
				("unused_providers", "Unused Providers"),
				("parser_diagnostics", "Parser Diagnostics"),
			] {
				add_doctor_check(
					&mut checks,
					id,
					title,
					DoctorStatus::Skip,
					"skipped because project scan did not complete".to_string(),
					None,
				);
			}
		}
	}

	let cache = inspect_project_cache(&root, &scan_options);
	if !cache.exists {
		add_doctor_check(
			&mut checks,
			"cache_artifact",
			"Cache Artifact",
			DoctorStatus::Warn,
			format!("cache artifact not found at {}", cache.path.display()),
			Some(
				"run `mdt check` or `mdt info` to trigger a scan and write the cache artifact"
					.to_string(),
			),
		);
	} else if !cache.readable {
		add_doctor_check(
			&mut checks,
			"cache_artifact",
			"Cache Artifact",
			DoctorStatus::Fail,
			format!(
				"cache artifact exists but is not readable: {}",
				cache.path.display()
			),
			Some("verify filesystem permissions for `.mdt/cache/`".to_string()),
		);
	} else if !cache.valid {
		let schema = cache
			.schema_version
			.map_or_else(|| "unknown".to_string(), |version| version.to_string());
		add_doctor_check(
			&mut checks,
			"cache_artifact",
			"Cache Artifact",
			DoctorStatus::Fail,
			format!("cache artifact is invalid for current schema (found version {schema})"),
			Some(
				"remove `.mdt/cache/index-v2.json` and rerun `mdt check` to rebuild clean cache \
				 metadata"
					.to_string(),
			),
		);
	} else if !cache.project_key_matches {
		add_doctor_check(
			&mut checks,
			"cache_artifact",
			"Cache Artifact",
			DoctorStatus::Warn,
			"cache artifact is readable but keyed for different scan options".to_string(),
			Some(
				"this is normal after config changes; rerun scans with stable options to rebuild \
				 cache history"
					.to_string(),
			),
		);
	} else {
		add_doctor_check(
			&mut checks,
			"cache_artifact",
			"Cache Artifact",
			DoctorStatus::Pass,
			format!(
				"cache artifact is readable and valid at {}",
				cache.path.display()
			),
			None,
		);
	}

	let hash_mode_message = if cache.hash_verification_enabled {
		"content-hash verification enabled (`MDT_CACHE_VERIFY_HASH` set)".to_string()
	} else {
		"content-hash verification disabled (mtime + size fingerprints only)".to_string()
	};
	add_doctor_check(
		&mut checks,
		"cache_hash_mode",
		"Cache Hash Mode",
		DoctorStatus::Pass,
		hash_mode_message,
		Some(cache_hash_mode_hint(cache.hash_verification_enabled)),
	);

	if let Some(telemetry) = &cache.telemetry {
		let total_files = telemetry
			.reused_file_count_total
			.saturating_add(telemetry.reparsed_file_count_total);
		if telemetry.scan_count < 3 || total_files == 0 {
			add_doctor_check(
				&mut checks,
				"cache_efficiency",
				"Cache Efficiency",
				DoctorStatus::Skip,
				"insufficient history for trend analysis (need at least 3 scans)".to_string(),
				None,
			);
		} else {
			let reparse_rate =
				ratio_percent_string(telemetry.reparsed_file_count_total, total_files);
			if telemetry.reparsed_file_count_total
				> telemetry.reused_file_count_total.saturating_mul(3)
			{
				add_doctor_check(
					&mut checks,
					"cache_efficiency",
					"Cache Efficiency",
					DoctorStatus::Warn,
					format!(
						"high reparse trend: {} reparsed vs {} reused ({reparse_rate} reparsed)",
						telemetry.reparsed_file_count_total, telemetry.reused_file_count_total
					),
					Some(cache_hash_mode_hint(cache.hash_verification_enabled)),
				);
			} else {
				let reuse_rate =
					ratio_percent_string(telemetry.reused_file_count_total, total_files);
				add_doctor_check(
					&mut checks,
					"cache_efficiency",
					"Cache Efficiency",
					DoctorStatus::Pass,
					format!(
						"healthy cache trend: {} reused vs {} reparsed ({reuse_rate} reused)",
						telemetry.reused_file_count_total, telemetry.reparsed_file_count_total
					),
					None,
				);
			}
		}
	} else {
		add_doctor_check(
			&mut checks,
			"cache_efficiency",
			"Cache Efficiency",
			DoctorStatus::Skip,
			"cache telemetry unavailable".to_string(),
			Some(
				"ensure cache artifact is valid, then run `mdt info` or `mdt check` a few times \
				 to gather telemetry"
					.to_string(),
			),
		);
	}

	let mut summary = DoctorSummary::default();
	for check in &checks {
		match check.status {
			DoctorStatus::Pass => summary.pass += 1,
			DoctorStatus::Warn => summary.warn += 1,
			DoctorStatus::Fail => summary.fail += 1,
			DoctorStatus::Skip => summary.skip += 1,
		}
	}

	let report = DoctorReport {
		ok: summary.fail == 0,
		summary,
		checks,
	};

	match format {
		DoctorOutputFormat::Json => {
			println!("{}", serde_json::to_string_pretty(&report)?);
		}
		DoctorOutputFormat::Text => {
			println!("{}", colored!("mdt doctor", bold));
			for check in &report.checks {
				println!(
					"[{}] {:<22} {}",
					check.status.colored_tag(),
					check.title,
					check.message
				);
				if let Some(hint) = &check.hint {
					println!("       hint: {hint}");
				}
			}

			println!();
			println!(
				"summary: {} pass, {} warn, {} fail, {} skip",
				report.summary.pass, report.summary.warn, report.summary.fail, report.summary.skip
			);
		}
	}

	if report.ok { Ok(()) } else { process::exit(1) }
}
fn run_lsp() -> Result<(), Box<dyn std::error::Error>> {
	let rt = tokio::runtime::Runtime::new()?;
	rt.block_on(mdt_lsp::run_server());
	Ok(())
}

fn run_mcp() -> Result<(), Box<dyn std::error::Error>> {
	let rt = tokio::runtime::Runtime::new()?;
	rt.block_on(mdt_mcp::run_server());
	Ok(())
}

/// Print warnings about undefined template variables.
fn print_template_warnings(warnings: &[TemplateWarning], root: &Path) {
	let mut sorted_warnings: Vec<_> = warnings.iter().collect();
	sorted_warnings.sort_by(|a, b| {
		make_relative(&a.provider_file, root)
			.cmp(&make_relative(&b.provider_file, root))
			.then_with(|| a.block_name.cmp(&b.block_name))
	});

	for warning in sorted_warnings {
		let rel = make_relative(&warning.provider_file, root);
		let mut undefined_vars = warning.undefined_variables.clone();
		undefined_vars.sort();
		let vars = undefined_vars.join(", ");
		eprintln!(
			"{} provider block `{}` in {rel} references undefined variable(s): {vars}",
			colored!("warning:", yellow),
			warning.block_name,
		);
	}
}

/// Print a unified diff between two strings, colorized.
fn print_diff(current: &str, expected: &str) {
	let diff = TextDiff::from_lines(current, expected);
	for change in diff.iter_all_changes() {
		match change.tag() {
			ChangeTag::Delete => {
				eprint!("  {}", colored!(format!("-{change}"), red));
			}
			ChangeTag::Insert => {
				eprint!("  {}", colored!(format!("+{change}"), green));
			}
			ChangeTag::Equal => {
				eprint!("   {change}");
			}
		}
	}
}

/// Make a path relative to root for display purposes.
fn make_relative(path: &Path, root: &Path) -> String {
	path.strip_prefix(root)
		.unwrap_or(path)
		.display()
		.to_string()
}

/// Convert a `ProjectDiagnostic` into a `miette::Report` with appropriate
/// severity, error code, and help text for rich terminal display.
fn diagnostic_to_report(
	diag: &ProjectDiagnostic,
	rel_path: &str,
	is_error: bool,
) -> miette::Report {
	let location = format!("{rel_path}:{}:{}", diag.line, diag.column);
	let severity = if is_error {
		miette::Severity::Error
	} else {
		miette::Severity::Warning
	};

	let message = format!("[{location}] {}", diag.message());
	let help: String = match &diag.kind {
		DiagnosticKind::UnclosedBlock { name } => {
			format!("add `<!-- {{/{name}}} -->` to close this block")
		}
		DiagnosticKind::UnknownTransformer { .. } => {
			"available transformers: trim, trimStart, trimEnd, indent, prefix, suffix, linePrefix, \
			 lineSuffix, wrap, codeBlock, code, replace"
				.to_string()
		}
		DiagnosticKind::InvalidTransformerArgs { .. } => {
			"check the transformer documentation for the correct number of arguments".to_string()
		}
		DiagnosticKind::UnusedProvider { name } => {
			format!(
				"add a consumer block `<!-- {{={name}}} -->...<!-- {{/{name}}} -->` or remove the \
				 unused provider"
			)
		}
		_ => diag.message(),
	};
	let code = match &diag.kind {
		DiagnosticKind::UnclosedBlock { .. } => "mdt::unclosed_block",
		DiagnosticKind::UnknownTransformer { .. } => "mdt::unknown_transformer",
		DiagnosticKind::InvalidTransformerArgs { .. } => "mdt::invalid_transformer_args",
		DiagnosticKind::UnusedProvider { .. } => "mdt::unused_provider",
		_ => "mdt::diagnostic",
	};

	let diag_value = miette::MietteDiagnostic::new(message)
		.with_code(code)
		.with_help(help)
		.with_severity(severity);
	miette::Report::new(diag_value)
}

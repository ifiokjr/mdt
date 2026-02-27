use std::collections::BTreeSet;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::process;
use std::sync::mpsc;
use std::time::Duration;

use clap::Parser;
use mdt_cli::Commands;
use mdt_cli::MdtCli;
use mdt_cli::OutputFormat;
use mdt_core::MdtConfig;
use mdt_core::TemplateWarning;
use mdt_core::check_project;
use mdt_core::compute_updates;
use mdt_core::project::ConsumerEntry;
use mdt_core::project::DiagnosticKind;
use mdt_core::project::ProjectContext;
use mdt_core::project::ProjectDiagnostic;
use mdt_core::project::ProviderEntry;
use mdt_core::project::ValidationOptions;
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
		Some(Commands::Info) => run_info(&args),
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
		match e.downcast::<mdt_core::MdtError>() {
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

fn run_init(args: &MdtCli) -> Result<(), Box<dyn std::error::Error>> {
	let root = resolve_root(args);
	let template_path = root.join("template.t.md");
	let config_path = root.join("mdt.toml");

	let template_exists = template_path.exists();
	let config_exists = config_path.exists();

	if template_exists {
		println!("Template file already exists: {}", template_path.display());
	} else {
		let sample_content = "<!-- {@greeting} -->\n\nHello from mdt! This is a provider \
		                      block.\n\n<!-- {/greeting} -->\n";

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
			 = \"Cargo.toml\"\n\n# Control blank lines between tags and content in source \
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
	data_sources: Vec<(String, PathBuf)>,
	template_dirs: Vec<PathBuf>,
}

fn load_config_summary(root: &Path) -> Result<ConfigSummary, Box<dyn std::error::Error>> {
	let config_path = root.join("mdt.toml");
	let config = MdtConfig::load(root)?;

	let Some(config) = config else {
		return Ok(ConfigSummary::default());
	};

	let mut data_sources: Vec<_> = config.data.into_iter().collect();
	data_sources.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

	let mut template_dirs = config.templates.paths;
	template_dirs.sort();
	template_dirs.dedup();

	Ok(ConfigSummary {
		path: Some(config_path),
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
	for canonical in ["templates/", "docs/templates/", "shared/templates/"] {
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
		.filter(|consumer| !providers.contains_key(&consumer.block.name))
		.count()
}

fn count_unused_providers(
	providers: &std::collections::HashMap<String, ProviderEntry>,
	consumers: &[ConsumerEntry],
) -> usize {
	let referenced: HashSet<&str> = consumers.iter().map(|c| c.block.name.as_str()).collect();
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
			let has_provider = ctx.project.providers.contains_key(&consumer.block.name);
			let status = if has_provider { "linked" } else { "orphan" };
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
			println!("  ={} {rel}{transformers} [{status}]", consumer.block.name);
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

fn run_info(args: &MdtCli) -> Result<(), Box<dyn std::error::Error>> {
	let root = resolve_root(args);
	let config = load_config_summary(&root)?;
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

	let template_hints = template_directory_hints(&config.template_dirs);
	let configured_template_dirs = if config.template_dirs.is_empty() {
		"default scan (*.t.md)".to_string()
	} else {
		config
			.template_dirs
			.iter()
			.map(|path| path.display().to_string())
			.collect::<Vec<_>>()
			.join(", ")
	};

	let resolved_config = config
		.path
		.map_or_else(|| "none".to_string(), |path| path.display().to_string());

	println!("{}", colored!("mdt info", bold));

	print_section("Project");
	print_field("Project root", root.display());
	print_field("Resolved config", resolved_config);

	print_section("Blocks");
	print_field("Providers", provider_count);
	print_field("Consumers", consumer_count);
	print_field("Orphan consumers", orphan_consumer_count);
	print_field("Unused providers", unused_provider_count);

	print_section("Data");
	print_field("Namespaces", config.data_sources.len());
	if config.data_sources.is_empty() {
		print_field("Source files", "none");
	} else {
		for (namespace, source_file) in &config.data_sources {
			println!("{:<28} {namespace} -> {}", "source", source_file.display());
		}
	}

	print_section("Templates");
	print_field("Template files", template_files.len());
	print_field("Configured dirs", configured_template_dirs);
	print_field("Canonical hints", template_hints.join(", "));
	if template_files.is_empty() {
		print_field("Discovered files", "none");
	} else {
		for file in &template_files {
			println!("{:<28} {file}", "template file");
		}
	}

	print_section("Diagnostics");
	print_field("Total", diagnostics_total);
	print_field("Errors", diagnostics_errors);
	print_field("Warnings", diagnostics_warnings);
	print_field("Missing providers", missing_providers.len());
	if missing_providers.is_empty() {
		print_field("Missing names", "none");
	} else {
		print_field("Missing names", missing_providers.join(", "));
	}

	Ok(())
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

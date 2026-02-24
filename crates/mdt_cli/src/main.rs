use std::path::Path;
use std::path::PathBuf;
use std::process;
use std::sync::mpsc;
use std::time::Duration;

use clap::Parser;
use mdt_cli::Commands;
use mdt_cli::MdtCli;
use mdt_cli::OutputFormat;
use mdt_core::check_project;
use mdt_core::compute_updates;
use mdt_core::project::ProjectContext;
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
	if args.no_color || std::env::var_os("NO_COLOR").is_some() {
		USE_COLOR.store(false, std::sync::atomic::Ordering::Relaxed);
	}

	let result = match args.command {
		Some(Commands::Init) => run_init(&args),
		Some(Commands::Check { diff, format }) => run_check(&args, diff, format),
		Some(Commands::Update { dry_run, watch }) => run_update(&args, dry_run, watch),
		Some(Commands::List) => run_list(&args),
		Some(Commands::Lsp) => run_lsp(),
		Some(Commands::Mcp) => run_mcp(),
		None => {
			eprintln!("No subcommand specified. Run `mdt --help` for usage.");
			process::exit(1);
		}
	};

	if let Err(e) = result {
		eprintln!("{} {e}", colored!("error:", red));
		process::exit(2);
	}
}

fn resolve_root(args: &MdtCli) -> PathBuf {
	args.path
		.clone()
		.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn run_init(args: &MdtCli) -> Result<(), Box<dyn std::error::Error>> {
	let root = resolve_root(args);
	let template_path = root.join("template.t.md");

	if template_path.exists() {
		println!("Template file already exists: {}", template_path.display());
		return Ok(());
	}

	let sample_content = "<!-- {@greeting} -->\n\nHello from mdt! This is a provider \
	                      block.\n\n<!-- {/greeting} -->\n";

	std::fs::write(&template_path, sample_content)?;
	println!("Created template file: {}", template_path.display());
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
			for (name, entry) in &ctx.project.providers {
				println!("    @{name} ({})", entry.file.display());
			}
		}
	}

	// Report diagnostics
	let mut has_errors = false;
	for diag in &ctx.project.diagnostics {
		let rel = make_relative(&diag.file, &root);
		if diag.is_error(&options) {
			eprintln!(
				"{} {rel}:{}:{}: {}",
				colored!("error:", red),
				diag.line,
				diag.column,
				diag.message()
			);
			has_errors = true;
		} else if args.verbose {
			eprintln!(
				"{} {rel}:{}:{}: {}",
				colored!("warning:", yellow),
				diag.line,
				diag.column,
				diag.message()
			);
		}
	}

	if has_errors {
		return Err("validation errors found".into());
	}

	// Warn about consumers referencing non-existent providers
	for name in &ctx.find_missing_providers() {
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
) -> Result<(), Box<dyn std::error::Error>> {
	let ctx = scan_and_warn(args)?;
	let root = resolve_root(args);
	let result = check_project(&ctx)?;

	if result.is_ok() {
		match format {
			OutputFormat::Json => {
				println!("{{\"ok\":true,\"stale\":[]}}");
			}
			OutputFormat::Github | OutputFormat::Text => {
				println!("All consumer blocks are up to date.");
			}
		}
		return Ok(());
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
			let output = serde_json::json!({
				"ok": false,
				"stale": stale_entries,
			});
			println!("{output}");
		}
		OutputFormat::Github => {
			for entry in &result.stale {
				let rel = make_relative(&entry.file, &root);
				println!(
					"::warning file={rel},line={},col={}::Consumer block `{}` is out of date",
					entry.line, entry.column, entry.block_name
				);
			}
			eprintln!(
				"\n{} consumer block(s) are out of date. Run `mdt update` to fix.",
				result.stale.len()
			);
		}
		OutputFormat::Text => {
			for entry in &result.stale {
				let rel = make_relative(&entry.file, &root);
				eprintln!(
					"Stale: block `{}` in {rel}:{}:{}",
					entry.block_name, entry.line, entry.column
				);

				if show_diff {
					print_diff(&entry.current_content, &entry.expected_content);
				}
			}
			eprintln!(
				"\n{} consumer block(s) are out of date. Run `mdt update` to fix.",
				result.stale.len()
			);
		}
	}

	process::exit(1);
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

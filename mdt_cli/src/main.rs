mod skill;

#[cfg(test)]
mod __tests;

use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::process;
use std::sync::mpsc;
use std::time::Duration;

use clap::Parser;
use mdt_cli::AssistOutputFormat;
use mdt_cli::Assistant;
use mdt_cli::Commands;
use mdt_cli::DoctorOutputFormat;
use mdt_cli::InfoOutputFormat;
use mdt_cli::MdtCli;
use mdt_cli::OutputFormat;
use mdt_core::BlockType;
use mdt_core::MdtConfig;
use mdt_core::MdtError;
use mdt_core::TemplateWarning;
use mdt_core::UpdateScope;
use mdt_core::check_project;
use mdt_core::compute_updates;
use mdt_core::doctor::DoctorStatus;
use mdt_core::doctor::diagnostic_help;
use mdt_core::doctor::display_path;
use mdt_core::doctor::orphan_description;
use mdt_core::doctor::sorted_render_errors;
use mdt_core::project::ConsumerEntry;
use mdt_core::project::ProjectContext;
use mdt_core::project::ProjectDiagnostic;
use mdt_core::project::ValidationOptions;
use mdt_core::project::normalize_lexically;
use mdt_core::project::relative_display_path;
use mdt_core::project::resolve_root as resolve_root_path;
use mdt_core::project::scan_project_with_config;
use mdt_core::project::suggest_similar_provider_names;
use mdt_core::write_updates;
use owo_colors::OwoColorize;
use similar::ChangeTag;
use similar::TextDiff;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;

static USE_STDOUT_COLOR: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static USE_STDERR_COLOR: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[derive(Clone, Copy)]
enum ColorStream {
	Stdout,
	Stderr,
}

fn color_enabled(stream: ColorStream) -> bool {
	match stream {
		ColorStream::Stdout => USE_STDOUT_COLOR.load(std::sync::atomic::Ordering::Relaxed),
		ColorStream::Stderr => USE_STDERR_COLOR.load(std::sync::atomic::Ordering::Relaxed),
	}
}

fn detect_color(stream: supports_color::Stream) -> bool {
	if let Some(force) = std::env::var_os("CLICOLOR_FORCE") {
		return force != "0";
	}

	if std::env::var_os("NO_COLOR").is_some() {
		return false;
	}

	if std::env::var_os("CLICOLOR").as_deref() == Some(std::ffi::OsStr::new("0")) {
		return false;
	}

	supports_color::on(stream).is_some()
}

/// Apply ANSI styles only when the target stream supports color.
macro_rules! styled {
	(stdout, $text:expr,bold) => {
		if color_enabled(ColorStream::Stdout) {
			format!("{}", $text.bold())
		} else {
			format!("{}", $text)
		}
	};
	(stdout, $text:expr,red_bold) => {
		if color_enabled(ColorStream::Stdout) {
			format!("{}", $text.red().bold())
		} else {
			format!("{}", $text)
		}
	};
	(stdout, $text:expr,green_bold) => {
		if color_enabled(ColorStream::Stdout) {
			format!("{}", $text.green().bold())
		} else {
			format!("{}", $text)
		}
	};
	(stdout, $text:expr,yellow_bold) => {
		if color_enabled(ColorStream::Stdout) {
			format!("{}", $text.yellow().bold())
		} else {
			format!("{}", $text)
		}
	};
	(stderr, $text:expr,red) => {
		if color_enabled(ColorStream::Stderr) {
			format!("{}", $text.red())
		} else {
			format!("{}", $text)
		}
	};
	(stderr, $text:expr,green) => {
		if color_enabled(ColorStream::Stderr) {
			format!("{}", $text.green())
		} else {
			format!("{}", $text)
		}
	};
	(stderr, $text:expr,yellow) => {
		if color_enabled(ColorStream::Stderr) {
			format!("{}", $text.yellow())
		} else {
			format!("{}", $text)
		}
	};
	(stderr, $text:expr,cyan) => {
		if color_enabled(ColorStream::Stderr) {
			format!("{}", $text.cyan())
		} else {
			format!("{}", $text)
		}
	};
	(stderr, $text:expr,red_bold) => {
		if color_enabled(ColorStream::Stderr) {
			format!("{}", $text.red().bold())
		} else {
			format!("{}", $text)
		}
	};
	(stderr, $text:expr,yellow_bold) => {
		if color_enabled(ColorStream::Stderr) {
			format!("{}", $text.yellow().bold())
		} else {
			format!("{}", $text)
		}
	};
}

fn main() {
	let args = MdtCli::parse();

	let stdout_color = !args.no_color && detect_color(supports_color::Stream::Stdout);
	let stderr_color = !args.no_color && detect_color(supports_color::Stream::Stderr);
	USE_STDOUT_COLOR.store(stdout_color, std::sync::atomic::Ordering::Relaxed);
	USE_STDERR_COLOR.store(stderr_color, std::sync::atomic::Ordering::Relaxed);

	if let Ok(filter) = EnvFilter::try_from_env("MDT_LOG") {
		fmt::Subscriber::builder()
			.with_env_filter(filter)
			.with_writer(std::io::stderr)
			.init();
	}

	let disable_miette_color = !stderr_color;
	miette::set_hook(Box::new(move |_| {
		let mut opts = miette::MietteHandlerOpts::new();
		if disable_miette_color {
			opts = opts.color(false).unicode(false);
		}

		Box::new(opts.build())
	}))
	.ok();

	if matches!(
		args.command,
		Commands::Check { .. }
			| Commands::Update { .. }
			| Commands::List
			| Commands::Info { .. }
			| Commands::Doctor { .. }
	) {
		note_discovered_root(&args);
	}
	let result = validate_project_root(&args).and_then(|()| {
		match args.command {
			Commands::Init => run_init(&args),
			Commands::Check {
				diff,
				format,
				watch,
			} => run_check(&args, diff, format, watch),
			Commands::Update {
				dry_run,
				watch,
				ref block,
				ref file,
			} => run_update(&args, dry_run, watch, block, file),
			Commands::List => run_list(&args),
			Commands::Info { format } => run_info(&args, format),
			Commands::Doctor { format } => run_doctor(&args, format),
			Commands::Assist { assistant, format } => run_assist(assistant, format),
			Commands::Skill { reference, install } => run_skill(reference, install.as_deref()),
			Commands::Lsp => run_lsp(),
			Commands::Mcp => run_mcp(&args),
		}
	});

	if let Err(e) = result {
		// Try to render through miette for rich diagnostics with help text
		// and error codes.
		match e.downcast::<MdtError>() {
			Ok(mdt_err) => {
				let report: miette::Report = (*mdt_err).into();
				eprintln!("{report:?}");
			}
			Err(e) => {
				eprintln!("{} {e}", styled!(stderr, "error:", red_bold));
			}
		}

		process::exit(2);
	}
}

fn print_section(title: &str) {
	println!();
	println!("{}", styled!(stdout, title, bold));
}

/// The project root: `--path` when given; otherwise the nearest directory,
/// from the current one up to the root of the enclosing git repository,
/// that has an mdt config file, so running from a subdirectory behaves like
/// running from the project root. The search never leaves the repository —
/// a stray config in a parent directory such as `$HOME` must not take over —
/// and outside a git repository, like `mdt init`, it uses the current
/// directory.
fn resolve_root(args: &MdtCli) -> PathBuf {
	let root = resolve_root_path(args.path.as_deref());

	if args.path.is_some() || matches!(args.command, Commands::Init) {
		return root;
	}

	let Some(repository) = root.ancestors().find(|dir| dir.join(".git").exists()) else {
		return root;
	};
	root.ancestors()
		.take_while(|dir| dir.starts_with(repository))
		.find(|dir| MdtConfig::resolve_path(dir).is_some())
		.map_or(root.clone(), Path::to_path_buf)
}

/// Say which project a command runs on when it is not the current directory,
/// so a surprising root is visible.
fn note_discovered_root(args: &MdtCli) {
	if args.path.is_some() || matches!(args.command, Commands::Init) {
		return;
	}

	let root = resolve_root(args);
	let cwd = resolve_root_path(None);

	if root != cwd {
		eprintln!(
			"{} using the mdt project at {}",
			styled!(stderr, "note:", cyan),
			display_path(&root)
		);
	}
}

/// Reject a `--path` that is not an existing directory. A mistyped path used
/// to "pass" every check against an empty project (and create a stray
/// `.mdt/` cache there), which kept CI green. `mdt init` creates the
/// directory instead.
fn validate_project_root(args: &MdtCli) -> Result<(), Box<dyn std::error::Error>> {
	let Some(path) = &args.path else {
		return Ok(());
	};

	if matches!(args.command, Commands::Init) || path.is_dir() {
		return Ok(());
	}
	let problem = if path.exists() {
		"is not a directory"
	} else {
		"does not exist"
	};

	Err(format!("project path `{}` {problem}", display_path(path)).into())
}

fn print_field(label: &str, value: impl std::fmt::Display) {
	println!("{label:<28} {value}");
}

fn run_init(args: &MdtCli) -> Result<(), Box<dyn std::error::Error>> {
	use mdt_core::init::ConfigOutcome;
	use mdt_core::init::GitignoreOutcome;
	use mdt_core::init::SampleOutcome;

	let root = resolve_root(args);
	let report = mdt_core::init::init_project(&root)
		.map_err(|error| format!("failed to initialize {}: {error}", display_path(&root)))?;
	let rel = |path: &Path| relative_display_path(path, &root);

	if args.path.is_some() {
		let verb = if report.created_root {
			"Created"
		} else {
			"Initializing"
		};

		println!("{verb} {}", display_path(&root));
	}

	match &report.config {
		ConfigOutcome::Created(path) => println!("Created {}", rel(path)),
		ConfigOutcome::Exists(path) => println!("Using existing config {}", rel(path)),
		_ => {}
	}

	match &report.sample {
		SampleOutcome::CreatedWithReadme { template, readme } => {
			println!(
				"Created {} with a sample `greeting` provider",
				rel(template)
			);
			println!("Created {} with a synced `greeting` consumer", rel(readme));
		}
		SampleOutcome::CreatedWithoutConsumer { template, readme } => {
			println!(
				"Created {} with a sample `greeting` provider",
				rel(template)
			);
			println!("Left the existing {} unchanged", rel(readme));
		}
		SampleOutcome::TemplateExists { template } => {
			println!("Template file already exists: {}", rel(template));
		}
		SampleOutcome::ProvidersExist { count } => {
			println!("Found {count} existing provider(s); skipped the sample template");
		}
		_ => {}
	}

	match &report.gitignore {
		GitignoreOutcome::Updated(path) | GitignoreOutcome::Created(path) => {
			println!("Added the `.mdt/` cache directory to {}", rel(path));
		}
		_ => {}
	}

	if let Some(enclosing) = &report.enclosing_project {
		eprintln!(
			"{} this is now a separate mdt project inside the one at {}, whose `mdt check` skips \
			 it; check it with `mdt check --path {}` too, or remove {} to keep one project",
			styled!(stderr, "warning:", yellow_bold),
			display_path(enclosing),
			display_path(&root),
			match &report.config {
				ConfigOutcome::Created(path) => display_path(path),
				_ => "the new config".to_string(),
			}
		);
	}

	let next_steps: Vec<String> = match &report.sample {
		SampleOutcome::CreatedWithReadme { template, readme } => {
			vec![
				format!("Open {} to see the synced sample block", rel(readme)),
				format!("Edit {}, then run `mdt update`", rel(template)),
				"Run `mdt check` in CI to fail builds on stale docs".to_string(),
			]
		}
		SampleOutcome::CreatedWithoutConsumer { template, readme } => {
			vec![
				format!(
					"Add a consumer to {}: <!-- {{=greeting}} --> <!-- {{/greeting}} -->",
					rel(readme)
				),
				"Run `mdt update` to fill it in (until then `mdt check` warns that `greeting` has \
				 no consumers)"
					.to_string(),
				format!(
					"Replace the sample in {} with your own providers",
					rel(template)
				),
			]
		}
		_ => Vec::new(),
	};

	if !next_steps.is_empty() {
		println!();
		println!("Next steps:");

		for (index, step) in next_steps.iter().enumerate() {
			println!("  {}. {step}", index + 1);
		}
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

fn scan(args: &MdtCli) -> Result<ProjectContext, Box<dyn std::error::Error>> {
	let root = resolve_root(args);
	let ctx = scan_project_with_config(&root)?;

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
				println!(
					"    @{name} ({})",
					relative_display_path(&entry.file, &root)
				);
			}
		}
	}

	Ok(ctx)
}

/// Diagnostics in file and line order, so output is stable.
fn sorted_diagnostics<'a>(ctx: &'a ProjectContext, root: &Path) -> Vec<&'a ProjectDiagnostic> {
	let mut diagnostics: Vec<_> = ctx.project.diagnostics.iter().collect();
	diagnostics.sort_by(|a, b| {
		relative_display_path(&a.file, root)
			.cmp(&relative_display_path(&b.file, root))
			.then_with(|| a.line.cmp(&b.line))
			.then_with(|| a.column.cmp(&b.column))
	});
	diagnostics
}

/// Print scan diagnostics for text output: errors always, warnings unless an
/// `--ignore-*` flag silences them, and silenced ones only with `--verbose`.
/// Returns whether any diagnostic is an error.
fn report_diagnostics(args: &MdtCli, ctx: &ProjectContext) -> bool {
	let root = resolve_root(args);
	let options = validation_options(args);
	let mut has_errors = false;

	for diag in sorted_diagnostics(ctx, &root) {
		let rel = relative_display_path(&diag.file, &root);
		let is_error = diag.is_error(&options);
		has_errors |= is_error;

		if is_error || !diag.is_ignored(&options) || args.verbose {
			eprintln!("{:?}", diagnostic_to_report(diag, &rel, is_error));
		}
	}

	has_errors
}

fn has_error_diagnostics(args: &MdtCli, ctx: &ProjectContext) -> bool {
	let options = validation_options(args);
	ctx.project
		.diagnostics
		.iter()
		.any(|diag| diag.is_error(&options))
}

/// Error returned once validation errors have been reported, so the process
/// exits with status 2 without repeating them.
fn validation_failed() -> Box<dyn std::error::Error> {
	"validation errors found; fix the errors above".into()
}

/// Warn about consumers whose name matches no provider. `check` reports
/// these as failures instead.
fn warn_orphans(ctx: &ProjectContext, root: &Path) -> usize {
	let mut orphans: Vec<&ConsumerEntry> = ctx
		.project
		.consumers
		.iter()
		.filter(|consumer| {
			consumer.block.r#type == BlockType::Consumer
				&& !ctx.project.providers.contains_key(&consumer.block.name)
		})
		.collect();
	orphans.sort_by(|a, b| {
		(&a.file, a.block.opening.start.line).cmp(&(&b.file, b.block.opening.start.line))
	});
	let count = orphans.len();

	for consumer in orphans {
		let suggestions: Vec<String> = suggest_similar_provider_names(
			&consumer.block.name,
			ctx.project.providers.keys().map(String::as_str),
		)
		.into_iter()
		.map(str::to_string)
		.collect();
		let location = format!(
			"{}:{}:{}",
			relative_display_path(&consumer.file, root),
			consumer.block.opening.start.line,
			consumer.block.opening.start.column
		);
		eprintln!(
			"{} {}",
			styled!(stderr, "warning:", yellow_bold),
			orphan_description(&consumer.block.name, &location, &suggestions)
		);
	}

	count
}

/// How a single `mdt check` run ended.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CheckStatus {
	/// Every consumer is linked and up to date.
	Clean,
	/// Stale, orphan, or unrenderable consumers (exit status 1).
	Failed,
	/// Validation errors stopped the check (exit status 2).
	Invalid,
}

fn run_check(
	args: &MdtCli,
	show_diff: bool,
	format: OutputFormat,
	watch: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	let status = run_check_once(args, show_diff, format)?;

	if !watch {
		match status {
			CheckStatus::Clean => return Ok(()),
			CheckStatus::Failed => process::exit(1),
			CheckStatus::Invalid => process::exit(2),
		}
	}

	// Watch mode. Status messages go to stderr so `--format json` output on
	// stdout stays machine-readable.
	let root = resolve_root(args);
	let (tx, mut rx) = mpsc::channel();

	// Keep the watcher guard alive for the lifetime of the loop.
	let _watcher = spawn_watcher(&root, tx)?;

	watch_loop(&mut rx, "checking", || {
		run_check_once(args, show_diff, format).map(|_| ())
	});

	Ok(())
}

/// Run `rerun` after every watched change, until the watcher is dropped and
/// its sender closes. Status messages go to stderr so stdout stays
/// machine-readable for `--format json` and piped output.
fn watch_loop(
	rx: &mut mpsc::Receiver<()>,
	verb: &str,
	mut rerun: impl FnMut() -> Result<(), Box<dyn std::error::Error>>,
) {
	eprintln!("\nWatching for file changes... (press Ctrl+C to stop)");

	while rx.recv().is_ok() {
		// Debounce: drain additional events within 200ms.
		while rx.recv_timeout(Duration::from_millis(200)).is_ok() {}

		eprintln!("\nFile change detected, {verb}...");

		if let Err(e) = rerun() {
			eprintln!("{} {e}", styled!(stderr, "error:", red_bold));
		}
	}
}

/// Build a recursive file watcher for `root` that signals the sender on
/// content changes.
///
/// Events inside `<root>/.mdt/` are ignored: every scan rewrites the index
/// cache artifact there, so watching it would make each run trigger the
/// next — an endless check/update loop in watch mode.
fn spawn_watcher(
	root: &Path,
	tx: mpsc::Sender<()>,
) -> Result<notify::RecommendedWatcher, Box<dyn std::error::Error>> {
	use notify::Watcher as _;

	let cache_dir = root.join(".mdt");
	let mut watcher =
		notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
			if let Ok(event) = res {
				if matches!(
					event.kind,
					notify::EventKind::Modify(_) | notify::EventKind::Create(_)
				) {
					let outside_cache = event
						.paths
						.iter()
						.any(|path| path.strip_prefix(&cache_dir).is_err());

					if outside_cache {
						let _ = tx.send(());
					}
				}
			}
		})?;
	watcher.watch(root, notify::RecursiveMode::Recursive)?;
	Ok(watcher)
}

fn diagnostic_json(
	diag: &ProjectDiagnostic,
	root: &Path,
	options: &ValidationOptions,
) -> serde_json::Value {
	serde_json::json!({
		"severity": if diag.is_error(options) { "error" } else { "warning" },
		"code": diag.kind.code(),
		"file": relative_display_path(&diag.file, root),
		"line": diag.line,
		"column": diag.column,
		"message": diag.message(),
	})
}

/// Run a single check and report how it ended.
fn run_check_once(
	args: &MdtCli,
	show_diff: bool,
	format: OutputFormat,
) -> Result<CheckStatus, Box<dyn std::error::Error>> {
	let ctx = match (scan(args), format) {
		(Ok(ctx), _) => ctx,
		(Err(error), OutputFormat::Text) => return Err(error),
		// Machine formats report scan failures (bad config, duplicate
		// providers, unreadable files) in their own shape instead of
		// leaving stdout empty.
		(Err(error), OutputFormat::Json) => {
			let output = serde_json::json!({
				"ok": false,
				"stale": [],
				"stale_files": [],
				"orphans": [],
				"errors": [],
				"diagnostics": [{
					"severity": "error",
					"code": error_code(error.as_ref()),
					"file": null,
					"line": null,
					"column": null,
					"message": error.to_string(),
				}],
			});
			println!("{output}");
			return Ok(CheckStatus::Invalid);
		}
		(Err(error), OutputFormat::Github) => {
			println!("{}", github_annotation("error", None, &error.to_string()));
			return Ok(CheckStatus::Invalid);
		}
	};

	let root = resolve_root(args);
	let options = validation_options(args);

	let has_errors = match format {
		OutputFormat::Text => report_diagnostics(args, &ctx),
		OutputFormat::Json | OutputFormat::Github => has_error_diagnostics(args, &ctx),
	};

	let visible_diagnostics: Vec<&ProjectDiagnostic> = sorted_diagnostics(&ctx, &root)
		.into_iter()
		.filter(|diag| diag.is_error(&options) || !diag.is_ignored(&options))
		.collect();

	if has_errors {
		match format {
			OutputFormat::Text => return Err(validation_failed()),
			OutputFormat::Json => {
				let diagnostics: Vec<_> = visible_diagnostics
					.iter()
					.map(|diag| diagnostic_json(diag, &root, &options))
					.collect();
				let output = serde_json::json!({
					"ok": false,
					"stale": [],
					"stale_files": [],
					"orphans": [],
					"errors": [],
					"diagnostics": diagnostics,
				});
				println!("{output}");
			}
			OutputFormat::Github => {
				for diag in &visible_diagnostics {
					print_github_diagnostic(diag, &options);
				}

				eprintln!("Check aborted by validation errors.");
			}
		}

		return Ok(CheckStatus::Invalid);
	}

	let result = check_project(&ctx)?;

	// Always print template variable warnings (they don't affect exit code).
	if !result.warnings.is_empty() {
		print_template_warnings(&result.warnings, &root);
	}

	match format {
		OutputFormat::Json => {
			let stale_entries: Vec<serde_json::Value> = sorted_stale_entries(&result, &root)
				.into_iter()
				.map(|entry| {
					serde_json::json!({
						"file": relative_display_path(&entry.file, &root),
						"block": entry.block_name,
						"line": entry.line,
						"column": entry.column,
					})
				})
				.collect();
			let stale_file_entries: Vec<serde_json::Value> = sorted_stale_files(&result, &root)
				.into_iter()
				.map(
					|entry| serde_json::json!({ "file": relative_display_path(&entry.file, &root) }),
				)
				.collect();
			let orphan_entries: Vec<serde_json::Value> = result
				.orphans
				.iter()
				.map(|orphan| {
					serde_json::json!({
						"file": relative_display_path(&orphan.file, &root),
						"block": orphan.block_name,
						"line": orphan.line,
						"column": orphan.column,
						"suggestions": orphan.suggestions,
					})
				})
				.collect();
			let error_entries: Vec<serde_json::Value> = sorted_render_errors(&result, &root)
				.into_iter()
				.map(|err| {
					serde_json::json!({
						"file": relative_display_path(&err.file, &root),
						"block": err.block_name,
						"line": err.line,
						"column": err.column,
						"message": err.message,
					})
				})
				.collect();
			let diagnostics: Vec<_> = visible_diagnostics
				.iter()
				.map(|diag| diagnostic_json(diag, &root, &options))
				.chain(
					template_warning_locations(&result.warnings, &ctx)
						.into_iter()
						.map(|(warning, location)| {
							serde_json::json!({
								"severity": "warning",
								"code": "mdt::undefined_variables",
								"file": relative_display_path(&warning.provider_file, &root),
								"line": location.map(|(_, line, _)| line),
								"column": location.map(|(_, _, column)| column),
								"message": template_warning_message(warning, &root),
							})
						}),
				)
				.collect();
			let output = serde_json::json!({
				"ok": result.is_ok(),
				"stale": stale_entries,
				"stale_files": stale_file_entries,
				"orphans": orphan_entries,
				"errors": error_entries,
				"diagnostics": diagnostics,
			});
			println!("{output}");
		}
		OutputFormat::Github => {
			for diag in &visible_diagnostics {
				print_github_diagnostic(diag, &options);
			}
			for (warning, location) in template_warning_locations(&result.warnings, &ctx) {
				let message = template_warning_message(warning, &root);
				println!("{}", github_annotation("warning", location, &message));
			}
			for err in sorted_render_errors(&result, &root) {
				let message = format!(
					"Template render failed for block `{}`: {}",
					err.block_name, err.message
				);
				println!(
					"{}",
					github_annotation("error", Some((&err.file, err.line, err.column)), &message)
				);
			}
			for orphan in &result.orphans {
				let location = format!(
					"{}:{}:{}",
					relative_display_path(&orphan.file, &root),
					orphan.line,
					orphan.column
				);
				let message =
					orphan_description(&orphan.block_name, &location, &orphan.suggestions);
				println!(
					"{}",
					github_annotation(
						"error",
						Some((&orphan.file, orphan.line, orphan.column)),
						&message
					)
				);
			}
			for entry in sorted_stale_entries(&result, &root) {
				let message = format!(
					"Consumer block `{}` is out of date; run `mdt update`",
					entry.block_name
				);
				println!(
					"{}",
					github_annotation(
						"error",
						Some((&entry.file, entry.line, entry.column)),
						&message
					)
				);
			}
			for entry in sorted_stale_files(&result, &root) {
				println!(
					"{}",
					github_annotation(
						"error",
						Some((&entry.file, 1, 1)),
						"Formatter-normalized file output is out of date; run `mdt update`"
					)
				);
			}
			if result.is_ok() {
				println!("All consumer blocks are up to date.");
			} else {
				eprintln!("{}", check_summary(&result));
			}
		}
		OutputFormat::Text => {
			if result.is_ok() {
				println!(
					"{}",
					styled!(
						stdout,
						"Check passed: all consumer blocks are up to date.",
						green_bold
					)
				);
			} else {
				print_check_failure(&result, &root, show_diff);
			}
		}
	}

	Ok(if result.is_ok() {
		CheckStatus::Clean
	} else {
		CheckStatus::Failed
	})
}

fn print_github_diagnostic(diag: &ProjectDiagnostic, options: &ValidationOptions) {
	let level = if diag.is_error(options) {
		"error"
	} else {
		"warning"
	};

	println!(
		"{}",
		github_annotation(
			level,
			Some((&diag.file, diag.line, diag.column)),
			&diag.message()
		)
	);
}

/// A source location: file, 1-indexed line, and 1-indexed column.
type Location<'a> = (&'a Path, usize, usize);

/// A GitHub Actions workflow command. File paths are relative to the
/// repository checkout — the nearest enclosing git work tree, or the working
/// directory when no git repository encloses the file — so annotations land
/// on the right file for `--path sub` runs and for steps with a
/// `working-directory:`. Messages are escaped so multi-line errors stay one
/// command.
fn github_annotation(level: &str, location: Option<Location<'_>>, message: &str) -> String {
	let message = message
		.replace('%', "%25")
		.replace('\r', "%0D")
		.replace('\n', "%0A");
	let Some((file, line, column)) = location else {
		return format!("::{level}::{message}");
	};
	format!(
		"::{level} file={},line={line},col={column}::{message}",
		github_annotation_path(file)
	)
}

/// Path of `file` the way GitHub reads annotation paths: relative to the
/// repository checkout. The nearest enclosing git work tree covers both
/// `--path sub` runs from the checkout root and runs from inside a
/// sub-project (`working-directory:` steps); outside a git repository the
/// working directory is the best available anchor.
fn github_annotation_path(file: &Path) -> String {
	// Compare canonical paths: the working directory is physical, while a
	// `--path` may go through a symlink (macOS `/var` is `/private/var`).
	let canonical = |path: &Path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
	let file = canonical(file);
	if let Some(root) = git_work_tree_root(&file) {
		return display_path(file.strip_prefix(&root).unwrap_or(&file));
	}
	let cwd = canonical(&std::env::current_dir().unwrap_or_default());
	file.strip_prefix(&cwd)
		.map_or_else(|_| display_path(&file), display_path)
}

/// Nearest ancestor directory of `file` that holds a `.git` entry (a
/// directory for normal repositories, a file for linked work trees and
/// submodules).
fn git_work_tree_root(file: &Path) -> Option<PathBuf> {
	let mut dir = file.parent()?.to_path_buf();
	loop {
		if dir.join(".git").exists() {
			return Some(dir);
		}
		if !dir.pop() {
			return None;
		}
	}
}

/// The diagnostic code of an error, when it is one of mdt's.
fn error_code(error: &(dyn std::error::Error + 'static)) -> String {
	error
		.downcast_ref::<MdtError>()
		.and_then(|error| miette::Diagnostic::code(error).map(|code| code.to_string()))
		.unwrap_or_else(|| "mdt::error".to_string())
}

/// Each template warning with the location of its provider's opening tag.
fn template_warning_locations<'a>(
	warnings: &'a [TemplateWarning],
	ctx: &'a ProjectContext,
) -> Vec<(&'a TemplateWarning, Option<Location<'a>>)> {
	warnings
		.iter()
		.map(|warning| {
			let location = ctx
				.project
				.providers
				.get(&warning.block_name)
				.map(|provider| {
					(
						provider.file.as_path(),
						provider.block.opening.start.line,
						provider.block.opening.start.column,
					)
				});
			(warning, location)
		})
		.collect()
}

fn print_check_failure(result: &mdt_core::CheckResult, root: &Path, show_diff: bool) {
	eprintln!("{}", styled!(stderr, "Check failed.", red_bold));
	let counts = [
		("render errors:", result.render_errors.len()),
		("orphan consumers:", result.orphans.len()),
		("stale consumers:", result.stale.len()),
		("stale files:", result.stale_files.len()),
	];

	for (label, count) in counts {
		if count > 0 {
			eprintln!(
				"  {} {}",
				styled!(stderr, label, yellow_bold),
				styled!(stderr, count.to_string(), yellow)
			);
		}
	}

	let sorted_errors = sorted_render_errors(result, root);

	if !sorted_errors.is_empty() {
		eprintln!();
		eprintln!("{}", styled!(stderr, "Render errors:", red_bold));

		for err in sorted_errors {
			let rel = relative_display_path(&err.file, root);
			eprintln!(
				"  block {} at {}:{}:{}: {}",
				styled!(stderr, format!("`{}`", err.block_name), yellow),
				styled!(stderr, rel, cyan),
				err.line,
				err.column,
				styled!(stderr, &err.message, red)
			);
		}
	}

	if !result.orphans.is_empty() {
		eprintln!();
		eprintln!("{}", styled!(stderr, "Orphan consumers:", yellow_bold));

		for orphan in &result.orphans {
			let location = format!(
				"{}:{}:{}",
				relative_display_path(&orphan.file, root),
				orphan.line,
				orphan.column
			);
			eprintln!(
				"  {}",
				orphan_description(&orphan.block_name, &location, &orphan.suggestions)
			);
		}
	}

	let sorted_stale = sorted_stale_entries(result, root);

	if !sorted_stale.is_empty() {
		eprintln!();
		eprintln!("{}", styled!(stderr, "Stale consumers:", yellow_bold));

		for entry in sorted_stale {
			let rel = relative_display_path(&entry.file, root);
			eprintln!(
				"  block {} at {}:{}:{}",
				styled!(stderr, format!("`{}`", entry.block_name), yellow),
				styled!(stderr, rel, cyan),
				entry.line,
				entry.column
			);

			if show_diff {
				print_diff(&entry.current_content, &entry.expected_content);
			}
		}
	}

	let sorted_stale_files = sorted_stale_files(result, root);

	if !sorted_stale_files.is_empty() {
		eprintln!();
		eprintln!("{}", styled!(stderr, "Stale files:", yellow_bold));

		for entry in sorted_stale_files {
			let rel = relative_display_path(&entry.file, root);
			eprintln!("  file {}", styled!(stderr, rel, cyan));

			if show_diff {
				print_diff(&entry.current_content, &entry.expected_content);
			}
		}
	}

	eprintln!();
	eprintln!("{}", check_summary(result));
}

/// Summarize a failed check and say how to fix each kind of failure.
fn check_summary(result: &mdt_core::CheckResult) -> String {
	let mut problems = Vec::new();
	let mut fixes = Vec::new();

	if !result.render_errors.is_empty() {
		problems.push(format!("{} render error(s)", result.render_errors.len()));
		fixes.push("fix the provider templates named above");
	}

	if !result.orphans.is_empty() {
		problems.push(format!(
			"{} consumer block(s) have no provider",
			result.orphans.len()
		));
		fixes.push("rename those consumers or define the providers in `*.t.md` files");
	}

	let stale = result.stale.len() + result.stale_files.len();

	if stale > 0 {
		if !result.stale.is_empty() {
			problems.push(format!(
				"{} consumer block(s) are out of date",
				result.stale.len()
			));
		}

		if !result.stale_files.is_empty() {
			problems.push(format!(
				"{} formatter-normalized file(s) are out of date",
				result.stale_files.len()
			));
		}

		fixes.push("run `mdt update`");
	}

	let mut fix_text = fixes.join(", then ");

	if let Some(first) = fix_text.get(..1) {
		fix_text = first.to_uppercase() + &fix_text[1..];
	}

	format!("{}. {fix_text}.", problems.join(" and "))
}

fn sorted_stale_entries<'a>(
	result: &'a mdt_core::CheckResult,
	root: &Path,
) -> Vec<&'a mdt_core::StaleEntry> {
	let mut stale_entries: Vec<_> = result.stale.iter().collect();
	stale_entries.sort_by(|a, b| {
		relative_display_path(&a.file, root)
			.cmp(&relative_display_path(&b.file, root))
			.then_with(|| a.line.cmp(&b.line))
			.then_with(|| a.column.cmp(&b.column))
			.then_with(|| a.block_name.cmp(&b.block_name))
	});
	stale_entries
}

fn sorted_stale_files<'a>(
	result: &'a mdt_core::CheckResult,
	root: &Path,
) -> Vec<&'a mdt_core::StaleFileEntry> {
	let mut stale_files: Vec<_> = result.stale_files.iter().collect();
	stale_files.sort_by(|a, b| {
		relative_display_path(&a.file, root).cmp(&relative_display_path(&b.file, root))
	});
	stale_files
}

fn run_update(
	args: &MdtCli,
	dry_run: bool,
	watch: bool,
	blocks: &[String],
	files: &[PathBuf],
) -> Result<(), Box<dyn std::error::Error>> {
	let succeeded = run_update_once(args, dry_run, blocks, files)?;

	if !watch {
		if !succeeded {
			process::exit(1);
		}

		return Ok(());
	}

	// Watch mode. Status messages go to stderr so update output on stdout
	// stays machine-readable, matching `check --watch`.
	let root = resolve_root(args);
	let (tx, mut rx) = mpsc::channel();

	// Keep the watcher guard alive for the lifetime of the loop.
	let _watcher = spawn_watcher(&root, tx)?;

	watch_loop(&mut rx, "updating", || {
		run_update_once(args, false, blocks, files).map(|_| ())
	});

	Ok(())
}

/// Resolve `--block` and `--file` arguments into an [`UpdateScope`] against the
/// scanned project. File paths resolve against the current directory and must
/// stay inside the project root and match a scanned file with consumers.
fn resolve_update_scope(
	ctx: &ProjectContext,
	root: &Path,
	blocks: &[String],
	files: &[PathBuf],
) -> Result<UpdateScope, Box<dyn std::error::Error>> {
	let mut scoped_files: Option<HashSet<PathBuf>> = None;

	if !files.is_empty() {
		let scanned: HashSet<&Path> = ctx
			.project
			.consumers
			.iter()
			.map(|consumer| consumer.file.as_path())
			.collect();
		let mut resolved: HashSet<PathBuf> = HashSet::new();

		for file in files {
			let absolute = normalize_lexically(&std::path::absolute(file)?);

			if !absolute.starts_with(root) {
				return Err(format!(
					"file `{}` is outside the project root {}",
					display_path(file),
					display_path(root)
				)
				.into());
			}

			if !scanned.contains(absolute.as_path()) {
				return Err(format!(
					"no scanned file with consumer blocks matches `{}`; run `mdt list` to see \
					 every consumer file",
					display_path(file)
				)
				.into());
			}

			resolved.insert(absolute);
		}

		scoped_files = Some(resolved);
	}

	let mut scope = UpdateScope::all();

	if !blocks.is_empty() {
		scope = scope.with_blocks(blocks.iter().cloned());
	}

	if let Some(files) = scoped_files {
		scope = scope.with_files(files);
	}

	Ok(scope)
}

/// Describe the scope for the `scoped to ...` line, or `None` when the update
/// covers every consumer.
fn describe_update_scope(root: &Path, blocks: &[String], files: &[PathBuf]) -> Option<String> {
	let mut parts = Vec::new();

	if !blocks.is_empty() {
		let mut names: Vec<&str> = Vec::new();
		for name in blocks {
			if !names.contains(&name.as_str()) {
				names.push(name);
			}
		}
		parts.push(format!("{} block(s): {}", names.len(), names.join(", ")));
	}

	if !files.is_empty() {
		let mut paths: Vec<String> = Vec::new();
		for file in files {
			let display = relative_display_path(file, root);
			if !paths.contains(&display) {
				paths.push(display);
			}
		}
		parts.push(format!("{} file(s): {}", paths.len(), paths.join(", ")));
	}

	(!parts.is_empty()).then(|| parts.join("; "))
}

/// Run a single update. Returns `false` when some consumers could not be
/// updated because their provider failed to render.
fn run_update_once(
	args: &MdtCli,
	dry_run: bool,
	blocks: &[String],
	files: &[PathBuf],
) -> Result<bool, Box<dyn std::error::Error>> {
	let ctx = scan(args)?;
	let root = resolve_root(args);

	if report_diagnostics(args, &ctx) {
		return Err(validation_failed());
	}

	let scope = resolve_update_scope(&ctx, &root, blocks, files)?;
	let scoped = !blocks.is_empty() || !files.is_empty();

	if let Some(description) = describe_update_scope(&root, blocks, files) {
		println!("scoped to {description}");
	}

	let orphan_count = warn_orphans(&ctx, &root);
	let updates = compute_updates(&ctx, &scope)?;

	// Print template variable warnings (they don't prevent updates).
	if !updates.warnings.is_empty() {
		print_template_warnings(&updates.warnings, &root);
	}

	let mut render_errors: Vec<_> = updates.render_errors.iter().collect();
	render_errors.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));

	for err in &render_errors {
		eprintln!(
			"{} block `{}` at {}:{}:{} was not updated: {}",
			styled!(stderr, "error:", red_bold),
			err.block_name,
			relative_display_path(&err.file, &root),
			err.line,
			err.column,
			err.message
		);
	}

	if updates.updated_files.is_empty() {
		if render_errors.is_empty() {
			let linked = if orphan_count > 0 { "linked " } else { "" };
			let in_scope = if scoped { "in-scope " } else { "" };
			println!("All {linked}{in_scope}consumer blocks are already up to date.");
		}

		return Ok(render_errors.is_empty());
	}

	if dry_run {
		if updates.updated_count == 0 {
			println!(
				"Dry run: would normalize {} file(s) via formatter integration:",
				updates.updated_files.len()
			);
		} else {
			println!(
				"Dry run: would update {} block(s) in {} file(s):",
				updates.updated_count,
				updates.updated_files.len()
			);
		}

		let mut paths: Vec<_> = updates.updated_files.keys().collect();
		paths.sort();

		for path in paths {
			let rel = relative_display_path(path, &root);
			println!("  {rel}");
		}
	} else {
		write_updates(&updates)?;

		if updates.updated_count == 0 {
			println!(
				"Normalized {} file(s) via formatter integration.",
				updates.updated_files.len()
			);
		} else {
			println!(
				"Updated {} block(s) in {} file(s).",
				updates.updated_count,
				updates.updated_files.len()
			);
		}

		if args.verbose {
			let mut paths: Vec<_> = updates.updated_files.keys().collect();
			paths.sort();

			for path in paths {
				let rel = relative_display_path(path, &root);
				println!("  {rel}");
			}
		}
	}

	Ok(render_errors.is_empty())
}

/// List every block. Validation errors are reported but never hide the
/// listing — it is the tool for diagnosing them — and still exit with
/// status 2 afterwards.
fn run_list(args: &MdtCli) -> Result<(), Box<dyn std::error::Error>> {
	let ctx = scan(args)?;
	let root = resolve_root(args);
	let has_errors = report_diagnostics(args, &ctx);

	if ctx.project.providers.is_empty() && ctx.project.consumers.is_empty() {
		println!("No provider or consumer blocks found.");

		return if has_errors {
			Err(validation_failed())
		} else {
			Ok(())
		};
	}

	// Providers
	if !ctx.project.providers.is_empty() {
		println!("{}", styled!(stdout, "Providers:", bold));
		// One pass over consumers instead of a scan per provider.
		let mut consumer_counts: HashMap<&str, usize> = HashMap::new();

		for consumer in &ctx.project.consumers {
			if consumer.block.r#type == BlockType::Consumer {
				*consumer_counts
					.entry(consumer.block.name.as_str())
					.or_default() += 1;
			}
		}

		let mut names: Vec<_> = ctx.project.providers.keys().collect();
		names.sort();

		for name in names {
			let entry = &ctx.project.providers[name];
			let rel = relative_display_path(&entry.file, &root);
			let line = entry.block.opening.start.line;
			let consumer_count = consumer_counts.get(name.as_str()).copied().unwrap_or(0);
			println!("  @{name} {rel}:{line} ({consumer_count} consumer(s))");
		}
	}

	// Consumers
	if !ctx.project.consumers.is_empty() {
		if !ctx.project.providers.is_empty() {
			println!();
		}

		println!("{}", styled!(stdout, "Consumers:", bold));

		for consumer in &ctx.project.consumers {
			let rel = relative_display_path(&consumer.file, &root);

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
				let described: Vec<String> = consumer
					.block
					.transformers
					.iter()
					.map(describe_transformer)
					.collect();
				format!(" |{}", described.join("|"))
			};

			println!(
				"  {sigil}{} {rel}:{}{transformers} [{status}]",
				consumer.block.name, consumer.block.opening.start.line
			);
		}
	}

	// Summary
	println!(
		"\n{} provider(s), {} consumer(s)",
		ctx.project.providers.len(),
		ctx.project.consumers.len()
	);

	if has_errors {
		return Err(validation_failed());
	}

	Ok(())
}

/// A transformer as written in a tag, e.g. `linePrefix:"/// ":true`.
fn describe_transformer(transformer: &mdt_core::Transformer) -> String {
	let arguments = transformer.args.iter().map(|argument| {
		match argument {
			mdt_core::Argument::String(value) => format!(":{value:?}"),
			mdt_core::Argument::Number(value) => format!(":{value}"),
			mdt_core::Argument::Boolean(value) => format!(":{value}"),
			_ => ":?".to_string(),
		}
	});
	std::iter::once(transformer.r#type.to_string())
		.chain(arguments)
		.collect()
}

fn run_info(args: &MdtCli, format: InfoOutputFormat) -> Result<(), Box<dyn std::error::Error>> {
	let root = resolve_root(args);
	let options = validation_options(args);
	let report = mdt_core::info::info_report(&root, &options)?;

	let configured_template_dirs_display = if report.templates.configured_dirs.is_empty() {
		"default scan (*.t.md)".to_string()
	} else {
		report.templates.configured_dirs.join(", ")
	};

	match format {
		InfoOutputFormat::Json => {
			println!("{}", serde_json::to_string_pretty(&report)?);
		}
		InfoOutputFormat::Text => {
			println!("{}", styled!(stdout, "mdt info", bold));

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
			let cache_status = if !report.cache.artifact.exists {
				"missing".to_string()
			} else if !report.cache.artifact.readable {
				"unreadable".to_string()
			} else if !report.cache.artifact.valid {
				"invalid".to_string()
			} else {
				"ok".to_string()
			};

			print_field("Artifact status", cache_status);
			let schema_display = report.cache.schema_version.map_or_else(
				|| "unknown".to_string(),
				|schema| {
					if report.cache.compatibility.schema_supported {
						format!("{schema} (supported)")
					} else {
						format!("{schema} (unsupported)")
					}
				},
			);
			print_field("Schema version", schema_display);
			print_field(
				"Project key match",
				if report.cache.compatibility.project_key_matches {
					"yes"
				} else {
					"no"
				},
			);
			print_field(
				"Hash verification",
				if report.cache.compatibility.hash_verification_enabled {
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

fn run_doctor(args: &MdtCli, format: DoctorOutputFormat) -> Result<(), Box<dyn std::error::Error>> {
	let root = resolve_root(args);
	let options = validation_options(args);
	let report = mdt_core::doctor::doctor_report(&root, &options);

	match format {
		DoctorOutputFormat::Json => {
			println!("{}", serde_json::to_string_pretty(&report)?);
		}
		DoctorOutputFormat::Text => {
			println!("{}", styled!(stdout, "mdt doctor", bold));
			for check in &report.checks {
				let tag = match check.status {
					DoctorStatus::Pass => styled!(stdout, check.status.tag(), green_bold),
					DoctorStatus::Warn => styled!(stdout, check.status.tag(), yellow_bold),
					DoctorStatus::Fail => styled!(stdout, check.status.tag(), red_bold),
					DoctorStatus::Skip => check.status.tag().to_string(),
				};
				println!("[{tag}] {:<22} {}", check.title, check.message);

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

fn run_mcp(args: &MdtCli) -> Result<(), Box<dyn std::error::Error>> {
	let rt = tokio::runtime::Runtime::new()?;
	rt.block_on(mdt_mcp::run_server_in(resolve_root(args)));
	Ok(())
}

fn assistant_id(assistant: Assistant) -> &'static str {
	match assistant {
		Assistant::Generic => "generic",
		Assistant::Claude => "claude",
		Assistant::Cursor => "cursor",
		Assistant::Copilot => "copilot",
		Assistant::Pi => "pi",
	}
}

fn assistant_display_name(assistant: Assistant) -> &'static str {
	match assistant {
		Assistant::Generic => "Generic MCP client",
		Assistant::Claude => "Claude Code",
		Assistant::Cursor => "Cursor",
		Assistant::Copilot => "GitHub Copilot (VS Code)",
		Assistant::Pi => "Pi",
	}
}

/// Where an assistant reads its MCP configuration, and the snippet to put
/// there. `None` for assistants without a built-in MCP client.
fn assistant_mcp_setup(assistant: Assistant) -> Option<(&'static str, serde_json::Value)> {
	let server = serde_json::json!({ "command": "mdt", "args": ["mcp"] });
	let stdio_server = serde_json::json!({ "type": "stdio", "command": "mdt", "args": ["mcp"] });

	match assistant {
		Assistant::Generic => {
			Some((
				"your client's MCP settings (stdio server)",
				serde_json::json!({ "mcpServers": { "mdt": server } }),
			))
		}
		Assistant::Claude => {
			Some((
				".mcp.json",
				serde_json::json!({ "mcpServers": { "mdt": stdio_server } }),
			))
		}
		Assistant::Cursor => {
			Some((
				".cursor/mcp.json",
				serde_json::json!({ "mcpServers": { "mdt": server } }),
			))
		}
		// VS Code keys servers by `servers`, not `mcpServers`.
		Assistant::Copilot => {
			Some((
				".vscode/mcp.json",
				serde_json::json!({ "servers": { "mdt": stdio_server } }),
			))
		}
		Assistant::Pi => None,
	}
}

/// The directory the assistant loads project skills from, if it has one.
fn assistant_skills_dir(assistant: Assistant) -> Option<&'static str> {
	match assistant {
		Assistant::Claude => Some(".claude/skills"),
		Assistant::Copilot => Some(".github/skills"),
		Assistant::Pi => Some(".pi/skills"),
		// The shared project skills directory (Pi reads it too).
		Assistant::Generic => Some(".agents/skills"),
		Assistant::Cursor => None,
	}
}

fn assistant_setup_payload(assistant: Assistant) -> serde_json::Value {
	let mcp_setup = assistant_mcp_setup(assistant);

	let install_command = match assistant {
		Assistant::Claude => {
			Some("claude mcp add --transport stdio --scope project mdt -- mdt mcp")
		}
		_ => None,
	};

	let skill_install =
		assistant_skills_dir(assistant).map(|dir| format!("mdt skill --install {dir}"));
	let guidance = vec![
		"Before editing mdt templates or synced docs, run `mdt skill` to load the mdt agent skill \
		 (tag syntax, transformers, configuration, and workflow for the installed version)."
			.to_string(),
		"Edit providers in `.templates/*.t.md`, never the synced copies, and prefer reusing an \
		 existing provider (`mdt list` or `mdt_find_reuse`) over creating a new one."
			.to_string(),
		"After documentation edits, run `mdt check`; run `mdt update` when consumers are stale."
			.to_string(),
	];

	let notes = match assistant {
		Assistant::Generic => {
			vec![
				"Add the MCP snippet to any client that supports stdio MCP servers. The server's \
				 project root is the directory it starts in; add `--path <repo>` to `args` for a \
				 user-level config."
					.to_string(),
				"`.agents/skills` is a shared project skills directory; check which directory \
				 your agent loads skills from. Agents without skill support can still learn mdt \
				 from `mdt skill` when the guidance above is in AGENTS.md."
					.to_string(),
			]
		}
		Assistant::Claude => {
			vec![
				"`claude mcp add ... --scope project` writes `.mcp.json` at the repository root; \
				 commit it to share the server with your team."
					.to_string(),
				"`mdt skill --install .claude/skills` adds the skill for this project; use \
				 `~/.claude/skills` to install it for every project."
					.to_string(),
			]
		}
		Assistant::Cursor => {
			vec![
				"Use `.cursor/mcp.json` for this project or `~/.cursor/mcp.json` for every \
				 project."
					.to_string(),
				"Add the guidance above to `.cursor/rules` or AGENTS.md so agents load the skill \
				 with `mdt skill` before editing docs."
					.to_string(),
			]
		}
		Assistant::Copilot => {
			vec![
				"VS Code reads `.vscode/mcp.json`, which keys servers by `servers` (not \
				 `mcpServers`)."
					.to_string(),
				"`mdt skill --install .github/skills` adds the skill for Copilot agents in this \
				 repository."
					.to_string(),
			]
		}
		Assistant::Pi => {
			vec![
				"Pi has no built-in MCP client, so it works with mdt through the CLI and the \
				 skill."
					.to_string(),
				"Install the skill with `mdt skill --install .pi/skills`, or from npm with `pi \
				 install npm:@m-d-t/skills`."
					.to_string(),
			]
		}
	};

	serde_json::json!({
		"id": assistant_id(assistant),
		"assistant": assistant_display_name(assistant),
		"strategy": {
			"type": "official-profile",
			"scope": "config-snippets-and-guidance",
			"summary": "mdt ships assistant setup presets and repo-local guidance, not a plugin marketplace."
		},
		"skill": {
			"print_command": "mdt skill",
			"install_command": skill_install,
		},
		"mcp_config_file": mcp_setup.as_ref().map(|(file, _)| *file),
		"mcp_install_command": install_command,
		"mcp_config": mcp_setup.map(|(_, config)| config),
		"repo_guidance": guidance,
		"notes": notes,
	})
}

fn run_assist(
	assistant: Assistant,
	format: AssistOutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
	let payload = assistant_setup_payload(assistant);

	match format {
		AssistOutputFormat::Json => {
			println!("{}", serde_json::to_string_pretty(&payload)?);
		}
		AssistOutputFormat::Text => {
			println!("{}", styled!(stdout, "mdt assist", bold));
			println!();
			println!("Assistant: {}", assistant_display_name(assistant));
			println!();
			println!("Load the mdt skill:");
			if let Some(command) = payload["skill"]["install_command"].as_str() {
				println!("  {command}");
				println!("  (or run `mdt skill` to print it into the conversation)");
			} else {
				println!("  mdt skill");
			}

			println!();
			println!("MCP server:");
			if let Some(file) = payload["mcp_config_file"].as_str() {
				if let Some(command) = payload["mcp_install_command"].as_str() {
					println!("  {command}");
					println!("  or add to {file}:");
				} else {
					println!("  add to {file}:");
				}

				println!("{}", serde_json::to_string_pretty(&payload["mcp_config"])?);
			} else {
				println!("  not supported by {}", assistant_display_name(assistant));
			}

			println!();
			println!("Suggested repo-local guidance:");
			for item in payload["repo_guidance"].as_array().into_iter().flatten() {
				if let Some(text) = item.as_str() {
					println!("- {text}");
				}
			}

			println!();
			println!("Notes for {}:", assistant_display_name(assistant));
			for item in payload["notes"].as_array().into_iter().flatten() {
				if let Some(text) = item.as_str() {
					println!("- {text}");
				}
			}
		}
	}

	Ok(())
}

fn run_skill(reference: bool, install: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
	let Some(skills_dir) = install else {
		print!(
			"{}",
			if reference {
				skill::REFERENCE_MD
			} else {
				skill::SKILL_MD
			}
		);

		return Ok(());
	};

	let written = skill::install(skills_dir).map_err(|error| {
		format!(
			"failed to install the mdt skill into {}: {error}",
			display_path(skills_dir.join(skill::SKILL_DIR_NAME))
		)
	})?;
	println!("Installed the mdt skill:");

	for path in written {
		println!("  {}", display_path(path));
	}

	Ok(())
}

/// Describe a template warning for humans.
fn template_warning_message(warning: &TemplateWarning, root: &Path) -> String {
	let rel = relative_display_path(&warning.provider_file, root);
	let vars = warning.undefined_variables.join(", ");

	if warning.template_rendered {
		format!(
			"provider block `{}` in {rel} references undefined variable(s): {vars}",
			warning.block_name
		)
	} else {
		format!(
			"provider block `{}` in {rel} uses template variable(s) {vars}, but this project has \
			 no `[data]`, so the text is copied without rendering; declare the namespace(s) under \
			 `[data]` in this project's mdt.toml",
			warning.block_name
		)
	}
}

/// Print warnings about undefined template variables.
fn print_template_warnings(warnings: &[TemplateWarning], root: &Path) {
	let mut sorted_warnings: Vec<_> = warnings.iter().collect();
	sorted_warnings.sort_by(|a, b| {
		relative_display_path(&a.provider_file, root)
			.cmp(&relative_display_path(&b.provider_file, root))
			.then_with(|| a.block_name.cmp(&b.block_name))
	});

	for warning in sorted_warnings {
		eprintln!(
			"{} {}",
			styled!(stderr, "warning:", yellow_bold),
			template_warning_message(warning, root)
		);
	}
}

/// Print a unified diff between two strings, colorized.
fn print_diff(current: &str, expected: &str) {
	let diff = TextDiff::from_lines(current, expected);

	for change in diff.iter_all_changes() {
		match change.tag() {
			ChangeTag::Delete => {
				eprint!("  {}", styled!(stderr, format!("-{change}"), red));
			}
			ChangeTag::Insert => {
				eprint!("  {}", styled!(stderr, format!("+{change}"), green));
			}
			ChangeTag::Equal => {
				eprint!("   {change}");
			}
		}
	}
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

	let mut diag_value = miette::MietteDiagnostic::new(format!("[{location}] {}", diag.message()))
		.with_code(diag.kind.code())
		.with_severity(severity);
	let help = diagnostic_help(&diag.kind);

	if !help.is_empty() {
		diag_value = diag_value.with_help(help);
	}

	miette::Report::new(diag_value)
}

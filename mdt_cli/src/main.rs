mod skill;

use std::collections::BTreeSet;
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

	let result = validate_project_root(&args).and_then(|()| {
		match args.command {
			Commands::Init => run_init(&args),
			Commands::Check {
				diff,
				format,
				watch,
			} => run_check(&args, diff, format, watch),
			Commands::Update { dry_run, watch } => run_update(&args, dry_run, watch),
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

fn resolve_root(args: &MdtCli) -> PathBuf {
	resolve_root_path(args.path.as_deref())
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
	use mdt_core::init::ConfigOutcome;
	use mdt_core::init::GitignoreOutcome;
	use mdt_core::init::SampleOutcome;

	let root = resolve_root(args);
	let report = mdt_core::init::init_project(&root)?;
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
		mdt_core::DataSource::Path(path) => (display_path(path), "file".to_string()),
		mdt_core::DataSource::Typed(typed) => (display_path(&typed.path), "file".to_string()),
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

/// Format a path for user-facing output.
///
/// Forward slashes are used on every platform so printed diagnostics and
/// JSON reports render identically across operating systems.
fn display_path(path: impl AsRef<Path>) -> String {
	path.as_ref().display().to_string().replace('\\', "/")
}

fn normalize_dir_hint(path: &Path) -> String {
	let mut hint = display_path(path);
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
	providers: &HashMap<String, ProviderEntry>,
	consumers: &[ConsumerEntry],
) -> usize {
	consumers
		.iter()
		.filter(|consumer| consumer.block.r#type == BlockType::Consumer)
		.filter(|consumer| !providers.contains_key(&consumer.block.name))
		.count()
}

fn count_unused_providers(
	providers: &HashMap<String, ProviderEntry>,
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
	"validation errors found; fix the errors above (see `mdt doctor` for hints)".into()
}

/// Describe a consumer whose name matches no provider.
fn orphan_description(block_name: &str, location: &str, suggestions: &[String]) -> String {
	let hint = suggestions
		.first()
		.map_or_else(String::new, |name| format!(" (did you mean `{name}`?)"));
	format!("consumer `{block_name}` at {location} has no provider{hint}")
}

/// Warn about consumers whose name matches no provider. `check` reports
/// these as failures instead.
fn warn_orphans(ctx: &ProjectContext, root: &Path) {
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
	eprintln!("\nWatching for file changes... (press Ctrl+C to stop)");

	let root = resolve_root(args);
	let (tx, rx) = mpsc::channel();

	// Keep the watcher guard alive for the lifetime of the loop.
	let _watcher = spawn_watcher(&root, tx)?;

	loop {
		rx.recv()?;
		// Debounce: drain additional events within 200ms.
		while rx.recv_timeout(Duration::from_millis(200)).is_ok() {}

		eprintln!("\nFile change detected, checking...");
		if let Err(e) = run_check_once(args, show_diff, format) {
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
		"code": diagnostic_code(&diag.kind),
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
	let ctx = scan(args)?;
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
					print_github_diagnostic(diag, &root, &options);
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
				print_github_diagnostic(diag, &root, &options);
			}
			for err in sorted_render_errors(&result, &root) {
				let rel = relative_display_path(&err.file, &root);
				println!(
					"::error file={rel},line={},col={}::Template render failed for block `{}`: {}",
					err.line, err.column, err.block_name, err.message
				);
			}
			for orphan in &result.orphans {
				let rel = relative_display_path(&orphan.file, &root);
				let location = format!("{rel}:{}:{}", orphan.line, orphan.column);
				println!(
					"::error file={rel},line={},col={}::{}",
					orphan.line,
					orphan.column,
					orphan_description(&orphan.block_name, &location, &orphan.suggestions)
				);
			}
			for entry in sorted_stale_entries(&result, &root) {
				let rel = relative_display_path(&entry.file, &root);
				println!(
					"::error file={rel},line={},col={}::Consumer block `{}` is out of date; run \
					 `mdt update`",
					entry.line, entry.column, entry.block_name
				);
			}
			for entry in sorted_stale_files(&result, &root) {
				let rel = relative_display_path(&entry.file, &root);
				println!(
					"::error file={rel}::Formatter-normalized file output is out of date; run \
					 `mdt update`"
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

fn print_github_diagnostic(diag: &ProjectDiagnostic, root: &Path, options: &ValidationOptions) {
	let level = if diag.is_error(options) {
		"error"
	} else {
		"warning"
	};
	println!(
		"::{level} file={},line={},col={}::{}",
		relative_display_path(&diag.file, root),
		diag.line,
		diag.column,
		diag.message()
	);
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

fn sorted_render_errors<'a>(
	result: &'a mdt_core::CheckResult,
	root: &Path,
) -> Vec<&'a mdt_core::RenderError> {
	let mut render_errors: Vec<_> = result.render_errors.iter().collect();
	render_errors.sort_by(|a, b| {
		relative_display_path(&a.file, root)
			.cmp(&relative_display_path(&b.file, root))
			.then_with(|| a.line.cmp(&b.line))
			.then_with(|| a.column.cmp(&b.column))
			.then_with(|| a.block_name.cmp(&b.block_name))
	});
	render_errors
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

fn run_update(args: &MdtCli, dry_run: bool, watch: bool) -> Result<(), Box<dyn std::error::Error>> {
	let succeeded = run_update_once(args, dry_run)?;

	if !watch {
		if !succeeded {
			process::exit(1);
		}
		return Ok(());
	}

	// Watch mode
	println!("\nWatching for file changes... (press Ctrl+C to stop)");

	let root = resolve_root(args);
	let (tx, rx) = mpsc::channel();

	// Keep the watcher guard alive for the lifetime of the loop.
	let _watcher = spawn_watcher(&root, tx)?;

	loop {
		rx.recv()?;
		// Debounce: drain additional events within 200ms.
		while rx.recv_timeout(Duration::from_millis(200)).is_ok() {}

		println!("\nFile change detected, updating...");
		if let Err(e) = run_update_once(args, false) {
			eprintln!("{} {e}", styled!(stderr, "error:", red_bold));
		}
	}
}

/// Run a single update. Returns `false` when some consumers could not be
/// updated because their provider failed to render.
fn run_update_once(args: &MdtCli, dry_run: bool) -> Result<bool, Box<dyn std::error::Error>> {
	let ctx = scan(args)?;
	let root = resolve_root(args);
	if report_diagnostics(args, &ctx) {
		return Err(validation_failed());
	}
	warn_orphans(&ctx, &root);
	let updates = compute_updates(&ctx)?;

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
			println!("All consumer blocks are already up to date.");
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
				let names: Vec<String> = consumer
					.block
					.transformers
					.iter()
					.map(|t| t.r#type.to_string())
					.collect();
				format!(" |{}", names.join("|"))
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
struct InfoCacheArtifactStateSection {
	exists: bool,
	readable: bool,
	valid: bool,
}

#[derive(serde::Serialize)]
struct InfoCacheCompatibilityStateSection {
	schema_supported: bool,
	project_key_matches: bool,
	hash_verification_enabled: bool,
}

#[derive(serde::Serialize)]
struct InfoCacheSection {
	path: String,
	#[serde(flatten)]
	artifact: InfoCacheArtifactStateSection,
	schema_version: Option<u32>,
	#[serde(flatten)]
	compatibility: InfoCacheCompatibilityStateSection,
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
		.map(|entry| relative_display_path(&entry.file, &root))
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
	let configured_template_dirs: Vec<String> =
		config.template_dirs.iter().map(display_path).collect();
	let configured_template_dirs_display = if configured_template_dirs.is_empty() {
		"default scan (*.t.md)".to_string()
	} else {
		configured_template_dirs.join(", ")
	};

	let resolved_config = config
		.path
		.as_ref()
		.map_or_else(|| "none".to_string(), display_path);

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
			root: display_path(&root),
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
			path: display_path(&cache_inspection.path),
			artifact: InfoCacheArtifactStateSection {
				exists: cache_inspection.artifact.exists,
				readable: cache_inspection.artifact.readable,
				valid: cache_inspection.artifact.valid,
			},
			schema_version: cache_inspection.schema_version,
			compatibility: InfoCacheCompatibilityStateSection {
				schema_supported: cache_inspection.compatibility.schema_supported,
				project_key_matches: cache_inspection.compatibility.project_key_matches,
				hash_verification_enabled: cache_inspection.compatibility.hash_verification_enabled,
			},
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
			Self::Pass => styled!(stdout, self.tag(), green_bold),
			Self::Warn => styled!(stdout, self.tag(), yellow_bold),
			Self::Fail => styled!(stdout, self.tag(), red_bold),
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
			format!("resolved config at {}", display_path(path)),
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
			.map(display_path)
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
	// Block checks scan without loading `[data]`, so a broken data file does
	// not hide unrelated block problems.
	match mdt_core::project::scan_project_with_options(&root, &scan_options) {
		Ok(project) => add_block_checks(&mut checks, &project, &root, &options),
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

	add_sync_check(&mut checks, &root);

	let cache = inspect_project_cache(&root, &scan_options);
	if !cache.artifact.exists {
		add_doctor_check(
			&mut checks,
			"cache_artifact",
			"Cache Artifact",
			DoctorStatus::Warn,
			format!("cache artifact not found at {}", display_path(&cache.path)),
			Some(
				"run `mdt check` or `mdt info` to trigger a scan and write the cache artifact"
					.to_string(),
			),
		);
	} else if !cache.artifact.readable {
		add_doctor_check(
			&mut checks,
			"cache_artifact",
			"Cache Artifact",
			DoctorStatus::Fail,
			format!(
				"cache artifact exists but is not readable: {}",
				display_path(&cache.path)
			),
			Some("verify filesystem permissions for `.mdt/cache/`".to_string()),
		);
	} else if !cache.artifact.valid {
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
	} else if !cache.compatibility.project_key_matches {
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
				display_path(&cache.path)
			),
			None,
		);
	}

	let hash_mode_message = if cache.compatibility.hash_verification_enabled {
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
		Some(cache_hash_mode_hint(
			cache.compatibility.hash_verification_enabled,
		)),
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
					Some(cache_hash_mode_hint(
						cache.compatibility.hash_verification_enabled,
					)),
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
			println!("{}", styled!(stdout, "mdt doctor", bold));
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
/// Checks that only need the scanned blocks: provider uniqueness, orphan
/// consumers, unused providers, and parser diagnostics.
fn add_block_checks(
	checks: &mut Vec<DoctorCheck>,
	project: &mdt_core::project::Project,
	root: &Path,
	options: &ValidationOptions,
) {
	add_doctor_check(
		checks,
		"duplicate_providers",
		"Duplicate Providers",
		DoctorStatus::Pass,
		"provider names are unique".to_string(),
		None,
	);

	let mut orphans: Vec<&ConsumerEntry> = project
		.consumers
		.iter()
		.filter(|consumer| {
			consumer.block.r#type == BlockType::Consumer
				&& !project.providers.contains_key(&consumer.block.name)
		})
		.collect();
	orphans.sort_by(|a, b| {
		(&a.file, a.block.opening.start.line).cmp(&(&b.file, b.block.opening.start.line))
	});
	if orphans.is_empty() {
		add_doctor_check(
			checks,
			"orphan_consumers",
			"Orphan Consumers",
			DoctorStatus::Pass,
			"every consumer block has a provider".to_string(),
			None,
		);
	} else {
		let described: Vec<String> = orphans
			.iter()
			.take(5)
			.map(|consumer| {
				let suggestions: Vec<String> = suggest_similar_provider_names(
					&consumer.block.name,
					project.providers.keys().map(String::as_str),
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
				orphan_description(&consumer.block.name, &location, &suggestions)
			})
			.collect();
		let more = orphans.len().saturating_sub(described.len());
		let mut message = format!(
			"{} consumer block(s) have no provider: {}",
			orphans.len(),
			described.join("; ")
		);
		if more > 0 {
			message = format!("{message}; and {more} more");
		}
		let hint = if project.providers.is_empty() {
			"no providers were found at all: mdt only reads providers from `*.t.md` files, so \
			 check that your template files end in `.t.md`"
		} else {
			"rename the consumers to match a provider, or define the providers in `*.t.md` files"
		};
		add_doctor_check(
			checks,
			"orphan_consumers",
			"Orphan Consumers",
			DoctorStatus::Fail,
			message,
			Some(hint.to_string()),
		);
	}

	let unused_provider_count = count_unused_providers(&project.providers, &project.consumers);
	if unused_provider_count == 0 {
		add_doctor_check(
			checks,
			"unused_providers",
			"Unused Providers",
			DoctorStatus::Pass,
			"all providers have at least one consumer".to_string(),
			None,
		);
	} else {
		add_doctor_check(
			checks,
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

	// Unused providers have their own check above.
	let mut diagnostics: Vec<&ProjectDiagnostic> = project
		.diagnostics
		.iter()
		.filter(|diag| !matches!(diag.kind, DiagnosticKind::UnusedProvider { .. }))
		.filter(|diag| !diag.is_ignored(options))
		.collect();
	diagnostics.sort_by_key(|diag| !diag.is_error(options));
	let errors = diagnostics
		.iter()
		.filter(|diag| diag.is_error(options))
		.count();
	let warnings = diagnostics.len() - errors;
	let Some(first) = diagnostics.first() else {
		add_doctor_check(
			checks,
			"parser_diagnostics",
			"Parser Diagnostics",
			DoctorStatus::Pass,
			"no parser diagnostics found".to_string(),
			None,
		);
		return;
	};
	let first_description = format!(
		"{}:{}:{} {}",
		relative_display_path(&first.file, root),
		first.line,
		first.column,
		first.message()
	);
	let status = if errors > 0 {
		DoctorStatus::Fail
	} else {
		DoctorStatus::Warn
	};
	let help = diagnostic_help(&first.kind);
	add_doctor_check(
		checks,
		"parser_diagnostics",
		"Parser Diagnostics",
		status,
		format!("{errors} error(s), {warnings} warning(s); first: {first_description}"),
		Some(if help.is_empty() {
			"run `mdt check` to see every diagnostic with its location".to_string()
		} else {
			format!("{help}. Run `mdt check` to see every diagnostic")
		}),
	);
}

/// Render every consumer (with `[data]`) to catch template errors and
/// report whether the project is in sync.
fn add_sync_check(checks: &mut Vec<DoctorCheck>, root: &Path) {
	let result = scan_project_with_config(root).and_then(|ctx| check_project(&ctx));
	let result = match result {
		Ok(result) => result,
		Err(error) => {
			add_doctor_check(
				checks,
				"consumer_sync",
				"Consumer Sync",
				DoctorStatus::Skip,
				format!("skipped because the project could not be rendered: {error}"),
				None,
			);
			return;
		}
	};

	if let Some(error) = sorted_render_errors(&result, root).first() {
		add_doctor_check(
			checks,
			"consumer_sync",
			"Consumer Sync",
			DoctorStatus::Fail,
			format!(
				"{} render error(s); first: block `{}` at {}:{}:{}: {}",
				result.render_errors.len(),
				error.block_name,
				relative_display_path(&error.file, root),
				error.line,
				error.column,
				error.message
			),
			Some("fix the provider template, then run `mdt update`".to_string()),
		);
	} else if !result.stale.is_empty() || !result.stale_files.is_empty() {
		add_doctor_check(
			checks,
			"consumer_sync",
			"Consumer Sync",
			DoctorStatus::Warn,
			format!(
				"{} stale consumer block(s), {} stale formatted file(s)",
				result.stale.len(),
				result.stale_files.len()
			),
			Some("run `mdt update`, then `mdt check`".to_string()),
		);
	} else {
		add_doctor_check(
			checks,
			"consumer_sync",
			"Consumer Sync",
			DoctorStatus::Pass,
			"every linked consumer renders and is up to date".to_string(),
			None,
		);
	}
}

fn run_lsp() -> Result<(), Box<dyn std::error::Error>> {
	let rt = tokio::runtime::Runtime::new()?;
	rt.block_on(mdt_lsp::run_server());
	Ok(())
}

fn run_mcp(_args: &MdtCli) -> Result<(), Box<dyn std::error::Error>> {
	let rt = tokio::runtime::Runtime::new()?;
	rt.block_on(mdt_mcp::run_server());
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
			println!(
				"Assistant                 {}",
				assistant_display_name(assistant)
			);
			println!(
				"Strategy                  {}",
				payload["strategy"]["summary"].as_str().unwrap_or_default()
			);
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

/// Print warnings about undefined template variables.
fn print_template_warnings(warnings: &[TemplateWarning], root: &Path) {
	let mut sorted_warnings: Vec<_> = warnings.iter().collect();
	sorted_warnings.sort_by(|a, b| {
		relative_display_path(&a.provider_file, root)
			.cmp(&relative_display_path(&b.provider_file, root))
			.then_with(|| a.block_name.cmp(&b.block_name))
	});

	for warning in sorted_warnings {
		let rel = relative_display_path(&warning.provider_file, root);
		let mut undefined_vars = warning.undefined_variables.clone();
		undefined_vars.sort();
		let vars = undefined_vars.join(", ");
		eprintln!(
			"{} provider block `{}` in {rel} references undefined variable(s): {vars}",
			styled!(stderr, "warning:", yellow_bold),
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

/// Stable machine-readable code for a diagnostic kind.
fn diagnostic_code(kind: &DiagnosticKind) -> &'static str {
	match kind {
		DiagnosticKind::UnclosedBlock { .. } => "mdt::unclosed_block",
		DiagnosticKind::UnknownTransformer { .. } => "mdt::unknown_transformer",
		DiagnosticKind::InvalidTransformerArgs { .. } => "mdt::invalid_transformer_args",
		DiagnosticKind::UnusedProvider { .. } => "mdt::unused_provider",
		DiagnosticKind::UnmatchedClosingTag { .. } => "mdt::unmatched_closing_tag",
		DiagnosticKind::InvalidTag { .. } => "mdt::invalid_tag",
		DiagnosticKind::NestedBlock { .. } => "mdt::nested_block",
		DiagnosticKind::ProviderOutsideTemplate { .. } => "mdt::provider_outside_template",
		_ => "mdt::diagnostic",
	}
}

/// How to fix a diagnostic, shown as miette help text.
fn diagnostic_help(kind: &DiagnosticKind) -> String {
	match kind {
		DiagnosticKind::UnclosedBlock { name } => {
			format!(
				"add `<!-- {{/{name}}} -->` to close this block, or, if a nearby closing tag has \
				 a different name, fix the misspelled tag so the names match"
			)
		}
		DiagnosticKind::UnknownTransformer { .. } => {
			format!(
				"available transformers: {}",
				mdt_core::TransformerType::NAMES.join(", ")
			)
		}
		DiagnosticKind::InvalidTransformerArgs { .. } => {
			"check the transformer documentation for the correct number of arguments".to_string()
		}
		DiagnosticKind::UnusedProvider { name } => {
			format!(
				"add a consumer block `<!-- {{={name}}} -->...<!-- {{/{name}}} -->`, remove the \
				 unused provider, or pass `--ignore-unused-blocks`"
			)
		}
		DiagnosticKind::UnmatchedClosingTag { name } => {
			format!(
				"if an opening tag nearby is misspelled, rename one of the two tags so they \
				 match; otherwise remove `<!-- {{/{name}}} -->`"
			)
		}
		DiagnosticKind::InvalidTag { .. } => {
			"write the sigil directly after `{` (`{@name}`, `{=name}`, `{~name}`, `{/name}`) and \
			 use a name matching `[A-Za-z_][A-Za-z0-9_-]*`; pass `--ignore-invalid-names` to skip \
			 this check"
				.to_string()
		}
		DiagnosticKind::NestedBlock { outer, inner } => {
			format!(
				"move `{inner}` outside `{outer}`: everything between a consumer's tags is \
				 replaced by `mdt update`"
			)
		}
		DiagnosticKind::ProviderOutsideTemplate { name } => {
			format!(
				"move this block into a `*.t.md` template file, or reference an existing provider \
				 here with `<!-- {{={name}}} -->`"
			)
		}
		_ => String::new(),
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
		.with_code(diagnostic_code(&diag.kind))
		.with_severity(severity);
	let help = diagnostic_help(&diag.kind);
	if !help.is_empty() {
		diag_value = diag_value.with_help(help);
	}
	miette::Report::new(diag_value)
}

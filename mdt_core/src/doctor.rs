//! Project health checks shared by `mdt doctor` and the MCP `mdt_doctor` tool.
//!
//! [`doctor_report`] runs the same checks as `mdt doctor`: config discovery
//! and parsing, data sources, template layout, block structure (duplicate
//! providers, orphan consumers, unused providers, parser diagnostics),
//! consumer sync, and cache health. Rendering the report — styled text or
//! JSON — stays with the CLI; this module builds the data.

use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

use crate::BlockType;
use crate::MdtConfig;
use crate::check_project;
use crate::project::ConsumerEntry;
use crate::project::DiagnosticKind;
use crate::project::ProviderEntry;
use crate::project::ScanOptions;
use crate::project::ValidationOptions;
use crate::project::inspect_project_cache;
use crate::project::relative_display_path;
use crate::project::scan_project_with_config;
use crate::project::scan_project_with_options;
use crate::project::suggest_similar_provider_names;

/// Format a path for user-facing output.
///
/// Forward slashes are used on every platform so printed diagnostics and
/// JSON reports render identically across operating systems.
pub fn display_path(path: impl AsRef<Path>) -> String {
	path.as_ref().display().to_string().replace('\\', "/")
}

/// The outcome of a doctor check.
#[derive(Debug, Clone, Copy, serde::Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DoctorStatus {
	Pass,
	Warn,
	Fail,
	Skip,
}

impl DoctorStatus {
	/// The fixed-width uppercase tag shown in text output (`PASS`, `WARN`,
	/// `FAIL`, `SKIP`).
	pub fn tag(self) -> &'static str {
		match self {
			Self::Pass => "PASS",
			Self::Warn => "WARN",
			Self::Fail => "FAIL",
			Self::Skip => "SKIP",
		}
	}
}

/// One health check: what was inspected, how it ended, and how to fix it.
#[derive(Debug, serde::Serialize)]
pub struct DoctorCheck {
	pub id: &'static str,
	pub title: &'static str,
	pub status: DoctorStatus,
	pub message: String,
	pub hint: Option<String>,
}

/// How many checks ended in each status.
#[derive(Debug, Default, serde::Serialize)]
pub struct DoctorSummary {
	pub pass: usize,
	pub warn: usize,
	pub fail: usize,
	pub skip: usize,
}

/// The full `mdt doctor` outcome.
#[derive(Debug, serde::Serialize)]
pub struct DoctorReport {
	pub ok: bool,
	pub summary: DoctorSummary,
	pub checks: Vec<DoctorCheck>,
}

/// Append one check to `checks`.
pub fn add_doctor_check(
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

/// True when `path` starts with the canonical `.templates` directory.
pub fn is_canonical_template_dir(path: &Path) -> bool {
	path.components()
		.next()
		.is_some_and(|component| component.as_os_str() == ".templates")
}

/// `numerator / denominator` as a percentage with one decimal, or `n/a` when
/// `denominator` is zero.
pub fn ratio_percent_string(numerator: u64, denominator: u64) -> String {
	if denominator == 0 {
		return "n/a".to_string();
	}

	let ratio = (numerator as f64 / denominator as f64) * 100.0;
	format!("{ratio:.1}%")
}

/// The troubleshooting hint about `MDT_CACHE_VERIFY_HASH`, whatever its
/// current mode.
pub fn cache_hash_mode_hint(hash_verification_enabled: bool) -> String {
	if hash_verification_enabled {
		"unset `MDT_CACHE_VERIFY_HASH` to compare performance if cache reparses look too high"
			.to_string()
	} else {
		"set `MDT_CACHE_VERIFY_HASH=1` to validate cache keys with content hashes while \
		 troubleshooting"
			.to_string()
	}
}

/// Consumer blocks whose name matches no provider.
#[allow(clippy::implicit_hasher)]
pub fn count_orphan_consumers(
	providers: &HashMap<String, ProviderEntry>,
	consumers: &[ConsumerEntry],
) -> usize {
	consumers
		.iter()
		.filter(|consumer| consumer.block.r#type == BlockType::Consumer)
		.filter(|consumer| !providers.contains_key(&consumer.block.name))
		.count()
}

/// Providers no consumer block references.
#[allow(clippy::implicit_hasher)]
pub fn count_unused_providers(
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

/// Describe a consumer whose name matches no provider.
pub fn orphan_description(block_name: &str, location: &str, suggestions: &[String]) -> String {
	let hint = suggestions
		.first()
		.map_or_else(String::new, |name| format!(" (did you mean `{name}`?)"));
	format!("consumer `{block_name}` at {location} has no provider{hint}")
}

/// Render errors in file and position order, so output is stable.
pub fn sorted_render_errors<'a>(
	result: &'a crate::CheckResult,
	root: &Path,
) -> Vec<&'a crate::RenderError> {
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

/// How to fix a diagnostic, shown as miette help text.
pub fn diagnostic_help(kind: &DiagnosticKind) -> String {
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
				crate::TransformerType::NAMES.join(", ")
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
				"move `{inner}` outside `{outer}`: `mdt update` replaces everything between a \
				 consumer's tags, and a provider's tags would be copied into every consumer"
			)
		}
		DiagnosticKind::ProviderOutsideTemplate { name } => {
			format!(
				"move this block into a `*.t.md` template file, or reference an existing provider \
				 here with `<!-- {{={name}}} -->`"
			)
		}
	}
}

/// Run every `mdt doctor` check against `root` and build the report.
///
/// `options` mirrors the CLI's `--ignore-*` flags for the checks that classify
/// parser diagnostics. The report is data only; rendering it is up to the
/// caller.
pub fn doctor_report(root: &Path, options: &ValidationOptions) -> DoctorReport {
	let mut checks = Vec::new();

	let config_path = MdtConfig::resolve_path(root);

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

	let config = match MdtConfig::load(root) {
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
			match config.load_data(root) {
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
	match scan_project_with_options(root, &scan_options) {
		Ok(project) => add_block_checks(&mut checks, &project, root, options),
		Err(error) => {
			match error {
				crate::MdtError::DuplicateProvider {
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

	add_sync_check(&mut checks, root);

	let cache = inspect_project_cache(root, &scan_options);

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

	DoctorReport {
		ok: summary.fail == 0,
		summary,
		checks,
	}
}

/// Checks that only need the scanned blocks: provider uniqueness, orphan
/// consumers, unused providers, and parser diagnostics.
fn add_block_checks(
	checks: &mut Vec<DoctorCheck>,
	project: &crate::project::Project,
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
	let mut diagnostics: Vec<&crate::project::ProjectDiagnostic> = project
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

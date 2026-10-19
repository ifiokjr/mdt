//! Project inspection shared by `mdt info` and the MCP `mdt_info` tool.
//!
//! [`info_report`] gathers everything `mdt info` shows: the resolved project
//! root and config, block counts, data namespaces, template directories and
//! files, diagnostic totals, and cache state. Rendering the report — styled
//! text or JSON — stays with the CLI; this module builds the data.

use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use crate::MdtConfig;
use crate::MdtError;
use crate::doctor::count_orphan_consumers;
use crate::doctor::count_unused_providers;
use crate::doctor::display_path;
use crate::doctor::ratio_percent_string;
use crate::project::ScanOptions;
use crate::project::ValidationOptions;
use crate::project::inspect_project_cache;
use crate::project::relative_display_path;
use crate::project::scan_project_with_config;

/// The config facts `mdt info` summarizes: where it was found, its data
/// namespaces, and its configured template directories.
#[derive(Debug, Default)]
pub struct ConfigSummary {
	pub path: Option<PathBuf>,
	pub data_sources: Vec<DataSourceSummary>,
	pub template_dirs: Vec<PathBuf>,
}

/// One `[data]` namespace as `mdt info` describes it.
#[derive(Debug)]
pub struct DataSourceSummary {
	pub namespace: String,
	pub location: String,
	pub kind: String,
	pub format: String,
	pub explicit_format: bool,
}

/// A data source's effective format and whether it was set explicitly.
pub fn data_source_format(source: &crate::DataSource) -> (String, bool) {
	if let Some(explicit) = source
		.format()
		.map(str::trim)
		.filter(|value| !value.is_empty())
	{
		return (explicit.to_ascii_lowercase(), true);
	}

	let inferred = match source {
		crate::DataSource::Path(path) => {
			path.extension()
				.and_then(|ext| ext.to_str())
				.unwrap_or("unknown")
				.to_ascii_lowercase()
		}
		crate::DataSource::Typed(typed) => {
			typed
				.path
				.extension()
				.and_then(|ext| ext.to_str())
				.unwrap_or("unknown")
				.to_ascii_lowercase()
		}
		crate::DataSource::Script(_) => "text".to_string(),
	};

	(inferred, false)
}

/// A data source's display location and kind, such as a file path or a
/// `script:` command with its watch count.
pub fn data_source_summary_fields(source: &crate::DataSource) -> (String, String) {
	match source {
		crate::DataSource::Path(path) => (display_path(path), "file".to_string()),
		crate::DataSource::Typed(typed) => (display_path(&typed.path), "file".to_string()),
		crate::DataSource::Script(script) => {
			(
				format!("script: {}", script.command),
				if script.watch.is_empty() {
					"script".to_string()
				} else {
					format!("script (watch: {})", script.watch.len())
				},
			)
		}
	}
}

/// Load the config at `root` and summarize it for `mdt info`.
pub fn load_config_summary(root: &Path) -> Result<ConfigSummary, MdtError> {
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

/// `path` with forward slashes and a trailing slash, for display.
pub fn normalize_dir_hint(path: &Path) -> String {
	let mut hint = display_path(path);

	if !hint.ends_with('/') {
		hint.push('/');
	}

	hint
}

/// The configured template directories plus mdt's canonical ones, sorted and
/// deduplicated.
pub fn template_directory_hints(template_dirs: &[PathBuf]) -> Vec<String> {
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

#[derive(serde::Serialize)]
pub struct InfoProjectSection {
	pub root: String,
	pub resolved_config: String,
}

#[derive(serde::Serialize)]
pub struct InfoBlocksSection {
	pub providers: usize,
	pub consumers: usize,
	pub orphan_consumers: usize,
	pub unused_providers: usize,
}

#[derive(serde::Serialize)]
pub struct InfoDataSourceSection {
	pub namespace: String,
	pub location: String,
	pub kind: String,
	pub format: String,
	pub explicit_format: bool,
}

#[derive(serde::Serialize)]
pub struct InfoDataSection {
	pub namespace_count: usize,
	pub namespaces: Vec<InfoDataSourceSection>,
}

#[derive(serde::Serialize)]
pub struct InfoTemplatesSection {
	pub file_count: usize,
	pub configured_dirs: Vec<String>,
	pub canonical_hints: Vec<String>,
	pub discovered_files: Vec<String>,
}

#[derive(serde::Serialize)]
pub struct InfoDiagnosticsSection {
	pub total: usize,
	pub errors: usize,
	pub warnings: usize,
	pub missing_provider_count: usize,
	pub missing_provider_names: Vec<String>,
}

#[derive(serde::Serialize)]
pub struct InfoCacheLastScanSection {
	pub timestamp_unix_ms: u64,
	pub full_project_hit: bool,
	pub reused_files: u64,
	pub reparsed_files: u64,
	pub total_files: u64,
}

#[derive(serde::Serialize)]
pub struct InfoCacheArtifactStateSection {
	pub exists: bool,
	pub readable: bool,
	pub valid: bool,
}

#[derive(serde::Serialize)]
pub struct InfoCacheCompatibilityStateSection {
	pub schema_supported: bool,
	pub project_key_matches: bool,
	pub hash_verification_enabled: bool,
}

#[derive(serde::Serialize)]
pub struct InfoCacheSection {
	pub path: String,
	#[serde(flatten)]
	pub artifact: InfoCacheArtifactStateSection,
	pub schema_version: Option<u32>,
	#[serde(flatten)]
	pub compatibility: InfoCacheCompatibilityStateSection,
	pub scan_count: u64,
	pub full_project_hit_count: u64,
	pub full_project_hit_rate: String,
	pub reused_file_count_total: u64,
	pub reparsed_file_count_total: u64,
	pub file_reuse_rate: String,
	pub last_scan: Option<InfoCacheLastScanSection>,
}

/// Everything `mdt info` reports about a project, section by section.
#[derive(serde::Serialize)]
pub struct InfoReport {
	pub project: InfoProjectSection,
	pub blocks: InfoBlocksSection,
	pub data: InfoDataSection,
	pub templates: InfoTemplatesSection,
	pub diagnostics: InfoDiagnosticsSection,
	pub cache: InfoCacheSection,
}

/// Build the `mdt info` report for `root`.
///
/// `options` mirrors the CLI's `--ignore-*` flags for classifying
/// diagnostics as errors or warnings. Rendering the report is up to the
/// caller.
pub fn info_report(root: &Path, options: &ValidationOptions) -> Result<InfoReport, MdtError> {
	let config = load_config_summary(root)?;
	let loaded_config = MdtConfig::load(root)?;
	let scan_options = ScanOptions::from_config(loaded_config.as_ref());
	let ctx = scan_project_with_config(root)?;

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
		.map(|entry| relative_display_path(&entry.file, root))
		.collect::<BTreeSet<_>>()
		.into_iter()
		.collect();

	let diagnostics_total = ctx.project.diagnostics.len();
	let diagnostics_errors = ctx
		.project
		.diagnostics
		.iter()
		.filter(|diag| diag.is_error(options))
		.count();
	let diagnostics_warnings = diagnostics_total.saturating_sub(diagnostics_errors);

	let mut missing_providers = ctx.find_missing_providers();
	missing_providers.sort();

	let cache_inspection = inspect_project_cache(root, &scan_options);
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

	Ok(InfoReport {
		project: InfoProjectSection {
			root: display_path(root),
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
	})
}

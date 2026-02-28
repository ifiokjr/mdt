use std::collections::BTreeMap;
use std::fs::Metadata;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use serde::Deserialize;
use serde::Serialize;

use crate::project::ConsumerEntry;
use crate::project::Project;
use crate::project::ProjectDiagnostic;
use crate::project::ProviderEntry;

pub(crate) const CACHE_SCHEMA_VERSION: u32 = 2;
const CACHE_FILE_NAME: &str = "index-v2.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct FileFingerprint {
	pub size: u64,
	pub modified_unix_ms: u64,
	pub content_hash: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CachedFileData {
	pub providers: Vec<ProviderEntry>,
	pub consumers: Vec<ConsumerEntry>,
	pub diagnostics: Vec<ProjectDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub(crate) struct LastScanTelemetry {
	pub timestamp_unix_ms: u64,
	pub full_project_hit: bool,
	pub reused_files: u64,
	pub reparsed_files: u64,
	pub total_files: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub(crate) struct CacheTelemetry {
	pub scan_count: u64,
	pub full_project_hit_count: u64,
	pub reused_file_count_total: u64,
	pub reparsed_file_count_total: u64,
	pub last_scan: Option<LastScanTelemetry>,
}

impl CacheTelemetry {
	pub(crate) fn record_scan(
		&mut self,
		full_project_hit: bool,
		reused_files: usize,
		reparsed_files: usize,
		total_files: usize,
	) {
		let reused_files = u64::try_from(reused_files).unwrap_or(u64::MAX);
		let reparsed_files = u64::try_from(reparsed_files).unwrap_or(u64::MAX);
		let total_files = u64::try_from(total_files).unwrap_or(u64::MAX);

		self.scan_count = self.scan_count.saturating_add(1);
		if full_project_hit {
			self.full_project_hit_count = self.full_project_hit_count.saturating_add(1);
		}
		self.reused_file_count_total = self.reused_file_count_total.saturating_add(reused_files);
		self.reparsed_file_count_total = self
			.reparsed_file_count_total
			.saturating_add(reparsed_files);
		self.last_scan = Some(LastScanTelemetry {
			timestamp_unix_ms: now_unix_ms(),
			full_project_hit,
			reused_files,
			reparsed_files,
			total_files,
		});
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ProjectIndexCache {
	pub schema_version: u32,
	pub project_key: String,
	pub files: BTreeMap<String, FileFingerprint>,
	pub file_data: BTreeMap<String, CachedFileData>,
	#[serde(default)]
	pub telemetry: CacheTelemetry,
	pub project: Project,
}

impl ProjectIndexCache {
	pub(crate) fn new(
		project_key: String,
		files: BTreeMap<String, FileFingerprint>,
		file_data: BTreeMap<String, CachedFileData>,
		project: Project,
	) -> Self {
		Self {
			schema_version: CACHE_SCHEMA_VERSION,
			project_key,
			files,
			file_data,
			telemetry: CacheTelemetry::default(),
			project,
		}
	}
}

fn now_unix_ms() -> u64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map_or(0, |duration| {
			duration.as_millis().try_into().unwrap_or(u64::MAX)
		})
}

pub(crate) fn cache_path(root: &Path) -> PathBuf {
	root.join(".mdt").join("cache").join(CACHE_FILE_NAME)
}

pub(crate) fn relative_file_key(root: &Path, file: &Path) -> String {
	file.strip_prefix(root)
		.unwrap_or(file)
		.to_string_lossy()
		.replace('\\', "/")
}

pub(crate) fn build_file_fingerprint(
	metadata: &Metadata,
	content_hash: Option<u64>,
) -> FileFingerprint {
	let modified_unix_ms = metadata
		.modified()
		.ok()
		.and_then(|time| time.duration_since(UNIX_EPOCH).ok())
		.and_then(|duration| duration.as_millis().try_into().ok())
		.unwrap_or(0);

	FileFingerprint {
		size: metadata.len(),
		modified_unix_ms,
		content_hash,
	}
}

pub(crate) fn load(root: &Path, project_key: &str) -> Option<ProjectIndexCache> {
	let cache_path = cache_path(root);
	let bytes = std::fs::read(cache_path).ok()?;
	let cache: ProjectIndexCache = serde_json::from_slice(&bytes).ok()?;

	if cache.schema_version != CACHE_SCHEMA_VERSION {
		return None;
	}

	if cache.project_key != project_key {
		return None;
	}

	Some(cache)
}

pub(crate) fn save(root: &Path, cache: &ProjectIndexCache) {
	let cache_path = cache_path(root);
	let Some(cache_dir) = cache_path.parent() else {
		return;
	};

	if std::fs::create_dir_all(cache_dir).is_err() {
		return;
	}

	let Ok(payload) = serde_json::to_vec_pretty(cache) else {
		return;
	};

	let temp_path = cache_path.with_extension(format!(
		"json.tmp-{}-{}",
		std::process::id(),
		SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.map_or(0, |duration| duration.as_nanos())
	));

	if std::fs::write(&temp_path, payload).is_err() {
		return;
	}

	if std::fs::rename(&temp_path, &cache_path).is_err() {
		let _ = std::fs::remove_file(temp_path);
	}
}

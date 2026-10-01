//! Ranking of existing providers for `mdt_find_reuse`.

use mdt_core::project::levenshtein_distance;
use serde::Serialize;

/// How a provider matched a `mdt_find_reuse` query, best first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MatchKind {
	/// The provider has exactly the requested name.
	Exact,
	/// The names differ only in case or `-`/`_` separators, such as
	/// `install-guide` and `installGuide`.
	Normalized,
	/// The provider name starts with the requested name.
	Prefix,
	/// One name contains the other.
	Substring,
	/// The provider name is a few edits away from the requested name.
	Similar,
	/// The name does not match, but the provider body contains the
	/// requested content.
	Content,
}

/// A name match: its kind and the edit distance between the normalized
/// names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NameMatch {
	pub kind: MatchKind,
	pub distance: usize,
}

/// Match the provider `name` against the requested `query` name, or `None`
/// when they are unrelated.
pub(crate) fn match_name(query: &str, name: &str) -> Option<NameMatch> {
	if query == name {
		return Some(NameMatch {
			kind: MatchKind::Exact,
			distance: 0,
		});
	}

	let query = normalize_name(query);
	let name = normalize_name(name);

	if query.is_empty() {
		return None;
	}

	let distance = levenshtein_distance(&query, &name);
	let kind = if query == name {
		MatchKind::Normalized
	} else if name.starts_with(&query) {
		MatchKind::Prefix
	} else if name.contains(&query) || query.contains(&name) {
		MatchKind::Substring
	} else if distance <= max_similar_distance(&query) {
		MatchKind::Similar
	} else {
		return None;
	};

	Some(NameMatch { kind, distance })
}

/// Whether `content` contains `query`, ignoring case.
pub(crate) fn content_contains(content: &str, query: &str) -> bool {
	content.to_lowercase().contains(&query.to_lowercase())
}

/// Lowercase `name` and drop `-` and `_`, so kebab-, snake-, and camel-case
/// spellings of a name compare equal.
fn normalize_name(name: &str) -> String {
	name.chars()
		.filter(|character| !matches!(character, '-' | '_'))
		.flat_map(char::to_lowercase)
		.collect()
}

/// The edit-distance cutoff for [`MatchKind::Similar`], matching the orphan
/// suggestions `mdt check` makes.
fn max_similar_distance(query: &str) -> usize {
	(query.len() / 2).max(2)
}

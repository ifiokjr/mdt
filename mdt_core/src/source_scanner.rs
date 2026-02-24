use markdown::mdast::Html;
use markdown::unist::Point as UnistPoint;
use markdown::unist::Position as UnistPosition;

use crate::MdtResult;
use crate::config::CodeBlockFilter;
use crate::lexer::memstr;
use crate::lexer::tokenize;
use crate::parser::Block;
use crate::parser::ParseDiagnostic;
use crate::parser::build_blocks_from_groups_lenient;
use crate::parser::build_blocks_from_groups_with_diagnostics;

/// Parse source code content (non-markdown) for mdt blocks by extracting HTML
/// comments directly from the raw text.
pub fn parse_source(content: &str) -> MdtResult<Vec<Block>> {
	let html_nodes = extract_html_comments(content);
	let token_groups = tokenize(html_nodes)?;
	build_blocks_from_groups_lenient(&token_groups)
}

/// Parse source code content and return blocks together with diagnostics.
/// When `filter` is enabled, HTML comments inside fenced code blocks
/// (within doc comments) are excluded from scanning.
pub fn parse_source_with_diagnostics(
	content: &str,
	filter: &CodeBlockFilter,
) -> MdtResult<(Vec<Block>, Vec<ParseDiagnostic>)> {
	let mut html_nodes = extract_html_comments(content);

	if filter.is_enabled() {
		let code_block_ranges = find_fenced_code_block_ranges(content, filter);
		html_nodes.retain(|node| {
			let Some(pos) = &node.position else {
				return true;
			};
			let offset = pos.start.offset;
			!code_block_ranges
				.iter()
				.any(|range| offset >= range.start && offset < range.end)
		});
	}

	let token_groups = tokenize(html_nodes)?;
	build_blocks_from_groups_with_diagnostics(&token_groups)
}

/// Pre-computed table of line-start byte offsets for efficient offset-to-point
/// conversion. Instead of scanning the entire string for each offset (O(n*m)),
/// we build this table once (O(n)) and use binary search (O(log n)) per lookup.
struct LineTable {
	/// Byte offsets of the start of each line. `line_starts[0]` is always 0.
	line_starts: Vec<usize>,
}

impl LineTable {
	fn new(content: &str) -> Self {
		let mut line_starts = vec![0];
		for (i, byte) in content.bytes().enumerate() {
			if byte == b'\n' {
				line_starts.push(i + 1);
			}
		}
		Self { line_starts }
	}

	/// Convert a byte offset to a `markdown::unist::Point` (1-indexed
	/// line/column). Uses binary search over the pre-computed line table.
	fn offset_to_point(&self, offset: usize) -> UnistPoint {
		// Binary search for the line containing this offset.
		let line_idx = match self.line_starts.binary_search(&offset) {
			Ok(exact) => exact,
			Err(insert) => insert.saturating_sub(1),
		};
		let line = line_idx + 1; // 1-indexed
		let column = offset - self.line_starts[line_idx] + 1; // 1-indexed

		UnistPoint {
			line,
			column,
			offset,
		}
	}
}

/// Extract HTML comments (`<!-- ... -->`) from raw text content, returning
/// `Html` nodes with correct byte positions. This is used for source files
/// where the markdown AST parser won't find HTML comments inside code
/// comments.
pub fn extract_html_comments(content: &str) -> Vec<Html> {
	let bytes = content.as_bytes();
	let open_marker = b"<!--";
	let close_marker = b"-->";
	let mut nodes = Vec::new();
	let mut search_from = 0;
	let line_table = LineTable::new(content);

	while search_from < bytes.len() {
		let Some(open_offset) = memstr(&bytes[search_from..], open_marker) else {
			break;
		};
		let abs_open = search_from + open_offset;

		let after_open = abs_open + open_marker.len();
		if after_open >= bytes.len() {
			break;
		}

		let Some(close_offset) = memstr(&bytes[after_open..], close_marker) else {
			break;
		};
		let abs_close_end = after_open + close_offset + close_marker.len();

		let value = String::from_utf8_lossy(&bytes[abs_open..abs_close_end]).to_string();

		let start_point = line_table.offset_to_point(abs_open);
		let end_point = line_table.offset_to_point(abs_close_end);

		nodes.push(Html {
			value,
			position: Some(UnistPosition {
				start: start_point,
				end: end_point,
			}),
		});

		search_from = abs_close_end;
	}

	nodes
}

/// A byte range representing the content region of a fenced code block.
struct CodeBlockRange {
	start: usize,
	end: usize,
}

/// Common comment prefixes stripped when detecting fenced code blocks in source
/// files. Order matters — longer prefixes are checked first to avoid partial
/// matches.
const COMMENT_PREFIXES: &[&str] = &[
	"///!", "//!", "///", "//", "##", "#", "* ", "**", "*", ";", "--",
];

/// Strip leading whitespace and a single comment prefix from a line,
/// returning the remaining text after stripping.
fn strip_comment_prefix(line: &str) -> &str {
	let trimmed = line.trim_start();
	for prefix in COMMENT_PREFIXES {
		if let Some(rest) = trimmed.strip_prefix(prefix) {
			// Strip one optional space after the prefix.
			return rest.strip_prefix(' ').unwrap_or(rest);
		}
	}
	trimmed
}

/// Find byte ranges of fenced code block content in raw source text.
///
/// This detects fenced code blocks that appear inside doc comments (for
/// example, triple-backtick fences prefixed with `///`) and returns the byte
/// ranges of their content so that HTML comments within can be filtered out.
fn find_fenced_code_block_ranges(content: &str, filter: &CodeBlockFilter) -> Vec<CodeBlockRange> {
	let mut ranges = Vec::new();
	let mut in_code_block = false;
	let mut block_start = 0;
	let mut should_skip_current = false;
	let mut fence_char = '`';
	let mut fence_len = 0;

	let mut offset = 0;

	for line in content.split('\n') {
		let line_end = offset + line.len();
		let stripped = strip_comment_prefix(line);

		if in_code_block {
			// Check for closing fence: same char, at least same length, no
			// info string.
			let closing_fence_len = stripped.chars().take_while(|&c| c == fence_char).count();
			let after_fence = &stripped[closing_fence_len..];
			if closing_fence_len >= fence_len && after_fence.trim().is_empty() {
				if should_skip_current {
					ranges.push(CodeBlockRange {
						start: block_start,
						end: line_end,
					});
				}
				in_code_block = false;
			}
		} else {
			// Check for opening fence: 3+ backticks or tildes.
			let backtick_len = stripped.chars().take_while(|&c| c == '`').count();
			let tilde_len = stripped.chars().take_while(|&c| c == '~').count();

			let (fc, fl) = if backtick_len >= 3 {
				('`', backtick_len)
			} else if tilde_len >= 3 {
				('~', tilde_len)
			} else {
				offset = line_end + 1; // +1 for the \n
				continue;
			};

			let info_string = stripped[fl..].trim();
			fence_char = fc;
			fence_len = fl;
			in_code_block = true;
			block_start = offset;
			should_skip_current = filter.should_skip(info_string);
		}

		offset = line_end + 1; // +1 for the \n
	}

	ranges
}

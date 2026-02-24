use markdown::mdast::Html;
use markdown::unist::Point as UnistPoint;
use markdown::unist::Position as UnistPosition;

use crate::MdtResult;
use crate::lexer::memstr;
use crate::lexer::tokenize;
use crate::parser::Block;
use crate::parser::build_blocks_from_groups_lenient;

/// Parse source code content (non-markdown) for mdt blocks by extracting HTML
/// comments directly from the raw text.
pub fn parse_source(content: &str) -> MdtResult<Vec<Block>> {
	let html_nodes = extract_html_comments(content);
	let token_groups = tokenize(html_nodes)?;
	build_blocks_from_groups_lenient(&token_groups)
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

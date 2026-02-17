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

		let start_point = offset_to_point(content, abs_open);
		let end_point = offset_to_point(content, abs_close_end);

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

/// Convert a byte offset to a `markdown::unist::Point` (1-indexed line/column).
fn offset_to_point(content: &str, offset: usize) -> UnistPoint {
	let mut line: usize = 1;
	let mut column: usize = 1;

	for (i, ch) in content.bytes().enumerate() {
		if i == offset {
			break;
		}
		if ch == b'\n' {
			line += 1;
			column = 1;
		} else {
			column += 1;
		}
	}

	UnistPoint {
		line,
		column,
		offset,
	}
}

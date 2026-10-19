//! Internal markdown table normalization.
//!
//! mdt never reformats provider text, with one exception: when
//! `[format] tables` is enabled, [`normalize_markdown_tables`] aligns GFM
//! table columns the same way dprint's markdown plugin does, so template
//! files and formatter-owned output files converge byte-for-byte without
//! excluding either side from a formatter.
//!
//! The rules mirror `dprint-plugin-markdown` 0.22.x, verified by probing
//! `dprint fmt --stdin` and by reading its `gen_table` implementation:
//!
//! - Column width is the maximum display width of every cell in the column
//!   (CJK characters count as two), or `colons + 1` for aligned columns so
//!   the delimiter keeps at least one dash.
//! - Cells are trimmed and padded: left and unaligned columns pad on the
//!   right, right-aligned columns pad on the left, and centered columns
//!   split the padding with the extra space on the right.
//! - The delimiter row is regenerated: alignment colons plus dashes filling
//!   the column width exactly — long delimiter rows shrink, short ones grow.
//! - Rows with fewer cells than the header are padded with empty cells.
//!
//! Deliberate deviations from dprint, all in the conservative direction:
//!
//! - Tables inside fenced code blocks are never touched. dprint formats
//!   ```` ```markdown ```` fences recursively; mdt leaves every fence alone
//!   so code samples keep their bytes.
//! - A body row with more cells than the header leaves the whole table
//!   unchanged. dprint truncates the row; mdt does not destroy content.
//! - A line without a pipe ends the table instead of becoming a row, so
//!   prose that follows a table without a blank line is never swallowed.
//! - A table glued to preceding paragraph text is left alone; mdt inserts
//!   no blank lines and does not move content between blocks.

use unicode_width::UnicodeWidthStr;

/// Alignment of one table column, taken from its delimiter-row cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColumnAlignment {
	/// `---` — cells pad on the right.
	None,
	/// `:--` — cells pad on the right.
	Left,
	/// `:-:` — cells center, extra space on the right.
	Center,
	/// `--:` — cells pad on the left.
	Right,
}

impl ColumnAlignment {
	/// Number of colon characters the delimiter cell renders with.
	fn colon_count(self) -> usize {
		match self {
			Self::None => 0,
			Self::Left | Self::Right => 1,
			Self::Center => 2,
		}
	}
}

/// A parsed GFM table block waiting to be rendered.
struct TableBlock {
	/// Leading whitespace shared by every row (list indentation).
	indent: String,
	/// Trimmed header cells.
	header: Vec<String>,
	/// Per-column alignment from the delimiter row.
	alignments: Vec<ColumnAlignment>,
	/// Trimmed body cells; short rows are padded at render time.
	rows: Vec<Vec<String>>,
}

/// Apply the enabled `[format]` rules to block content.
///
/// Table alignment runs first so the trailing-whitespace rule sees the
/// re-rendered delimiter row. Both rules skip fenced code blocks: internal
/// formatting never rewrites code.
pub(crate) fn normalize(content: &str, rules: &crate::config::ResolvedFormatRules) -> String {
	if !rules.tables && !rules.trailing_whitespace {
		return content.to_string();
	}

	let content = if rules.tables {
		normalize_markdown_tables(content)
	} else {
		content.to_string()
	};

	if rules.trailing_whitespace {
		trim_trailing_whitespace(&content)
	} else {
		content
	}
}

/// Trim trailing whitespace from every line, leaving fenced code blocks
/// untouched.
///
/// Idempotent, like every `[format]` rule: trimming already-trimmed content
/// returns it unchanged.
pub(crate) fn trim_trailing_whitespace(content: &str) -> String {
	if !content
		.bytes()
		.any(|b| b == b' ' || b == b'\t' || b == b'\r')
	{
		return content.to_string();
	}

	let mut fence: Option<(char, usize)> = None;

	let lines: Vec<String> = content
		.split('\n')
		.map(|line| {
			if let Some((fence_char, fence_len)) = fence {
				if is_closing_fence(line, fence_char, fence_len) {
					fence = None;
				}

				return line.to_string();
			}

			if let Some((fence_char, fence_len)) = opening_fence(line) {
				fence = Some((fence_char, fence_len));

				return line.to_string();
			}

			line.trim_end().to_string()
		})
		.collect();

	lines.join("\n")
}

/// Align every GFM table in `content`.
///
/// Idempotent: normalizing already-normalized content returns it unchanged.
/// Content is expected to use `\n` line endings; every caller normalizes
/// before splicing and restores afterwards.
pub(crate) fn normalize_markdown_tables(content: &str) -> String {
	if !content.contains('|') {
		return content.to_string();
	}

	let lines: Vec<&str> = content.split('\n').collect();
	let mut output: Vec<String> = Vec::with_capacity(lines.len());
	// Open fence: (fence character, run length). Lines inside a fence pass
	// through untouched.
	let mut fence: Option<(char, usize)> = None;
	let mut index = 0;

	while index < lines.len() {
		let line = lines[index];

		if let Some((fence_char, fence_len)) = fence {
			output.push(line.to_string());
			if is_closing_fence(line, fence_char, fence_len) {
				fence = None;
			}
			index += 1;
			continue;
		}

		if let Some((fence_char, fence_len)) = opening_fence(line) {
			output.push(line.to_string());
			fence = Some((fence_char, fence_len));
			index += 1;
			continue;
		}

		if let Some((table, consumed)) = try_parse_table(&lines, index) {
			output.extend(render_table(&table));
			index += consumed;
		} else {
			output.push(line.to_string());
			index += 1;
		}
	}

	output.join("\n")
}

/// Parse the table starting at `lines[start]`, if any.
///
/// Returns the parsed block and the number of lines it spans, or `None` when
/// the lines do not form a table this module normalizes.
fn try_parse_table(lines: &[&str], start: usize) -> Option<(TableBlock, usize)> {
	let header_line = lines[start];
	let trimmed_header = header_line.trim();

	if trimmed_header.is_empty() || !has_unescaped_pipe(trimmed_header) {
		return None;
	}

	// A table cannot interrupt a paragraph; only detect one after a blank
	// line, a heading, an HTML comment, a closing fence, or at the start of
	// the content.
	if let Some(previous) = start.checked_sub(1).map(|index| lines[index]) {
		let previous = previous.trim();
		let block_boundary = previous.is_empty()
			|| previous.starts_with('#')
			|| previous.starts_with("<!--")
			|| opening_fence(previous).is_some();
		if !block_boundary {
			return None;
		}
	}

	let delimiter_line = lines.get(start + 1)?;
	let trimmed_delimiter = delimiter_line.trim();

	if trimmed_delimiter.is_empty() || !has_unescaped_pipe(trimmed_delimiter) {
		return None;
	}

	let header = split_row_cells(header_line);
	let delimiter_cells = split_row_cells(delimiter_line);

	if header.is_empty() || header.len() != delimiter_cells.len() {
		return None;
	}

	let alignments = delimiter_cells
		.iter()
		.map(|cell| parse_delimiter_cell(cell))
		.collect::<Option<Vec<_>>>()?;

	let indent = header_line
		.chars()
		.take_while(|character| character.is_whitespace())
		.collect::<String>();

	let mut rows = Vec::new();
	let mut end = start + 2;

	while let Some(line) = lines.get(end) {
		let trimmed = line.trim();
		if trimmed.is_empty() || !has_unescaped_pipe(trimmed) {
			break;
		}
		rows.push(split_row_cells(line));
		end += 1;
	}

	// Never truncate a row: leave tables with overflowing rows alone.
	if rows.iter().any(|row| row.len() > header.len()) {
		return None;
	}

	Some((
		TableBlock {
			indent,
			header,
			alignments,
			rows,
		},
		end - start,
	))
}

/// Render a table as aligned lines.
fn render_table(table: &TableBlock) -> Vec<String> {
	let widths = column_widths(table);
	let mut lines = Vec::with_capacity(2 + table.rows.len());

	lines.push(render_row(
		&table.indent,
		&table.header,
		&widths,
		&table.alignments,
	));
	lines.push(render_delimiter_row(
		&table.indent,
		&widths,
		&table.alignments,
	));

	for row in &table.rows {
		let padded = pad_row(row, table.header.len());
		lines.push(render_row(
			&table.indent,
			&padded,
			&widths,
			&table.alignments,
		));
	}

	lines
}

/// Per-column widths: the widest cell, the alignment's colon requirement,
/// and at least one column so the delimiter keeps a dash.
fn column_widths(table: &TableBlock) -> Vec<usize> {
	(0..table.header.len())
		.map(|column| {
			let mut width = display_width(&table.header[column]);

			for row in &table.rows {
				if let Some(cell) = row.get(column) {
					width = width.max(display_width(cell));
				}
			}

			if table.alignments[column] != ColumnAlignment::None {
				width = width.max(table.alignments[column].colon_count() + 1);
			}

			width.max(1)
		})
		.collect()
}

fn render_row(
	indent: &str,
	cells: &[String],
	widths: &[usize],
	alignments: &[ColumnAlignment],
) -> String {
	let mut line = String::with_capacity(indent.len() + cells.len() * 8);
	line.push_str(indent);
	line.push_str("| ");

	for (column, cell) in cells.iter().enumerate() {
		if column > 0 {
			line.push_str(" | ");
		}

		line.push_str(&pad_cell(
			cell,
			widths[column],
			alignments
				.get(column)
				.copied()
				.unwrap_or(ColumnAlignment::None),
		));
	}

	line.push_str(" |");
	line
}

fn render_delimiter_row(indent: &str, widths: &[usize], alignments: &[ColumnAlignment]) -> String {
	let mut line = String::with_capacity(indent.len() + widths.len() * 8);
	line.push_str(indent);
	line.push_str("| ");

	for (column, width) in widths.iter().enumerate() {
		if column > 0 {
			line.push_str(" | ");
		}

		let alignment = alignments
			.get(column)
			.copied()
			.unwrap_or(ColumnAlignment::None);
		let dashes = width.saturating_sub(alignment.colon_count());

		if matches!(alignment, ColumnAlignment::Left | ColumnAlignment::Center) {
			line.push(':');
		}
		line.push_str(&"-".repeat(dashes));
		if matches!(alignment, ColumnAlignment::Right | ColumnAlignment::Center) {
			line.push(':');
		}
	}

	line.push_str(" |");
	line
}

/// Pad `cell` to `width` according to the column alignment.
fn pad_cell(cell: &str, width: usize, alignment: ColumnAlignment) -> String {
	let difference = width.saturating_sub(display_width(cell));

	match alignment {
		ColumnAlignment::None | ColumnAlignment::Left => {
			format!("{cell}{}", " ".repeat(difference))
		}
		ColumnAlignment::Right => format!("{}{cell}", " ".repeat(difference)),
		// The extra space goes on the right, matching dprint's floor/ceil
		// split.
		ColumnAlignment::Center => {
			let leading = difference / 2;
			format!(
				"{}{cell}{}",
				" ".repeat(leading),
				" ".repeat(difference - leading)
			)
		}
	}
}

/// Extend `row` with empty cells up to the header's column count.
fn pad_row(row: &[String], columns: usize) -> Vec<String> {
	let mut padded = row.to_vec();
	padded.resize(columns, String::new());
	padded
}

fn display_width(cell: &str) -> usize {
	cell.width()
}

/// Split a table row into cells on unescaped pipes.
///
/// A single leading or trailing pipe is dropped: `| a | b |` and `a | b`
/// both split to `["a", "b"]`. `\|` is a literal pipe and `\\` a literal
/// backslash, so `\\|` still separates cells. Cells are trimmed.
fn split_row_cells(line: &str) -> Vec<String> {
	let trimmed = line.trim();
	let mut cells = Vec::new();
	let mut current = String::new();
	let mut chars = trimmed.chars().peekable();

	if chars.peek() == Some(&'|') {
		chars.next();
	}

	let mut ends_with_separator = false;

	while let Some(character) = chars.next() {
		match character {
			'\\' => {
				current.push('\\');
				if let Some(&escaped) = chars.peek() {
					current.push(escaped);
					chars.next();
				}
			}
			'|' => {
				ends_with_separator = chars.peek().is_none();
				cells.push(std::mem::take(&mut current));
			}
			_ => current.push(character),
		}
	}

	cells.push(current);

	if ends_with_separator {
		cells.pop();
	}

	cells
		.into_iter()
		.map(|cell| cell.trim().to_string())
		.collect()
}

/// Parse a delimiter-row cell such as `---`, `:--`, `--:`, or `:-:`.
///
/// Returns `None` when the cell is not delimiter-shaped, which means the
/// line is not a table delimiter row.
fn parse_delimiter_cell(cell: &str) -> Option<ColumnAlignment> {
	let left = cell.starts_with(':');
	let right = cell.ends_with(':') && cell.len() > usize::from(left);

	let dashes = &cell[usize::from(left)..cell.len() - usize::from(right)];

	if dashes.is_empty() || !dashes.chars().all(|character| character == '-') {
		return None;
	}

	Some(match (left, right) {
		(true, true) => ColumnAlignment::Center,
		(true, false) => ColumnAlignment::Left,
		(false, true) => ColumnAlignment::Right,
		(false, false) => ColumnAlignment::None,
	})
}

/// Whether the line contains a pipe that is not backslash-escaped.
fn has_unescaped_pipe(line: &str) -> bool {
	let mut chars = line.chars();

	while let Some(character) = chars.next() {
		match character {
			'\\' => {
				chars.next();
			}
			'|' => return true,
			_ => {}
		}
	}

	false
}

/// The fence a line opens, if it starts a fenced code block.
fn opening_fence(line: &str) -> Option<(char, usize)> {
	let trimmed = line.trim_start();

	for fence_char in ['`', '~'] {
		let count = trimmed.chars().take_while(|&c| c == fence_char).count();

		if count >= 3 {
			return Some((fence_char, count));
		}
	}

	None
}

/// Whether the line closes the given open fence.
fn is_closing_fence(line: &str, fence_char: char, fence_len: usize) -> bool {
	let trimmed = line.trim();
	let count = trimmed.chars().take_while(|&c| c == fence_char).count();

	count >= fence_len && trimmed.chars().all(|c| c == fence_char)
}

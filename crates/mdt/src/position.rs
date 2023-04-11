use std::fmt::Debug;
use std::fmt::Display;

use markdown::unist::Point as UnistPoint;
use markdown::unist::Position as UnistPosition;

/// One place in a source file. This is taken from the [unist] crate with the
/// `Copy` trait added.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Point {
  /// 1-indexed integer representing a line in a source file.
  pub line: usize,
  /// 1-indexed integer representing a column in a source file.
  pub column: usize,
  /// 0-indexed integer representing a character in a source file.
  pub offset: usize,
}

impl Point {
  #[must_use]
  pub fn new(line: usize, column: usize, offset: usize) -> Point {
    Self {
      line,
      column,
      offset,
    }
  }

  pub fn advance(&mut self, text: impl Display) {
    for char in text.to_string().chars() {
      if char == '\n' {
        self.line += 1;
        self.column = 0;
        self.offset += 1;
      } else {
        self.column += 1;
        self.offset += 1;
      }
    }
  }
}

impl From<UnistPoint> for Point {
  fn from(point: UnistPoint) -> Self {
    Self {
      line: point.line,
      column: point.column,
      offset: point.offset,
    }
  }
}

impl Debug for Point {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}:{} ({})", self.line, self.column, self.offset)
  }
}

/// Location of a node in a source file. This is taken from the `unist` crate
/// with the `Copy` trait added.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Position {
  /// Represents the place of the first character of the parsed source region.
  pub start: Point,
  /// Represents the place of the first character after the parsed source
  /// region, whether it exists or not.
  pub end: Point,
}

impl Position {
  #[must_use]
  pub fn new(
    start_line: usize,
    start_column: usize,
    start_offset: usize,
    end_line: usize,
    end_column: usize,
    end_offset: usize,
  ) -> Position {
    Self {
      start: Point::new(start_line, start_column, start_offset),
      end: Point::new(end_line, end_column, end_offset),
    }
  }

  pub fn from_point(point: Point) -> Self {
    Self {
      start: point,
      end: point,
    }
  }

  pub fn from_points(start: Point, end: Point) -> Self {
    Self { start, end }
  }

  pub fn advance_start(&mut self, text: impl Display) {
    self.start.advance(text);
  }

  pub fn advance_end(&mut self, text: impl Display) {
    self.end.advance(text);
  }
}

impl From<UnistPosition> for Position {
  fn from(position: UnistPosition) -> Self {
    Self {
      start: Point::from(position.start),
      end: Point::from(position.end),
    }
  }
}

impl Debug for Position {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{}:{}-{}:{} ({}-{})",
      self.start.line,
      self.start.column,
      self.end.line,
      self.end.column,
      self.start.offset,
      self.end.offset
    )
  }
}

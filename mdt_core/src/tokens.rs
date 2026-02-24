use std::fmt::Display;
use std::ops::Bound;
use std::ops::Range;
use std::ops::RangeBounds;
use std::ops::RangeFrom;
use std::ops::RangeInclusive;
use std::ops::RangeTo;
use std::ops::RangeToInclusive;

use derive_more::Deref;
use derive_more::DerefMut;
use float_cmp::approx_eq;

use crate::Position;

/// Only tokenize the blocks, not the content inside them or anything else.
#[derive(Debug, Clone)]
pub enum Token {
	/// `\n`
	Newline,
	/// `<!--`
	HtmlCommentOpen,
	/// `-->`
	HtmlCommentClose,
	/// `{=`
	ConsumerTag,
	/// `{@`
	ProviderTag,
	/// `{/`
	CloseTag,
	/// `}`
	BraceClose,
	/// `|`
	Pipe,
	/// `:`
	ArgumentDelimiter,
	/// ` ` | `\t` | `\r`
	Whitespace(u8),
	/// String content passed into a filter function e.g. `"my content"`
	String(String, u8),
	/// An identifier, e.g. `exampleName`
	Ident(String),
	/// An integer number, e.g. `123`
	Int(i64),
	/// A floating point number, e.g. `123.456`
	Float(f64),
}

impl Eq for Token {}
impl PartialEq for Token {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Token::Newline, Token::Newline)
			| (Token::HtmlCommentOpen, Token::HtmlCommentOpen)
			| (Token::HtmlCommentClose, Token::HtmlCommentClose)
			| (Token::ConsumerTag, Token::ConsumerTag)
			| (Token::ProviderTag, Token::ProviderTag)
			| (Token::CloseTag, Token::CloseTag)
			| (Token::BraceClose, Token::BraceClose)
			| (Token::Pipe, Token::Pipe)
			| (Token::ArgumentDelimiter, Token::ArgumentDelimiter) => true,
			(Token::Whitespace(byte), Token::Whitespace(other_byte)) => byte == other_byte,
			(Token::String(value, delimiter), Token::String(other_value, other_delimiter)) => {
				value == other_value && delimiter == other_delimiter
			}
			(Token::Ident(value), Token::Ident(other_value)) => value == other_value,
			(Token::Int(value), Token::Int(other_value)) => value == other_value,
			(Token::Float(value), Token::Float(other_value)) => {
				approx_eq!(f64, *value, *other_value, ulps = 2)
			}
			_ => false,
		}
	}
}

impl Token {
	pub fn increment(&self) -> usize {
		match self {
			Token::HtmlCommentOpen => 4,
			Token::HtmlCommentClose => 3,
			Token::ProviderTag | Token::ConsumerTag | Token::CloseTag => 2,
			Token::Newline
			| Token::BraceClose
			| Token::Pipe
			| Token::ArgumentDelimiter
			| Token::Whitespace(_) => 1,
			Token::String(string, _) => string.len() + 2,
			Token::Ident(ident) => ident.len(),
			Token::Int(number) => number.to_string().len(),
			Token::Float(number) => number.to_string().len(),
		}
	}

	pub fn same_type(&self, other: &Token) -> bool {
		match (self, other) {
			(Token::String(..), Token::String(..))
			| (Token::Int(_), Token::Int(_))
			| (Token::Float(_), Token::Float(_)) => true,
			// Ident's can be a wildcard or specific name like `true` or `false`
			(Token::Ident(value), Token::Ident(other_value)) => {
				value == "*" || other_value == "*" || value == other_value
			}
			(Token::Whitespace(byte), Token::Whitespace(other_byte)) => {
				byte == &b'*' || other_byte == &b'*' || byte == other_byte
			}
			_ => self == other,
		}
	}
}

impl Display for Token {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Token::Newline => writeln!(f),
			Token::Whitespace(byte) => write!(f, "{}", *byte as char),
			Token::HtmlCommentOpen => write!(f, "<!--"),
			Token::HtmlCommentClose => write!(f, "-->"),
			Token::ConsumerTag => write!(f, "{{="),
			Token::ProviderTag => write!(f, "{{@"),
			Token::CloseTag => write!(f, "{{/"),
			Token::BraceClose => write!(f, "}}"),
			Token::Pipe => write!(f, "|"),
			Token::ArgumentDelimiter => write!(f, ":"),
			Token::String(string, ch) => {
				let ch = *ch as char;
				write!(f, "{ch}{string}{ch}")
			}
			Token::Ident(ident) => write!(f, "{ident}"),
			Token::Int(number) => write!(f, "{number}"),
			Token::Float(number) => write!(f, "{number}"),
		}
	}
}

/// A group of tokens extracted from a single HTML comment node.
///
/// Each HTML comment in the source (e.g., `\<!-- {=name|trim} --\>`) is
/// tokenized into a `TokenGroup` containing its individual tokens and the
/// position of the comment in the original file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenGroup {
	/// The sequence of tokens parsed from the HTML comment.
	pub tokens: Vec<Token>,
	/// The position of the HTML comment in the source file.
	pub position: Position,
}

impl TokenGroup {
	/// Get the position of a range from the token group. If the index is out of
	/// bounds, it will be limited to the max length of `tokens`.
	pub fn position_of_range(&self, range: &impl GetDynamicRange) -> Position {
		let range = range.get_dynamic_range();
		let max = self.tokens.len();
		let start = range.start().unwrap_or(0).clamp(0, max - 1);
		let end = range.end().unwrap_or(max).clamp(0, max);

		let mut position = self.position;

		if let Some(tokens) = self.tokens.get(0..start) {
			for token in tokens {
				position.advance_start(token);
			}
		}

		position.end = position.start;

		if let Some(tokens) = self.tokens.get(start..end) {
			for token in tokens {
				position.advance_end(token);
			}
		}

		position
	}
}

/// A wrapper around a `RangeBounds<usize>` that provides uniform access to
/// start and end bounds, normalizing inclusive/exclusive/unbounded variants.
#[derive(Deref, DerefMut)]
pub struct DynamicRange<B>(
	#[deref]
	#[deref_mut]
	B,
)
where
	B: RangeBounds<usize>;

impl<B> From<B> for DynamicRange<B>
where
	B: RangeBounds<usize>,
{
	fn from(range: B) -> Self {
		Self(range)
	}
}

impl<B> DynamicRange<B>
where
	B: RangeBounds<usize>,
{
	pub fn start(&self) -> Option<usize> {
		match self.0.start_bound() {
			Bound::Included(value) | Bound::Excluded(value) => Some(*value),
			Bound::Unbounded => None,
		}
	}

	pub fn end(&self) -> Option<usize> {
		match self.0.end_bound() {
			Bound::Included(value) => Some(*value + 1),
			Bound::Excluded(value) => Some(*value),
			Bound::Unbounded => None,
		}
	}
}

/// Trait for converting various range types into a [`DynamicRange`].
pub trait GetDynamicRange {
	type Range: RangeBounds<usize>;
	fn get_dynamic_range(&self) -> DynamicRange<Self::Range>;
}

impl GetDynamicRange for usize {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((*self)..=(*self))
	}
}

impl GetDynamicRange for u128 {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((*self as usize)..=(*self as usize))
	}
}

impl GetDynamicRange for u64 {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((*self as usize)..=(*self as usize))
	}
}

impl GetDynamicRange for u32 {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((*self as usize)..=(*self as usize))
	}
}

impl GetDynamicRange for u16 {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((*self as usize)..=(*self as usize))
	}
}

impl GetDynamicRange for u8 {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((*self as usize)..=(*self as usize))
	}
}

impl GetDynamicRange for isize {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((*self as usize)..=(*self as usize))
	}
}

impl GetDynamicRange for i128 {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((*self as usize)..=(*self as usize))
	}
}

impl GetDynamicRange for i64 {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((*self as usize)..=(*self as usize))
	}
}

impl GetDynamicRange for i32 {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((*self as usize)..=(*self as usize))
	}
}

impl GetDynamicRange for i16 {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((*self as usize)..=(*self as usize))
	}
}

impl GetDynamicRange for i8 {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((*self as usize)..=(*self as usize))
	}
}

impl GetDynamicRange for &usize {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((**self)..=(**self))
	}
}

impl GetDynamicRange for &u128 {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((**self as usize)..=(**self as usize))
	}
}

impl GetDynamicRange for &u64 {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((**self as usize)..=(**self as usize))
	}
}

impl GetDynamicRange for &u32 {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((**self as usize)..=(**self as usize))
	}
}

impl GetDynamicRange for &u16 {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((**self as usize)..=(**self as usize))
	}
}

impl GetDynamicRange for &u8 {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((**self as usize)..=(**self as usize))
	}
}

impl GetDynamicRange for &isize {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((**self as usize)..=(**self as usize))
	}
}

impl GetDynamicRange for &i128 {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((**self as usize)..=(**self as usize))
	}
}

impl GetDynamicRange for &i64 {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((**self as usize)..=(**self as usize))
	}
}

impl GetDynamicRange for &i32 {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((**self as usize)..=(**self as usize))
	}
}

impl GetDynamicRange for &i16 {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((**self as usize)..=(**self as usize))
	}
}

impl GetDynamicRange for &i8 {
	type Range = RangeInclusive<usize>;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from((**self as usize)..=(**self as usize))
	}
}

impl GetDynamicRange for (Bound<usize>, Bound<usize>) {
	type Range = Self;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from(*self)
	}
}

impl GetDynamicRange for Range<&usize> {
	type Range = Self;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from(self.clone())
	}
}

impl GetDynamicRange for Range<usize> {
	type Range = Self;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from(self.clone())
	}
}

impl GetDynamicRange for RangeFrom<&usize> {
	type Range = Self;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from(self.clone())
	}
}

impl GetDynamicRange for RangeFrom<usize> {
	type Range = Self;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from(self.clone())
	}
}

impl GetDynamicRange for RangeInclusive<&usize> {
	type Range = Self;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from(self.clone())
	}
}

impl GetDynamicRange for RangeInclusive<usize> {
	type Range = Self;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from(self.clone())
	}
}

impl GetDynamicRange for RangeTo<&usize> {
	type Range = Self;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from(*self)
	}
}

impl GetDynamicRange for RangeTo<usize> {
	type Range = Self;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from(*self)
	}
}

impl GetDynamicRange for RangeToInclusive<&usize> {
	type Range = Self;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from(*self)
	}
}

impl GetDynamicRange for RangeToInclusive<usize> {
	type Range = Self;

	fn get_dynamic_range(&self) -> DynamicRange<Self::Range> {
		DynamicRange::from(*self)
	}
}

use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
pub enum MdtError {
	#[error(transparent)]
	#[diagnostic(code(mdt::io_error))]
	Io(#[from] std::io::Error),

	#[error("failure to load markdown: {0}")]
	#[diagnostic(code(mdt::io_error))]
	Markdown(String),
	#[diagnostic(code(mdt::missing_closing_tag))]
	#[error("missing closing tag for block: {0}")]
	MissingClosingTag(String),
	#[error("invalid token sequence")]
	#[diagnostic(code(mdt::invalid_token_sequence))]
	InvalidTokenSequence(usize),
	#[error("no provider found for consumer block: {0}")]
	#[diagnostic(code(mdt::missing_provider))]
	MissingProvider(String),
	#[error("consumer block `{name}` in {file} is out of date")]
	#[diagnostic(code(mdt::stale_consumer))]
	StaleConsumer { name: String, file: String },
}

pub type MdtResult<T> = Result<T, MdtError>;
pub type AnyError = Box<dyn std::error::Error>;
pub type AnyEmptyResult = Result<(), AnyError>;
pub type AnyResult<T> = Result<T, AnyError>;

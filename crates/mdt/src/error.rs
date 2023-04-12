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
}

pub type MdtResult<T> = std::result::Result<T, MdtError>;

use thiserror::Error as ThisError;

#[derive(Debug, ThisError, Clone)]
pub enum Error {
  #[error("failure to load markdown: {0}")]
  MarkdownError(String),
}

pub type Result<T> = std::result::Result<T, Error>;

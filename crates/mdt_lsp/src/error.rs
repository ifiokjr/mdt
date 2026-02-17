use thiserror::Error as ThisError;

#[derive(Debug, ThisError, Clone)]
pub enum Error {
	#[error("failed to parse document: {0}")]
	Parse(String),
	#[error("failed to scan project: {0}")]
	ProjectScan(String),
}

pub type Result<T> = std::result::Result<T, Error>;

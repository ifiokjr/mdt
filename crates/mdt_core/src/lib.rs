//! `mdt` is a data-driven template engine for keeping documentation
//! synchronized across your project. It uses comment-based template tags to
//! define content once and distribute it to multiple locations — markdown
//! files, code documentation comments (in any language), READMEs, mdbook docs,
//! and more.

pub use config::*;
pub use engine::*;
pub use error::*;
pub use parser::*;
pub use position::*;
pub use project::*;
pub use source_scanner::*;

pub mod config;
mod engine;
mod error;
pub(crate) mod lexer;
mod parser;
pub(crate) mod patterns;
mod position;
pub mod project;
mod source_scanner;
pub(crate) mod tokens;

#[cfg(test)]
mod __fixtures;
#[cfg(test)]
mod __tests;

//! `mdt` is a data-driven template engine for keeping documentation
//! synchronized across your project. It uses comment-based template tags to
//! define content once and distribute it to multiple locations — markdown
//! files, code documentation comments (in any language), READMEs, mdbook docs,
//! and more.

pub use config::*;
pub use engine::*;
pub use error::*;
pub use lexer::*;
pub use parser::*;
pub use patterns::PatternMatcher;
pub use position::*;
pub use project::*;
pub use source_scanner::*;
pub use tokens::*;

pub mod config;
mod engine;
mod error;
mod lexer;
mod parser;
pub mod patterns;
mod position;
pub mod project;
mod source_scanner;
mod tokens;

#[cfg(test)]
mod __fixtures;
#[cfg(test)]
mod __tests;

//! <!-- {=mdtCoreOverview|trim|linePrefix:"//! ":true} -->
//! `mdt_core` is the core library for the [mdt](https://github.com/ifiokjr/mdt) template engine. It provides the lexer, parser, project scanner, and template engine for processing markdown template tags. Content defined once in provider blocks can be distributed to consumer blocks across markdown files, code documentation comments, READMEs, and more.
//!
//! ## Processing Pipeline
//!
//! ```text
//! Markdown / source file
//!   → Lexer (tokenizes HTML comments into TokenGroups)
//!   → Pattern matcher (validates token sequences)
//!   → Parser (classifies groups, extracts names + transformers, matches open/close into Blocks)
//!   → Project scanner (walks directory tree, builds provider→content map + consumer list)
//!   → Engine (matches consumers to providers, applies transformers, replaces content)
//! ```
//!
//! ## Modules
//!
//! - [`config`] — Configuration loading from `mdt.toml`, including data source mappings, exclude/include patterns, and template paths.
//! - [`project`] — Project scanning and directory walking. Discovers provider and consumer blocks across all files in a project.
//! - [`source_scanner`] — Source file scanning for consumer tags inside code comments (Rust, TypeScript, Python, Go, Java, etc.).
//!
//! ## Key Types
//!
//! - [`Block`] — A parsed template block (provider or consumer) with its name, type, position, and transformers.
//! - [`Transformer`] — A pipe-delimited content filter (e.g., `trim`, `indent`, `linePrefix`) applied during injection.
//! - [`ProjectContext`] — A scanned project together with its loaded template data, ready for checking or updating.
//! - [`MdtConfig`] — Configuration loaded from `mdt.toml`.
//! - [`CheckResult`] — Result of checking a project for stale consumers.
//! - [`UpdateResult`] — Result of computing updates for consumer blocks.
//!
//! ## Data Interpolation
//!
//! Provider content supports [`minijinja`](https://docs.rs/minijinja) template variables populated from project files. The `mdt.toml` config maps source files to namespaces:
//!
//! ```toml
//! [data]
//! pkg = "package.json"
//! cargo = "Cargo.toml"
//! ```
//!
//! Then in provider blocks: `{{ pkg.version }}` or `{{ cargo.package.edition }}`.
//!
//! Supported formats: JSON, TOML, YAML, and KDL.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use mdt_core::project::scan_project_with_config;
//! use mdt_core::{check_project, compute_updates, write_updates};
//! use std::path::Path;
//!
//! let ctx = scan_project_with_config(Path::new(".")).unwrap();
//!
//! // Check for stale consumers
//! let result = check_project(&ctx).unwrap();
//! if !result.is_ok() {
//!     eprintln!("{} stale consumer(s) found", result.stale.len());
//! }
//!
//! // Update all consumer blocks
//! let updates = compute_updates(&ctx).unwrap();
//! write_updates(&updates).unwrap();
//! ```
//! <!-- {/mdtCoreOverview} -->

pub use config::*;
pub use engine::*;
pub use error::*;
pub use parser::*;
pub use position::*;
pub use project::*;
pub use source_scanner::*;

pub mod config;
mod engine;
#[allow(unused_assignments)]
mod error;
pub(crate) mod lexer;
mod parser;
pub(crate) mod patterns;
mod position;
pub mod project;
pub mod source_scanner;
pub(crate) mod tokens;

#[cfg(test)]
mod __fixtures;
#[cfg(test)]
mod __tests;

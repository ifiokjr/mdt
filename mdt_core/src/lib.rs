//! <!-- {=mdtCoreOverview|trim|linePrefix:"//! ":true} -->
//! `mdt_core` is the core library for [mdt](https://github.com/ifiokjr/mdt). It provides the lexer, parser, project scanner, and template engine behind the `mdt` CLI, language server, and MCP server. Content defined once in a provider block is distributed to consumer blocks across markdown files, code documentation comments, READMEs, and more.
//!
//! ## Processing Pipeline
//!
//! ```text
//! Markdown / source file
//!   → Lexer (tokenizes HTML comments into TokenGroups)
//!   → Pattern matcher (validates token sequences)
//!   → Parser (classifies groups, extracts names + transformers, matches open/close into Blocks)
//!   → Project scanner (walks the project, collects providers from *.t.md files and consumers everywhere)
//!   → Engine (renders providers, applies transformers, replaces consumer content)
//! ```
//!
//! ## Modules
//!
//! - [`config`]: loads `mdt.toml`, including data sources, scan patterns, padding, comparison mode, and formatters.
//! - [`project`]: walks the project and discovers provider and consumer blocks.
//! - [`source_scanner`]: finds tags inside code comments (Rust, TypeScript, Python, Go, Java, Dart, and more).
//! - [`init`]: the project setup shared by `mdt init` and the MCP `mdt_init` tool.
//!
//! ## Key Types
//!
//! - [`Block`]: a parsed provider, consumer, or inline block with its name, type, position, and transformers.
//! - [`Transformer`]: a pipe-delimited content filter (such as `trim`, `indent`, or `linePrefix`) applied during injection.
//! - [`ProjectContext`]: a scanned project with its loaded template data, ready for checking or updating.
//! - [`MdtConfig`]: configuration loaded from `mdt.toml`.
//! - [`CheckResult`]: stale consumers, stale files, orphans, and render errors found by a check.
//! - [`UpdateResult`]: the file contents to write, plus consumers skipped because their provider failed to render.
//!
//! ## Data Interpolation
//!
//! When `mdt.toml` has a `[data]` section, provider content is rendered with [`minijinja`](https://docs.rs/minijinja) using values from project files and commands:
//!
//! ```toml
//! [data]
//! pkg = "package.json"
//! cargo = "Cargo.toml"
//! ```
//!
//! Providers can then use `{{ pkg.version }}` or `{{ cargo.package.edition }}`.
//!
//! Supported sources: files and commands. Supported formats: text, JSON, TOML, YAML, KDL, and INI.
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
//! // Check that every consumer is linked and current
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
mod default_mdt_toml;
mod engine;
#[allow(unused_assignments)]
mod error;
mod index_cache;
pub mod init;
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

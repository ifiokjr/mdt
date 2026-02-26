# mdt_core

> core library for mdt (manage markdown templates)

<br />

[![Crate][crate-image]][crate-link] [![Docs][docs-image]][docs-link] [![Status][ci-status-image]][ci-status-link] [![Coverage][coverage-image]][coverage-link] [![Unlicense][unlicense-image]][unlicense-link]

<br />

<!-- {=mdtCoreOverview} -->

`mdt_core` is the core library for the [mdt](https://github.com/ifiokjr/mdt) template engine. It provides the lexer, parser, project scanner, and template engine for processing markdown template tags. Content defined once in provider blocks can be distributed to consumer blocks across markdown files, code documentation comments, READMEs, and more.

## Processing Pipeline

```text
Markdown / source file
  → Lexer (tokenizes HTML comments into TokenGroups)
  → Pattern matcher (validates token sequences)
  → Parser (classifies groups, extracts names + transformers, matches open/close into Blocks)
  → Project scanner (walks directory tree, builds provider→content map + consumer list)
  → Engine (matches consumers to providers, applies transformers, replaces content)
```

## Modules

- [`config`] — Configuration loading from `mdt.toml`, including data source mappings, exclude/include patterns, and template paths.
- [`project`] — Project scanning and directory walking. Discovers provider and consumer blocks across all files in a project.
- [`source_scanner`] — Source file scanning for consumer tags inside code comments (Rust, TypeScript, Python, Go, Java, etc.).

## Key Types

- [`Block`] — A parsed template block (provider or consumer) with its name, type, position, and transformers.
- [`Transformer`] — A pipe-delimited content filter (e.g., `trim`, `indent`, `linePrefix`) applied during injection.
- [`ProjectContext`] — A scanned project together with its loaded template data, ready for checking or updating.
- [`MdtConfig`] — Configuration loaded from `mdt.toml`.
- [`CheckResult`] — Result of checking a project for stale consumers.
- [`UpdateResult`] — Result of computing updates for consumer blocks.

## Data Interpolation

Provider content supports [`minijinja`](https://docs.rs/minijinja) template variables populated from project files. The `mdt.toml` config maps source files to namespaces:

```toml
[data]
pkg = "package.json"
cargo = "Cargo.toml"
```

Then in provider blocks: `{{ pkg.version }}` or `{{ cargo.package.edition }}`.

Supported formats: JSON, TOML, YAML, and KDL.

## Quick Start

```rust,no_run
use mdt_core::project::scan_project_with_config;
use mdt_core::{check_project, compute_updates, write_updates};
use std::path::Path;

let ctx = scan_project_with_config(Path::new(".")).unwrap();

// Check for stale consumers
let result = check_project(&ctx).unwrap();
if !result.is_ok() {
    eprintln!("{} stale consumer(s) found", result.stale.len());
}

// Update all consumer blocks
let updates = compute_updates(&ctx).unwrap();
write_updates(&updates).unwrap();
```

<!-- {/mdtCoreOverview} -->

## Installation

<!-- {=mdtCoreInstall} -->

```toml
[dependencies]
mdt_core = "0.6.0"
```

<!-- {/mdtCoreInstall} -->

<!-- {=mdtBadgeLinks:"mdt_core"} -->

[crate-image]: https://img.shields.io/crates/v/mdt_core.svg
[crate-link]: https://crates.io/crates/mdt_core
[docs-image]: https://docs.rs/mdt_core/badge.svg
[docs-link]: https://docs.rs/mdt_core/
[ci-status-image]: https://github.com/ifiokjr/mdt/workflows/ci/badge.svg
[ci-status-link]: https://github.com/ifiokjr/mdt/actions?query=workflow:ci
[coverage-image]: https://codecov.io/gh/ifiokjr/mdt/branch/main/graph/badge.svg
[coverage-link]: https://codecov.io/gh/ifiokjr/mdt
[unlicense-image]: https://img.shields.io/badge/license-Unlicence-blue.svg
[unlicense-link]: https://opensource.org/license/unlicense

<!-- {/mdtBadgeLinks} -->

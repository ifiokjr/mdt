# mdt_core

> core library for mdt (manage markdown templates)

<br />

[![Crate][crate-image]][crate-link] [![Docs][docs-image]][docs-link] [![Status][ci-status-image]][ci-status-link] [![Coverage][coverage-image]][coverage-link] [![Unlicense][unlicense-image]][unlicense-link]

<br />

<!-- {=mdtCoreOverview} -->

`mdt_core` is the core library for [mdt](https://github.com/ifiokjr/mdt). It provides the lexer, parser, project scanner, and template engine behind the `mdt` CLI, language server, and MCP server. Content defined once in a provider block is distributed to consumer blocks across markdown files, code documentation comments, READMEs, and more.

## Processing Pipeline

```text
Markdown / source file
  → Lexer (tokenizes HTML comments into TokenGroups)
  → Pattern matcher (validates token sequences)
  → Parser (classifies groups, extracts names + transformers, matches open/close into Blocks)
  → Project scanner (walks the project, collects providers from *.t.md files and consumers everywhere)
  → Engine (renders providers, applies transformers, replaces consumer content)
```

## Modules

- [`config`]: loads `mdt.toml`, including data sources, scan patterns, padding, comparison mode, and formatters.
- [`project`]: walks the project and discovers provider and consumer blocks.
- [`source_scanner`]: finds tags inside code comments (Rust, TypeScript, Python, Go, Java, Dart, and more).
- [`init`]: the project setup shared by `mdt init` and the MCP `mdt_init` tool.

## Key Types

- [`Block`]: a parsed provider, consumer, or inline block with its name, type, position, and transformers.
- [`Transformer`]: a pipe-delimited content filter (such as `trim`, `indent`, or `linePrefix`) applied during injection.
- [`ProjectContext`]: a scanned project with its loaded template data, ready for checking or updating.
- [`MdtConfig`]: configuration loaded from `mdt.toml`.
- [`CheckResult`]: stale consumers, stale files, orphans, and render errors found by a check.
- [`UpdateResult`]: the file contents to write, plus consumers skipped because their provider failed to render.

## Data Interpolation

When `mdt.toml` has a `[data]` section, provider content is rendered with [`minijinja`](https://docs.rs/minijinja) using values from project files and commands:

```toml
[data]
pkg = "package.json"
cargo = "Cargo.toml"
```

Providers can then use `{{ pkg.version }}` or `{{ cargo.package.edition }}`.

Supported sources: files and commands. Supported formats: text, JSON, TOML, YAML, KDL, and INI.

## Quick Start

```rust,no_run
use mdt_core::project::scan_project_with_config;
use mdt_core::{check_project, compute_updates, write_updates};
use std::path::Path;

let ctx = scan_project_with_config(Path::new(".")).unwrap();

// Check that every consumer is linked and current
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
mdt_core = "0.9.5"
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
[unlicense-image]: https://img.shields.io/badge/license-Unlicense-blue.svg
[unlicense-link]: https://opensource.org/license/unlicense

<!-- {/mdtBadgeLinks} -->

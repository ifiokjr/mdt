# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

mdt (manage **m**ark**d**own **t**emplates) is a data-driven template engine for keeping documentation synchronized across your project. It uses comment-based template tags to define content once and distribute it to multiple locations — markdown files, code documentation comments (in any language), READMEs, mdbook docs, and more.

### Core Concepts

1. **Content synchronization**: Provider blocks define content in `*.t.md` template files. Consumer blocks in other files reference providers by name and get replaced with the provider content on `mdt update`.

2. **Data interpolation** (planned, via `minijinja`): Provider content can reference data pulled from project files (e.g., `package.json` version, `Cargo.toml` metadata). A config file will map source files to namespaces for template variable access.

3. **Source file scanning** (planned): Consumer tags will be supported inside code comments (`//`, `#`, `///`, `//!`, etc.) in any language, not just markdown files. This enables keeping Rust doc comments, JSDoc, Python docstrings, etc. in sync with central template definitions.

4. **Transformers**: Pipe-delimited filters modify content during injection — `trim`, `indent`, `prefix`, `wrap`, `codeBlock`, `code`, `replace`. Example: `<!-- {=docs|prefix:"\n"|indent:"//! "} -->` turns content into Rust doc comments.

**Status:** Early development (v0.0.0)

## Build & Development

### Environment Setup

Uses `devenv` (Nix-based) for reproducible development environments. After cloning:

```sh
# Enter the dev shell (automatic with direnv, or manually):
devenv shell

# Install all tooling (cargo binaries):
install:all
```

Cargo binaries are managed via `cargo-run-bin` (versions pinned in `[workspace.metadata.bin]` in root `Cargo.toml`).

### Key Commands

```sh
# Build
cargo build --all-features        # Build all crates
build:all                          # Build all (cargo + mdbook)

# Test
cargo test                         # Run all tests
cargo nextest run                  # Run tests with nextest (faster)
test:all                           # Run all tests (cargo + doc tests)

# Lint & Format
lint:all                           # Run all checks (clippy + format)
lint:clippy                        # cargo clippy --all-features
lint:format                        # dprint check
fix:all                            # Auto-fix all (clippy + format)
fix:clippy                         # cargo clippy --fix
fix:format                         # dprint fmt

# Coverage
cargo llvm-cov                     # Code coverage via cargo-llvm-cov

# Semver checking
cargo semver-checks                # Check for semver violations
```

### Formatting

Formatting is handled by `dprint` (not `cargo fmt` directly). dprint delegates to `rustfmt` for `.rs` files, `nixfmt` for `.nix`, and `shfmt` for shell scripts. Always use `fix:format` or `dprint fmt` rather than running `rustfmt` directly.

Key style rules: hard tabs, max width 100, one import per line (`imports_granularity = "Item"`), imports grouped by `StdExternalCrate`.

## Architecture

### Workspace Crates

- **`crates/mdt`** — Core library. Provides the lexer, parser, pattern matcher, project scanner, and template engine for processing markdown template tags. Uses `minijinja` for template rendering (planned for data interpolation) and `miette` for error reporting.
- **`crates/mdt_cli`** — CLI tool. Provides `init`, `check`, and `update` commands for managing markdown templates via the command line. Uses `clap` for argument parsing.
- **`crates/mdt_lsp`** — LSP server (planned, `publish = false`). Will provide language server protocol support for editor integration using `tower-lsp`. Not published until real features are implemented.
- **`docs/`** — mdbook documentation.

### Internal Pipeline

```
Markdown source
  → markdown crate AST (extracts HTML comment nodes)
  → Lexer (tokenizes comments into TokenGroups)
  → Pattern matcher (validates token sequences)
  → Parser (classifies groups, extracts names + transformers, matches open↔close into Blocks)
  → Project scanner (walks directory tree, builds provider→content map + consumer list)
  → Engine (matches consumers to providers, applies transformers, replaces content)
```

### Template Syntax

**Provider tag** (defines a template block in `*.t.md` definition files):

```markdown
<!-- {@blockName} -->

Content to inject

<!-- {/blockName} -->
```

**Consumer tag** (marks where content should be injected):

```markdown
<!-- {=blockName} -->

This content gets replaced

<!-- {/blockName} -->
```

**Close tag** (shared by both):

```markdown
<!-- {/blockName} -->
```

**Filters and pipes:** Template values support pipe-delimited transformers:

```markdown
<!-- {=block|prefix:"\n"|indent:"  "} -->
```

**Available transformers:** `trim`, `trimStart`, `trimEnd`, `indent`, `prefix`, `wrap`, `codeBlock`, `code`, `replace`.

### File Conventions

- `*.t.md` — Template definition files containing provider blocks. Only provider blocks in these files are recognized.
- All other `.md`/`.mdx`/`.markdown` files — Scanned for consumer blocks.
- Hidden directories (`.git`, etc.), `node_modules`, and `target` are skipped during scanning.
- Future: source code files (`.rs`, `.ts`, `.py`, etc.) will also be scanned for consumer tags inside code comments.

### CLI Commands

- `mdt init [--path <dir>]` — Create a sample `template.t.md` file with getting-started instructions.
- `mdt check [--path <dir>] [--verbose]` — Verify all consumer blocks are up-to-date. Exits non-zero if any are stale.
- `mdt update [--path <dir>] [--verbose] [--dry-run]` — Update all consumer blocks with latest provider content.

### Future: Data Interpolation

Provider content will support `minijinja` template variables populated from project files. A config file (e.g., `.mdt.toml`) will map source files to namespaces:

```toml
# Example (planned syntax)
[data.pkg]
source = "package.json"

[data.cargo]
source = "Cargo.toml"
```

Then in templates: `Version: {{ pkg.version }}` or `Edition: {{ cargo.package.edition }}`.

## Toolchain

- **Rust:** 1.87.0 (stable), edition 2024, MSRV 1.86.0
- **Formatter:** dprint (orchestrates rustfmt, nixfmt, shfmt)
- **Linting:** clippy with strict workspace lints — `unsafe_code` and `unstable_features` are **denied**
- **Test runner:** cargo-nextest (also standard `cargo test`)
- **Coverage:** cargo-llvm-cov
- **Snapshot testing:** cargo-insta
- **Semver:** cargo-semver-checks
- **Release management:** knope (bot workflow)
- **Publishing:** cargo-workspaces (`cargo workspaces publish --from-git`)

## Release & Changelog Workflow

Uses [knope bot workflow](https://knope.tech/tutorials/bot-workflow/). Each publishable crate has its own changelog and version. `mdt_lsp` is excluded from releases until it has real functionality.

```sh
# Document a change (creates a changeset file in .changeset/):
knope document-change

# Prepare a release (bumps versions, updates changelogs):
knope release

# Publish to crates.io:
knope publish
```

Changesets should be highly detailed. Conventional commit scopes map to packages: `mdt`, `mdt_cli`. Extra changelog sections: `Notes` (type: `note`) and `Documentation` (type: `docs`).

### Changeset Requirement

**Every pull request that modifies code in any crate (`crates/`) MUST include at least one changeset file in `.changeset/`.** This ensures all changes are tracked in changelogs and version bumps are applied correctly.

To create a changeset interactively:

```sh
knope document-change
```

Or create one manually by adding a markdown file in `.changeset/` with TOML-style frontmatter:

```markdown
---
package_name: change_type
---

Detailed description of the change.
```

**Change types:**

- `major` — breaking changes
- `minor` — new features (backwards compatible)
- `patch` — bug fixes
- `docs` — documentation-only changes
- `note` — general notes

**Package names:** `mdt`, `mdt_cli` (not `mdt_lsp` — it's excluded from releases)

A single changeset file can reference multiple packages. Always run `dprint fmt .changeset/* --allow-no-files` after creating changeset files.

## Cargo Aliases

Defined in `.cargo/config.toml` — these proxy to `cargo-run-bin`:

- `cargo insta` — run cargo-insta
- `cargo llvm-cov` — run cargo-llvm-cov
- `cargo nextest` — run cargo-nextest
- `cargo semver-checks` — run cargo-semver-checks
- `cargo workspaces` — run cargo-workspaces

## Security Constraints

- `unsafe_code` is **denied** workspace-wide
- `unstable_features` is **denied** workspace-wide
- `clippy::correctness` is **denied** (not just warned)
- `clippy::wildcard_dependencies` is **denied**
- `Result::expect` is a disallowed method (use `unwrap_or_else` with explicit panic message instead)

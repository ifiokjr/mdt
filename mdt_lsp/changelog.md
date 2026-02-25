# Changelog

This file is maintained by `knope`.

## 0.4.0 (2026-02-25)

### Breaking Changes

#### Large refactor of codebase

A large refactor of the codebase to make it easier to navigate and improve releases.

## 0.2.1 (2026-02-24)

### Features

#### Enhanced LSP diagnostics to surface all errors that the CLI `check` command reports. The language server now detects:

- **Unclosed blocks**: Opening tags without matching close tags are reported as errors with the block name and position.
- **Unknown transformers**: Invalid transformer names (e.g., `|foobar`) are reported as errors.
- **Invalid transformer arguments**: Transformers with the wrong number of arguments are reported as errors.
- **Unused providers**: Provider blocks in template files that have no matching consumers are reported as warnings.
- **Name suggestions**: When a consumer references a missing provider, the LSP now suggests similar provider names using Levenshtein distance matching (e.g., "Did you mean: `greeting`?").

The parser was upgraded from `parse()` to `parse_with_diagnostics()` to capture parse-level diagnostics that were previously silently discarded.

Added `cargo-semver-checks` CI job to pull requests that detects breaking API changes in published crates and enforces that a `major` changeset is included when breakage is found. The job posts a PR comment with the semver-checks output on failure.

## 0.2.0 (2026-02-24)

### Breaking Changes

#### Rename the core library crate from `mdt` to `mdt_core` to resolve a crate name conflict on crates.io. The `mdt` name on crates.io is owned by a different project ("a markdown tool for writers").

This is a breaking change for any downstream code that depends on the `mdt` crate directly. All `use mdt::` imports must be updated to `use mdt_core::`.

The CLI binary name remains `mdt` (unchanged). The `mdt_cli`, `mdt_lsp`, and `mdt_mcp` crate names are also unchanged.

#### Migration

Replace all occurrences of `use mdt::` with `use mdt_core::` in your code, and update your `Cargo.toml` dependency from `mdt` to `mdt_core`.

## 0.1.0 (2026-02-24)

### Breaking Changes

- Bump up version

## 0.0.1 (2026-02-20)

### Features

#### Implement the mdt language server (LSP).

**LSP features:**

- **Diagnostics:** Warns about stale consumer blocks (content out of date), missing providers (consumer references non-existent provider), and provider blocks in non-template files.
- **Hover:** Shows provider content preview when hovering over consumer tags. Shows consumer count and locations when hovering over provider tags. Displays transformer chain and source file path.
- **Completion:** Suggests block names when typing inside `{=`, `{@`, or `{/` tags. Suggests transformer names (`trim`, `indent`, `codeBlock`, etc.) after pipe `|` characters with usage descriptions.
- **Go to Definition:** Navigates from consumer tag to its provider definition in the template file. From provider tags, navigates to all consumer locations.
- **Document Symbols:** Lists all provider (`@name`) and consumer (`=name`) blocks in the editor outline/breadcrumb.
- **Code Actions:** Offers "Update block" quick-fix for stale consumer blocks that replaces content with the latest provider output.

**Incremental updates:** The language server performs incremental document updates on save instead of full project rescans, with full rescans only when `mdt.toml` changes.

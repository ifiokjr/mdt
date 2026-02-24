# Changelog

This file is maintained by `knope`.

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

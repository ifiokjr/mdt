# Changelog

This file is maintained by `knope`.

## [0.9.0](https://github.com/ifiokjr/mdt/releases/tag/v0.9.0) (2026-07-04)

### 🚀 Feature

#### Expose MCP diagnostics through MDT_LOG tracing

The MCP server now initializes `tracing-subscriber` with an `EnvFilter` sourced from `MDT_LOG`. Logs are emitted to stderr so tracing does not corrupt MCP stdio messages or structured tool responses.

This makes agent-driven workflows easier to debug when a check, update, preview, or init call behaves unexpectedly, while leaving normal tool output unchanged unless logging is explicitly enabled.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Return structured JSON envelopes from MCP tools

MCP tools now return more consistent JSON-oriented `structured_content` envelopes for `mdt_check`, `mdt_update`, `mdt_preview`, and `mdt_init`. Text content is still preserved for clients that display plain tool messages.

`mdt_preview` now behaves more like an authoring workflow by returning per-consumer rendered output, parameterized previews, render details, and mismatch information. Check and update responses also surface undefined-variable warnings so agents can reason about template problems without scraping text.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #116](https://github.com/ifiokjr/mdt/pull/116) · _Closed issues:_ [#108](https://github.com/ifiokjr/mdt/issues/108) · _Related issues:_ [#112](https://github.com/ifiokjr/mdt/issues/112), [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Support lenient comparison in MCP checks

The MCP `mdt_check` tool now honors `[check] comparison = "lenient"`, matching the CLI behavior for whitespace-tolerant verification. Agent workflows can therefore distinguish real template drift from harmless formatter whitespace changes.

The MCP response still reports mismatches when normalized content differs, and update behavior remains exact.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Run configured formatters from MCP tools

MCP `mdt_update` and `mdt_check` now honor opt-in `[[formatters]]` configuration. Agent workflows can preview, update, and verify formatter-aware template output using the same configuration as the CLI.

This keeps MCP-driven synchronization aligned with project formatters while preserving structured diagnostics for formatter failures.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

### 🐛 Fixed

#### Respect existing template locations during MCP init

The MCP initialization flow now follows the same bootstrap rules as the CLI. It detects existing canonical and legacy template locations before writing starter content, avoiding unnecessary root-level `template.t.md` files in projects that already have a usable template directory.

Generated READMEs and initialization guidance now consistently describe `.templates/template.t.md` as the preferred starter path.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Reuse shared core helpers in MCP tools

MCP tools now use shared `mdt_core` helpers for project-root resolution and relative path display. Agent-facing responses therefore stay aligned with CLI and LSP behavior while preserving the existing MCP tool contract.

The cleanup reduces duplicated app-surface code and makes future fixes easier to apply consistently.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Update rmcp server result construction

`rmcp` has been updated to 2.1.0. The MCP server now uses `ServerInfo::new().with_instructions()` for server metadata, the `CallToolResult::success()` and `CallToolResult::error()` constructors, and `ContentBlock` for text tool responses.

This keeps the MCP integration aligned with the current `rmcp` API while preserving the same tool behavior for clients.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141)

## 0.4.1 (2026-02-25)

### Features

#### Add positional block arguments to provider and consumer tags.

Provider blocks can now declare named parameters using `:"param_name"` syntax after the block name. Consumer blocks pass string values as positional arguments in the same position. The provider's parameter names become template variables that are interpolated with the consumer's argument values during rendering.

**Syntax:**

```markdown
<!-- Provider declares a parameter -->
<!-- {@badges:"crate_name"} -->

[![crates.io](https://img.shields.io/crates/v/{{ crate_name }})]

<!-- {/badges} -->

<!-- Consumer passes a value -->
<!-- {=badges:"mdt_core"} -->
<!-- {/badges} -->

<!-- Another consumer with different value -->
<!-- {=badges:"mdt_cli"} -->
<!-- {/badges} -->
```

Arguments work alongside existing features:

- Multiple arguments: `<!-- {@tmpl:"a":"b":"c"} -->`
- With transformers: `<!-- {=badges:"mdt_core"|trim} -->`
- With data interpolation: `{{ crate_name }}` and `{{ pkg.version }}` can coexist
- Single-quoted strings: `<!-- {@tmpl:'param'} -->`

Argument count mismatches between provider parameters and consumer arguments are reported as render errors during `check` and skipped during `update`.

This is a breaking change because the `Block` struct now includes an `arguments: Vec<String>` field.

### Fixes

#### Expand test coverage for LSP and MCP server crates with tests for error handling, edge cases, and tool functionality.

#### `mdt_lsp`

- Test `compute_diagnostics` with template data interpolation, stale diagnostic payloads, and multiple consumers in a single document.
- Test `compute_completions` with multiple providers returning all names, and verify transformer completions include all known transformers with correct kinds and sort text.
- Test `block_name_completions` returns REFERENCE kind with file detail.
- Test `compute_document_symbols` full range spans opening to closing tag.
- Test `compute_code_actions` edit targets content between tags and multiple stale blocks produce separate actions.
- Test `levenshtein_distance` for single char, case sensitivity, and symmetry.
- Test `suggest_similar_names` for empty providers and max 3 results.
- Test `to_lsp_position` saturating subtraction at zero.
- Test `parse_document_content` for Python files and empty strings.
- Test `compute_hover` for provider content in code blocks and consumer source paths.
- Test `compute_goto_definition` for cursor between blocks.

#### `mdt_mcp`

- Test error cases: invalid mdt.toml config for check, update, list, get_block, and preview tools.
- Test edge cases: empty projects, providers-only projects, consumers-only projects.
- Test `list` tool: consumer transformers, summary format, trimmed provider content, relative file paths.
- Test `update` tool: dry-run reports counts, empty project no-op, synced project dry-run, idempotent updates, multi-file updates.
- Test `get_block` tool: provider with no consumers, raw vs rendered content with data interpolation, multiple consumer files.
- Test `preview` tool: data interpolation, transformer display, multiple consumers.
- Test `check` tool: stale block names and file paths in output, multiple stale blocks, missing data files.
- Test `init` tool: nested directories.
- Test `scan_ctx` graceful handling of nonexistent paths.

#### Fix clippy warnings across the workspace.

- Replace `map().unwrap_or()` with `map_or()` in `engine.rs`.
- Suppress `too_many_arguments` on `scan_project_with_options` (to be refactored separately).
- Suppress `only_used_in_recursion` on `walk_dir`'s `root` parameter.
- Suppress `variant_size_differences` on `PaddingValue` enum.
- Suppress `unused_assignments` from thiserror-generated code in `MdtError`.
- Suppress `struct_excessive_bools` on `MdtCli`.
- Fix redundant closures in `mdt_lsp` (`map(|p| p.into_owned())` to `map(Cow::into_owned)`).
- Suppress deprecated `root_uri` field usage in LSP (separate migration PR).
- Suppress `disallowed_methods` false positives from `tokio::test` macro in `mdt_mcp` tests.
- Fix `cmp_owned` warning in `mdt_mcp` tests.
- Fix unnecessary qualifications, single-char string patterns, doc comment backticks, `approx_constant` errors, and `float_cmp` warnings in `mdt_core` tests.

## 0.4.0 (2026-02-25)

### Breaking Changes

#### Large refactor of codebase

A large refactor of the codebase to make it easier to navigate and improve releases.

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

#### Add MCP server for mdt.

Provides a Model Context Protocol server with 6 tools: `mdt_check`, `mdt_update`, `mdt_list`, `mdt_get_block`, `mdt_preview`, and `mdt_init`. The server communicates over stdin/stdout and can be launched via `mdt mcp`. Built on the `rmcp` SDK.

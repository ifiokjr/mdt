## [0.9.0](https://github.com/ifiokjr/mdt/releases/tag/v0.9.0) (2026-07-04)

Grouped release for `mdt`.

### 💥 Breaking Change

#### Add formatter-aware full-file normalization

_Packages:_ _mdt_core_

`mdt_core` now supports opt-in `[[formatters]]` configuration in `mdt.toml`. Matching formatter commands run against the entire updated target file in declaration order using stdin/stdout, enabling formatter-aware drift detection and update output.

This is a major release because public constructible structs such as `MdtConfig` and `ProjectContext` gained fields. Downstream crates that build these structs with literals must add the new fields or use defaults/builders where available.

```rust
let config = MdtConfig {
    formatters: Vec::new(),
    ..existing_config
};
```

Formatter failures now surface as dedicated diagnostics so callers can distinguish rendering problems from formatter command failures.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Tighten core error handling and API ergonomics

_Packages:_ _mdt_core_

`mdt_core` now applies a set of Rust API and implementation best practices across error handling, allocation behavior, and public function documentation. The changes remove broad `AnyError` style aliases in favor of `MdtResult`, add `# Errors` documentation to result-returning public functions, mark useful return values with `#[must_use]`, and pre-allocate vectors in hot paths.

This is a major release because public aliases were removed and `render_template` now returns `Cow<'_, str>` to avoid allocations when no template syntax is present. Downstream callers may need to adjust type annotations or convert borrowed results when an owned `String` is required.

```rust
let rendered = render_template(template, &data)?;
let owned: String = rendered.into_owned();
```

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Add official assistant setup profiles

_Packages:_ _mdt_cli_

The CLI now includes an `mdt assist` command that prints official assistant setup profiles. It focuses on practical adoption by producing ready-to-copy MCP configuration snippets and suggested repo-local guidance for Claude, Cursor, Copilot, Pi, and generic MCP clients.

This is a major release because the public CLI command model gains a new `Commands::Assist` variant. Downstream crates that exhaustively match command variants will need to handle the new case.

```rust
match command {
    Commands::Assist(args) => run_assist(args),
    other => run_existing_command(other),
}
```

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #117](https://github.com/ifiokjr/mdt/pull/117) · _Closed issues:_ [#109](https://github.com/ifiokjr/mdt/issues/109) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Publish the CLI through npm packages

_Packages:_ _@m-d-t/cli-darwin-arm64_, _@m-d-t/cli-darwin-x64_, _@m-d-t/cli-linux-arm64-gnu_, _@m-d-t/cli-linux-arm64-musl_, _@m-d-t/cli-linux-x64-gnu_, _@m-d-t/cli-linux-x64-musl_, _@m-d-t/cli-win32-x64-msvc_, _@m-d-t/cli-win32-arm64-msvc_

`mdt` now has an official npm distribution channel. Releases prepare a top-level `@m-d-t/cli` package plus platform-specific binary packages for Linux, macOS, and Windows.

Users can install the CLI globally with npm or run it on demand through npx, making adoption easier in JavaScript-heavy projects and environments that do not already have Rust tooling installed.

```bash
npx @m-d-t/cli init
```

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #121](https://github.com/ifiokjr/mdt/pull/121) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

### 🚀 Feature

#### Instrument core template processing with tracing

_Packages:_ _mdt_core_

`mdt_core` now emits structured tracing spans and events around important template-processing boundaries. Public API entry points are annotated with `#[instrument]`, and the engine records `debug!`, `trace!`, and `warn!` events while loading projects, resolving providers, rendering consumers, and reporting notable processing states.

This makes failures and performance issues easier to diagnose from CLI, LSP, and MCP callers without changing the core API or the rendered markdown output.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Add lenient block comparison to configuration

_Packages:_ _mdt_core_

`mdt_core` now supports `[check] comparison = "lenient"` for whitespace-tolerant block comparison. In lenient mode, the engine normalizes blank-line counts and trailing whitespace before comparing expected and actual consumer content.

This reduces false-positive stale-block reports after external formatter rewrites. Update operations remain exact and continue to write the rendered provider output byte-for-byte.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Expose structured CLI logs with MDT_LOG

_Packages:_ _mdt_cli_

The CLI now initializes `tracing-subscriber` with an `EnvFilter` sourced from `MDT_LOG`. This gives operators and contributors a consistent way to inspect command execution without adding ad-hoc debug output or changing normal terminal output.

The subscriber is installed at process startup and defaults to quiet behavior unless the environment variable is set. Users can opt into targeted diagnostics for parser, project-loading, update, or check flows while preserving the existing user-facing command experience.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Support lenient whitespace comparison in check

_Packages:_ _mdt_cli_

`mdt check` now honors `[check] comparison = "lenient"` for whitespace-tolerant verification. This mode allows projects to keep external formatters enabled without reporting stale blocks for harmless whitespace rewrites.

The command still reports meaningful content drift, while `mdt update` continues to write exact rendered bytes regardless of the comparison setting.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Run configured formatters during check and update

_Packages:_ _mdt_cli_

`mdt update` and `mdt check` now support opt-in `[[formatters]]` configuration. When a formatter matches a target file, `mdt` runs the formatter against the full updated file so template output converges with project tools such as dprint or Prettier.

This lets teams keep normal formatting workflows enabled while still detecting formatter-aware template drift during checks.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Publish the CLI through npm packages

_Packages:_ _mdt_cli_

`mdt` now has an official npm distribution channel. Releases prepare a top-level `@m-d-t/cli` package plus platform-specific binary packages for Linux, macOS, and Windows.

Users can install the CLI globally with npm or run it on demand through npx, making adoption easier in JavaScript-heavy projects and environments that do not already have Rust tooling installed.

```bash
npx @m-d-t/cli init
```

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #121](https://github.com/ifiokjr/mdt/pull/121) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Publish official agent skills for mdt

_Packages:_ _mdt_cli_

`mdt` now publishes an official `@m-d-t/skills` npm package for Pi and other harnesses that support the Agent Skills standard. The package includes quick-start instructions, MCP tool guidance, and a detailed reference for template syntax, transformers, interpolation, inline blocks, configuration, CLI commands, MCP tools, and source-file patterns.

The release tooling now generates and publishes the skills package alongside the CLI, and `mdt assist pi` points users toward the packaged skill.

```sh
pi install npm:@m-d-t/skills
pi -e npm:@m-d-t/skills
```

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #128](https://github.com/ifiokjr/mdt/pull/128) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Expose LSP diagnostics through MDT_LOG tracing

_Packages:_ _mdt_lsp_

The language server now initializes `tracing-subscriber` with an `EnvFilter` sourced from `MDT_LOG`. Logs are written to stderr so tracing never interferes with the JSON-RPC protocol carried over stdio.

This gives editor integrations a safe opt-in diagnostics path for initialization, document updates, and template checks while preserving the default quiet behavior expected by LSP clients.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Expose MCP diagnostics through MDT_LOG tracing

_Packages:_ _mdt_mcp_

The MCP server now initializes `tracing-subscriber` with an `EnvFilter` sourced from `MDT_LOG`. Logs are emitted to stderr so tracing does not corrupt MCP stdio messages or structured tool responses.

This makes agent-driven workflows easier to debug when a check, update, preview, or init call behaves unexpectedly, while leaving normal tool output unchanged unless logging is explicitly enabled.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Return structured JSON envelopes from MCP tools

_Packages:_ _mdt_mcp_

MCP tools now return more consistent JSON-oriented `structured_content` envelopes for `mdt_check`, `mdt_update`, `mdt_preview`, and `mdt_init`. Text content is still preserved for clients that display plain tool messages.

`mdt_preview` now behaves more like an authoring workflow by returning per-consumer rendered output, parameterized previews, render details, and mismatch information. Check and update responses also surface undefined-variable warnings so agents can reason about template problems without scraping text.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #116](https://github.com/ifiokjr/mdt/pull/116) · _Closed issues:_ [#108](https://github.com/ifiokjr/mdt/issues/108) · _Related issues:_ [#112](https://github.com/ifiokjr/mdt/issues/112), [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Support lenient comparison in MCP checks

_Packages:_ _mdt_mcp_

The MCP `mdt_check` tool now honors `[check] comparison = "lenient"`, matching the CLI behavior for whitespace-tolerant verification. Agent workflows can therefore distinguish real template drift from harmless formatter whitespace changes.

The MCP response still reports mismatches when normalized content differs, and update behavior remains exact.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Run configured formatters from MCP tools

_Packages:_ _mdt_mcp_

MCP `mdt_update` and `mdt_check` now honor opt-in `[[formatters]]` configuration. Agent workflows can preview, update, and verify formatter-aware template output using the same configuration as the CLI.

This keeps MCP-driven synchronization aligned with project formatters while preserving structured diagnostics for formatter failures.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

### 🐛 Fixed

#### Extract shared app-surface helpers into core

_Packages:_ _mdt_core_

`mdt_core` now exposes shared helpers for project-root resolution, relative path display, and similar-name scoring. These utilities centralize behavior that was previously reimplemented by multiple application surfaces.

The extraction keeps user-facing behavior stable while making CLI, LSP, and MCP code easier to maintain consistently.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Add package repository metadata

_Packages:_ _mdt_core_, _mdt_cli_, _mdt_lsp_, _@m-d-t/cli_, _@m-d-t/cli-darwin-arm64_, _@m-d-t/cli-darwin-x64_, _@m-d-t/cli-linux-arm64-gnu_, _@m-d-t/cli-linux-arm64-musl_, _@m-d-t/cli-linux-x64-gnu_, _@m-d-t/cli-linux-x64-musl_, _@m-d-t/cli-win32-x64-msvc_, _@m-d-t/cli-win32-arm64-msvc_, _@m-d-t/skills_

Cargo and npm package manifests now include package-specific repository URLs. This keeps package metadata aligned with monochange manifest linting and points registry users directly to each package's source directory.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #153](https://github.com/ifiokjr/mdt/pull/153)

#### Update logos and remove the empty skip regex

_Packages:_ _mdt_core_

`logos` has been updated to 0.16.1. The tokenizer no longer uses the `#[logos(skip r"")]` attribute because the newer release rejects empty regular expressions that can match the empty string.

This keeps the lexer compatible with the current `logos` API without changing tokenization behavior for valid input.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Respect terminal color settings in check output

_Packages:_ _mdt_cli_

`mdt check` now applies ANSI color handling consistently across diagnostics and stale-block summaries. Color is enabled when the terminal supports it or `CLICOLOR_FORCE` is set, and it remains disabled when users pass `--no-color`, set `NO_COLOR`, or set `CLICOLOR=0`.

The result is clearer interactive output without surprising color in scripts or environments that explicitly request plain text.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #120](https://github.com/ifiokjr/mdt/pull/120) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Reuse shared core helpers in CLI surfaces

_Packages:_ _mdt_cli_

The CLI now uses shared `mdt_core` helpers for project-root resolution, relative path display, and similar-name scoring. This reduces duplicated logic between app surfaces while preserving the same command behavior and diagnostics users already expect.

Keeping these concerns in one place makes future fixes less likely to drift between CLI, LSP, and MCP integrations.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Rename npm packages under the m-d-t scope

_Packages:_ _mdt_cli_

The npm distribution has moved from the `@ifi` scope to the `@m-d-t` organization. The top-level CLI package is now `@m-d-t/cli`, the skills package is `@m-d-t/skills`, and all platform-specific binary packages now use the `@m-d-t/cli-*` naming pattern.

This aligns npm package names with the project name and makes the distribution easier to recognize in package registries and install commands.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #130](https://github.com/ifiokjr/mdt/pull/130) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Reuse shared core helpers in the LSP server

_Packages:_ _mdt_lsp_

The language server now uses shared `mdt_core` helpers for project-root resolution and relative path display. This keeps editor diagnostics aligned with the CLI and MCP surfaces without changing the LSP protocol behavior.

Centralizing this logic reduces the chance that path formatting or project discovery diverges across integrations.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Respect existing template locations during MCP init

_Packages:_ _mdt_mcp_

The MCP initialization flow now follows the same bootstrap rules as the CLI. It detects existing canonical and legacy template locations before writing starter content, avoiding unnecessary root-level `template.t.md` files in projects that already have a usable template directory.

Generated READMEs and initialization guidance now consistently describe `.templates/template.t.md` as the preferred starter path.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Reuse shared core helpers in MCP tools

_Packages:_ _mdt_mcp_

MCP tools now use shared `mdt_core` helpers for project-root resolution and relative path display. Agent-facing responses therefore stay aligned with CLI and LSP behavior while preserving the existing MCP tool contract.

The cleanup reduces duplicated app-surface code and makes future fixes easier to apply consistently.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Update rmcp server result construction

_Packages:_ _mdt_mcp_

`rmcp` has been updated to 2.1.0. The MCP server now uses `ServerInfo::new().with_instructions()` for server metadata, the `CallToolResult::success()` and `CallToolResult::error()` constructors, and `ContentBlock` for text tool responses.

This keeps the MCP integration aligned with the current `rmcp` API while preserving the same tool behavior for clients.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141)

<details>
<summary><strong>📖 Documentation</strong></summary>

#### Improve first-time installation and quick-start guidance

_Packages:_ _mdt_cli_

The installation and quick-start documentation now better serves users who want `mdt` without building Rust source locally. It recommends prebuilt release binaries for non-Rust projects, removes stale version-pinned snippets, and presents a concise first-run workflow.

The new quick start shows how to keep a README section and a source-doc comment synchronized from a single provider, giving new users a practical end-to-end success path.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #113](https://github.com/ifiokjr/mdt/pull/113) · _Closed issues:_ [#106](https://github.com/ifiokjr/mdt/issues/106) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Clarify the documentation drift problem

_Packages:_ _mdt_cli_

The README and guide introduction now lead with the core problem `mdt` solves: keeping README sections, source-doc comments, and docs-site content synchronized as projects evolve. The positioning is clearer for library and tool maintainers evaluating whether markdown templates fit their workflow.

The updated copy also explains how editor and agent integrations support a human-first documentation process instead of replacing normal authoring practices.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #114](https://github.com/ifiokjr/mdt/pull/114) · _Closed issues:_ [#107](https://github.com/ifiokjr/mdt/issues/107) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Add adoption walkthroughs for real documentation flows

_Packages:_ _mdt_cli_

The documentation now includes proof-of-value and migration walkthroughs that show how to adopt `mdt` in realistic projects. The examples cover synchronizing README content, source documentation, and docs-site pages without forcing teams to rewrite their documentation process.

These guides make it easier to evaluate the tool, migrate incrementally, and understand where provider and consumer blocks fit in existing markdown and source files.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #118](https://github.com/ifiokjr/mdt/pull/118) · _Closed issues:_ [#110](https://github.com/ifiokjr/mdt/issues/110) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

</details>

<details>
<summary><strong>🔨 Refactor</strong></summary>

#### Remove the legacy npm source folder

_Packages:_ _@m-d-t/cli_, _@m-d-t/skills_

The old `npm/` tree has been removed now that npm packages live under `packages/`. Tests and repository metadata now point at the generated package launcher and package directories under `packages/`.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #147](https://github.com/ifiokjr/mdt/pull/147) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

</details>

## 0.7.0 (2026-03-02)

### Breaking Changes

- Add `if` conditional transformer for selectively including block content based on data values. The `if` transformer takes a dot-separated data path as an argument and includes the block content only when the referenced value is truthy (exists and is not false, null, empty string, or zero). Example usage: `<!-- {=block|if:"config.features.enabled"} -->`.

#### Harden public API and improve crate documentation.

**Breaking (`mdt_core`):** Add `#[non_exhaustive]` to all 9 public enums (`MdtError`, `ParseDiagnostic`, `Argument`, `TransformerType`, `BlockType`, `PaddingValue`, `CodeBlockFilter`, `Token`, `DiagnosticKind`). This prevents downstream exhaustive pattern matching and allows new variants to be added in future minor releases without breaking changes. Downstream code matching on these enums must add a wildcard (`_`) arm.

**Breaking (`mdt_core`):** Make `source_scanner` module public (`pub mod source_scanner`). Previously it was private with items re-exported at the crate root via `pub use source_scanner::*`. The module is now directly accessible, fixing rustdoc link warnings for `[`source_scanner`]` references in crate-level documentation.

**`mdt_lsp`:** Add crate-level documentation using the `mdtLspOverview` template block, providing an overview of LSP capabilities and usage instructions directly in `lib.rs`.

**`mdt_mcp`:** Add crate-level documentation using the `mdtMcpOverview` template block, providing an overview of MCP tools and configuration directly in `lib.rs`.

**`mdt_cli`:** Add wildcard arms to `DiagnosticKind` match statements for forward compatibility with new diagnostic kinds.

**All crates:** Replace `tokio` `features = ["full"]` with minimal required feature sets — `mdt_cli` uses `["rt-multi-thread"]`, `mdt_lsp` uses `["rt-multi-thread", "macros", "sync", "io-std"]`, `mdt_mcp` uses `["rt-multi-thread", "macros"]`. This makes dependency requirements explicit.

**Config:** Remove stale data entries from `mdt.toml` (`cargo_mdt_core`, `cargo_mdt_cli`, `cargo_mdt_lsp`, `cargo_mdt_mcp`) that referenced individual crate `Cargo.toml` files no longer needed since crates use `version = { workspace = true }`.

#### Add a new public `Commands::Info` variant to `mdt_cli` and improve human-readable CLI output formatting (`mdt check` and new `mdt info`).

This is marked major because `Commands` is a public enum and adding a variant is a breaking change for exhaustive matches in downstream crates.

#### Expand config and data-source capabilities in `mdt_core`:

- Add config discovery precedence across `mdt.toml`, `.mdt.toml`, and `.config/mdt.toml`.
- Add typed `[data]` entries (`{ path, format }`) while keeping string-path compatibility.
- Add `ini` data format support.
- Expose new config/data APIs (`CONFIG_FILE_CANDIDATES`, `MdtConfig::resolve_path`, `DataSource`, `TypedDataSource`).

This is marked major because `MdtConfig.data` changes type from `HashMap<String, PathBuf>` to `HashMap<String, DataSource>`.

### Features

- Add `--watch` flag to `mdt check` command. When enabled, the check command monitors the project directory for file changes and automatically re-runs the check whenever files are modified or created. Uses 200ms debouncing to avoid redundant checks during rapid file changes. Unlike single-run mode, watch mode does not exit with a non-zero status code on stale consumers -- it prints the results and continues watching.
- Add `references` and `rename` support to the LSP server. `textDocument/references` returns all provider and consumer blocks sharing the same name. `textDocument/rename` renames a block name across all provider and consumer tags (both opening and closing tags) in the workspace.

#### Add cache observability across core and CLI diagnostics.

- Persist cache telemetry counters in the project index cache (scan count, full-hit count, cumulative reused/reparsed file counts, and last scan details).
- Expose cache inspection APIs from `mdt_core::project` for diagnostics surfaces.
- Extend `mdt info` with a cache section in text and JSON output (artifact health, schema/key compatibility, hash mode, cumulative metrics, and last scan summary).
- Extend `mdt doctor` with cache checks for artifact validity, hash mode guidance, and efficiency trend heuristics.
- Add unit/e2e/snapshot coverage and docs updates for the new observability output.

### Fixes

- Switch LSP text document synchronization from full to incremental mode. The server now receives only changed text ranges instead of the entire document content on each edit, improving performance for large files. Includes proper UTF-16 offset handling for LSP position conversion.

#### Fix collapsed newlines in `mdtBadgeLinks` provider block in `template.t.md`. The multi-line link reference definitions were accidentally collapsed to a single line by an external markdown formatter (`dprint fmt`) that doesn't recognize `{{ }}` template syntax in URLs as valid link definitions.

Restored the template content to its correct multi-line format. Added unit tests and CLI integration tests to verify that `mdt update` preserves newlines in multi-line content through the full scan → render → update pipeline, including idempotency after write-back.

### Documentation

- Fix broken markdown rendering in all crate READMEs. Badge reference links were collapsed onto a single line, causing them to render as plain text instead of clickable badge images. Each reference link definition now appears on its own line. Also added reusable mdt template blocks for LSP overview, MCP overview, CLI install, and contributing sections. Updated `mdt_core` title from `mdt` to `mdt_core`. Replaced stale `mdt_lsp` README content with mdt template blocks. Updated root README with crate table and streamlined contributing section.

## 0.6.0 (2026-02-26)

### Breaking Changes

#### Unify all workspace crates under a single shared version.

Previously each crate (`mdt_core`, `mdt_cli`, `mdt_lsp`, `mdt_mcp`) maintained its own independent version, which led to version drift — e.g., `mdt_core` at 0.5.0 while `mdt_lsp` and `mdt_mcp` were still at 0.4.1. This made it harder to reason about compatibility and complicated the release process with per-crate changelogs and tag prefixes (`mdt_cli/v0.4.1`).

All crates now inherit `version = { workspace = true }` from the root `Cargo.toml` workspace version. The knope release configuration has been consolidated from four separate `[packages.*]` sections into a single `[package]` with all versioned files and dependencies listed together. Releases now use a single changelog (`changelog.md`) and simplified version tags (`v0.5.0` instead of `mdt_cli/v0.5.0`).

This is a breaking change to the release workflow and tag format, not to the library APIs.

### Fixes

- Add integration tests for positional block arguments in LSP and MCP servers.
- Add `mdt_mcp` to workspace members so it is built, tested, and published through normal CI workflows.

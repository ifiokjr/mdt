# Changelog

This file is maintained by `knope`.

## 0.4.0 (2026-02-25)

### Breaking Changes

#### Large refactor of codebase

A large refactor of the codebase to make it easier to navigate and improve releases.

#### Replace `pad_blocks` boolean with `[padding]` configuration section.

The top-level `pad_blocks = true` setting has been replaced with a `[padding]` section that provides fine-grained control over blank lines between block tags and their content:

```toml
[padding]
before = 0 # content on next line (no blank lines)
after = 0
```

The `before` and `after` values accept:

- `false` — Content appears inline with the tag (no newline separator).
- `0` — Content on the very next line (one newline, no blank lines). **Recommended for projects using formatters** like `rustfmt` or `dprint`, as it minimizes whitespace that formatters might alter.
- `1` — One blank line between the tag and content (equivalent to the old `pad_blocks = true` behavior for source files with comment prefixes).
- `2` — Two blank lines, and so on.

When `[padding]` is present but values are omitted, `before` and `after` default to `1`.

**Migration:** Replace `pad_blocks = true` with `[padding]` in your `mdt.toml`. For the same behavior as before, use `[padding]` with no values (defaults to `before = 1, after = 1`). For compatibility with code formatters, use `before = 0, after = 0`.

### Features

#### Consolidate `[ignore]` into `[exclude]` and add new exclusion options.

**Breaking:** The `[ignore]` config section has been removed. Its functionality is now part of `[exclude]`, which uses gitignore-style patterns (supporting negation `!`, directory markers `/`, and all standard gitignore wildcards). Existing `[ignore]` patterns should be moved to `[exclude] patterns`.

**New `[exclude]` sub-properties:**

- `markdown_codeblocks`: Controls whether mdt tags inside fenced code blocks in source files are processed. Can be set to `true` (skip all code blocks), a string like `"ignore"` (skip code blocks whose info string contains the string), or an array of strings (skip code blocks matching any). Defaults to `false`.

- `blocks`: An array of block names to exclude from processing. Any provider or consumer block whose name appears in this list is completely ignored during scanning — it won't be matched, checked, or updated.

**DevEnv integration:** Added `mdt update --ignore-unused-blocks` to the `fix:all` command in `devenv.nix` (runs before `dprint fmt` to ensure content is updated then formatted).

#### Use mdt template blocks for `mdt_core` library documentation and fix formatter compatibility.

**Template blocks in lib docs:** Replace hand-written doc comments on `Block`, `Transformer`, `Argument` structs in `parser.rs` and the module-level doc comment in `lib.rs` with mdt consumer blocks that pull content from `template.t.md` provider blocks. This ensures documentation stays synchronized across the codebase.

**Formatter compatibility fixes:**

- Set `[padding] before = 0, after = 0` in project `mdt.toml` to eliminate blank lines between tags and content that formatters would modify.
- Disable `wrap_comments` and `format_code_in_doc_comments` in `rustfmt.toml` to prevent rustfmt from reflowing doc comment text and reformatting code blocks, which would break the `mdt update → dprint fmt → mdt check` cycle.
- Fix `linePrefix` and `lineSuffix` transformers to trim trailing/leading whitespace on empty lines. Previously, `linePrefix:"//! ":true` would produce `//!` (with trailing space) on empty lines; now it produces `//!` (no trailing space), matching what formatters expect.
- Fix `pad_content_with_config` to use trimmed prefix for blank padding lines, avoiding trailing whitespace on empty comment lines in before/after padding.
- Set `keep_trailing_newline(true)` on the minijinja environment to preserve trailing newlines in rendered template content, fixing a mismatch where minijinja would strip the final newline from provider content.

### Fixes

#### Show all errors in `mdt check` instead of stopping at the first failure.

Previously, `check_project` would abort on the first template render error (e.g., invalid minijinja syntax). Now it collects all render errors alongside stale consumer entries and reports everything in a single pass.

The `CheckResult` struct has a new `render_errors` field containing `RenderError` entries. The CLI and MCP server both display these errors before the stale block list.

#### Fix release and docs-pages CI workflows.

**Release workflow:** Remove the strict version verification step that caused failures when tags were created by knope before version bumps. Add `workflow_dispatch` trigger with a `tag` input so release builds can be manually triggered for any `mdt_cli` tag. Check out the tag ref directly instead of `main` so binaries are built from the tagged commit.

**Docs-pages workflow:** Fix cancellation issue where multiple simultaneous releases caused the valid `mdt_cli` run to be cancelled by a subsequent non-matching release. Changed `cancel-in-progress` to `false` so runs queue instead of cancelling. Add `workflow_dispatch` trigger with an optional `ref` input (tag, branch, or commit SHA) for manually building and deploying docs. Check out the specified ref for both release and manual triggers.

#### Improve error display using miette for rich, contextual diagnostics.

Errors from mdt now include error codes (e.g., `mdt::unclosed_block`), actionable help text, and visual formatting with Unicode markers when color is enabled. The miette handler respects `--no-color` and the `NO_COLOR` environment variable.

Validation diagnostics (unclosed blocks, unknown transformers, unused providers) are now rendered through miette with severity levels (error vs warning) and context-specific help messages.

## 0.3.0 (2026-02-24)

### Breaking Changes

#### Add comprehensive validation diagnostics with file location reporting.

**`mdt_core` changes:**

- Add `ProjectDiagnostic` and `DiagnosticKind` types for reporting validation issues during project scanning, including unclosed blocks, unknown transformers, invalid transformer arguments, and unused providers.
- Add `ValidationOptions` struct to control which diagnostics are treated as errors vs warnings.
- Add `parse_with_diagnostics()` function that collects parse issues as diagnostics instead of hard-erroring, enabling lenient parsing for editor tooling and better error reporting.
- Add `parse_source_with_diagnostics()` for source file scanning with diagnostic collection.
- Add `line` and `column` fields to `StaleEntry` for precise location reporting in check results.
- Project scanning now collects diagnostics for all validation issues and attaches file/line/column context.

**`mdt_cli` changes:**

- Add `--ignore-unclosed-blocks` flag to suppress unclosed block errors during validation.
- Add `--ignore-unused-blocks` flag to suppress warnings about providers with no consumers.
- Add `--ignore-invalid-names` flag to suppress invalid block name errors.
- Add `--ignore-invalid-transformers` flag to suppress unknown transformer and invalid argument errors.
- Error and check output now includes `file:line:column` location information.
- JSON check output now includes `line` and `column` fields in stale entries.
- GitHub Actions annotation format now includes `line` and `col` parameters.

### Features

#### Add comprehensive CLI integration tests using `insta-cmd` snapshot testing.

19 new integration tests covering `mdt check`, `mdt update`, and `mdt update --dry-run` across multiple scenarios:

- **pad_blocks with Rust doc comments**: Verifies `//!` and `///` doc comments are not mangled after update, with check/update/idempotency/diff snapshots.
- **pad_blocks with multiple languages**: Tests Rust, TypeScript (JSDoc), Python, and Go source files with data interpolation from `package.json`, ensuring all comment styles are preserved correctly.
- **Validation diagnostics**: Snapshots error output for unclosed blocks and verifies `--ignore-unclosed-blocks` bypasses the error.
- **includeEmpty on linePrefix**: Verifies the difference between `linePrefix` with and without `includeEmpty:true` — blank lines get the prefix when enabled.
- **TypeScript workspace**: Adds snapshot coverage for the existing fixture, including file content verification after update.

Also adds extra blank line padding in `pad_blocks` mode: when a comment prefix is present (e.g., `//!`, `///`, `*`), an additional blank line using that prefix is inserted between the opening tag and the content, and between the content and the closing tag.

Sorts file paths in `mdt update --dry-run` and `--verbose` output for deterministic ordering.

### Fixes

#### Add comprehensive CLI integration tests covering all commands and features.

**New tests (28 added, 47 total):**

- `mdt init` — fresh directory creation and existing template detection
- `mdt list` — block listing with provider/consumer counts, empty project, verbose output
- `mdt check --format json` — JSON output for stale and up-to-date states
- `mdt check --format github` — GitHub Actions annotation format
- `mdt check --diff` — unified diff output for stale blocks
- `mdt update --verbose` — verbose output with provider listing and updated file paths
- `--ignore-unused-blocks` — suppresses unused provider diagnostics
- `--ignore-invalid-transformers` — suppresses unknown transformer errors
- `--ignore-unclosed-blocks` — suppresses unclosed block errors
- Missing provider warnings — consumers referencing non-existent providers
- Multiple providers — multiple blocks consumed across multiple files
- Empty project — no providers or consumers
- No subcommand — error message when running `mdt` without a command

**Bug fixes:**

- Sort provider names in verbose output for deterministic ordering

**Snapshot stability:**

- Add path redaction (`[TEMP_DIR]`) to all snapshots containing absolute paths, ensuring reproducibility across machines
- Enable `insta` `filters` feature for regex-based path filtering

#### Increase test coverage across all crates.

**`mdt_core`:** Added 47 new tests covering config file parsing (TOML integers, floats, arrays, tables, datetime; KDL empty nodes, named entries, mixed entries, children, integer/float/bool/null values), project scanning (`scan_project_with_config` with data and pad_blocks, template directories, include patterns, CRLF normalization), diagnostic conversion (unclosed blocks, unknown transformers, invalid transformer args), validation options, error display formatting, and edge cases for `is_template_file`, `find_missing_providers`, and `validate_project`.

**`mdt_lsp`:** Added 28 new tests covering `WorkspaceState::rescan_project` (valid project, invalid config, data loading), `update_document_in_project` (non-file URI, non-template providers), template rendering failure paths for diagnostics/hover/code actions, stale consumers with transformers, provider hover with zero consumers, document symbols for both provider and consumer blocks, completion with multiple providers, `suggest_similar_names` edge cases, and `levenshtein_distance` edge cases.

**`mdt_mcp`:** Added 44 new tests (from 0) covering all MCP server tool methods (`check`, `update`, `list`, `get_block`, `preview`, `init`), helper functions (`resolve_root`, `make_relative`, `scan_ctx`), `get_info` server handler, `Default` trait, and various scenarios including stale consumers, dry-run mode, missing providers, data interpolation, and template file existence checks.

**`mdt_cli`:** Added 13 new snapshot tests covering orphan consumer display in `list` and `check`, verbose output for stale checks with diffs, verbose dry-run updates, verbose update with file listing, and verbose diagnostic warnings with ignore flags for both unclosed blocks and unknown transformers.

### Documentation

#### Add comprehensive doc comments to `mdt_core` public API types and enrich CLI help text for all `mdt_cli` commands.

**mdt_core:**

- Expand crate-level documentation with processing pipeline diagram, module overview, key types reference, data interpolation guide, and quick start code example.
- Add struct/enum-level doc comments for `Block`, `Transformer`, and `Argument` explaining their role in the template system.
- Add field-level doc comments for `Block`, `Transformer`, `Argument`, and `StaleEntry` fields.
- Add doc comments for internal types `TokenGroup`, `DynamicRange`, and `GetDynamicRange` in the tokens module.
- Add provider blocks to `template.t.md` for `mdtCoreOverview`, `mdtBlockDocs`, `mdtTransformerDocs`, and `mdtArgumentDocs` for potential use by markdown consumers.

**mdt_cli:**

- Expand `Init` help with details about what file is created and no-op behavior.
- Expand `Check` help with CI usage guidance, `--diff` and `--format` tips.
- Expand `Update` help with template rendering flow, `--dry-run` and `--watch` details.
- Expand `List` help with output format description.
- Expand `Lsp` help with diagnostics and auto-completion features.
- Expand `Mcp` help with available tools description.
- Enrich `OutputFormat` variant docs and field-level docs for `Check` and `Update` args.

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

## 0.0.2 (2026-02-24)

### Fixes

- Add `cargo-deny` for automated security auditing, license compliance checking, and dependency ban enforcement. Integrates with CI via `EmbarkStudios/cargo-deny-action` and adds `deny:check` to the local `lint:all` workflow.
- Fix docs-pages workflow by enabling automatic GitHub Pages configuration. The `enablement: true` flag on `actions/configure-pages@v5` auto-enables Pages via the GitHub API, resolving the "Get Pages site failed" error.
- Fix release workflow to checkout `main` branch instead of tag ref, and add version verification step to prevent publishing mismatches. Also add `cargo check --workspace` to the knope release workflow to catch build errors before creating tags.
- Update workspace dependencies: replace archived `serde_yml` with maintained `serde_yaml_ng` fork, replace unmaintained `tower-lsp` with community fork `tower-lsp-server`, and update `cargo-nextest` to 0.9.129. Run `cargo update` for latest compatible versions of all dependencies.

## 0.0.1 (2026-02-20)

### Features

#### Comprehensive API completeness improvements across all crates.

**New transformers:** Added `suffix`, `linePrefix`, and `lineSuffix` transformers for more granular content manipulation. `linePrefix` and `lineSuffix` apply prefixes/suffixes to each non-empty line, while `suffix` appends to the entire content. All support both camelCase and snake_case names.

**Duplicate provider detection:** The project scanner now detects and reports duplicate provider block names across template files, with clear error messages indicating both file locations.

**Rich diagnostics:** All error types now include `#[help(...)]` attributes with actionable guidance. Added `UnknownTransformer` and `InvalidTransformerArgs` error variants for better error reporting.

**Transformer validation:** New `validate_transformers()` function checks argument counts against expected ranges for each transformer type.

**Block PartialEq:** `Block`, `Transformer`, and `Argument` types now derive `PartialEq` for easier testing and comparison. Introduced `OrderedFloat` wrapper for approximate float equality.

**CLI enhancements:**

- `mdt check --diff` shows a colorized unified diff of stale consumer blocks
- `mdt check --format json|github|text` supports machine-readable output formats including GitHub Actions annotations
- `mdt list` command displays all providers and consumers with relationship status and transformer info
- `mdt update --watch` watches for file changes and re-runs updates with 200ms debouncing
- Colored terminal output with `NO_COLOR` env var and `--no-color` flag support

**Config improvements:** Added `[include]` patterns and `[templates]` paths sections to `mdt.toml` for controlling which files are scanned and where template files are located.

**LSP incremental updates:** The language server now performs incremental document updates on save instead of full project rescans, with full rescans only when `mdt.toml` changes.

#### Add config file support (`mdt.toml`), minijinja template rendering, and source file scanning.

**Config file (`mdt.toml`):** Map data files to namespaces under `[data]`. Supports JSON, TOML, YAML, and KDL data sources.

**Template variables:** Provider blocks can use `{{ namespace.key }}` syntax (powered by minijinja) to interpolate data from configured files.

**Source file scanning:** Consumer blocks are now detected in source code comments (`.ts`, `.rs`, `.py`, `.go`, `.java`, etc.), not just markdown files.

#### mdt

Implement the core template management engine:

- **Parser**: Complete the `parse()` function that converts markdown content into structured `Block` types (provider and consumer) by wiring the lexer output through pattern matching into block construction. Extracts block names, types, and transformer/filter chains from token groups.
- **Project scanner** (`project` module): Walk a directory tree to discover `*.t.md` template definition files (providers) and other markdown files containing consumer blocks. Builds a map of provider name to content and collects all consumer entries with their file paths.
- **Content replacement engine** (`engine` module): Implement `check_project()` to verify all consumer blocks are up to date with their providers, and `compute_updates()` / `write_updates()` to replace stale consumer content. Supports all transformer types: `trim`, `trimStart`, `trimEnd`, `indent`, `prefix`, `wrap`, `codeBlock`, `code`, and `replace`.
- **New `Prefix` transformer type**: Added to support prefixing content with a string.
- **New error variants**: `MissingProvider` and `StaleConsumer` for better diagnostics.
- **Removed debug `println!`** from the lexer that was accidentally left in.

#### mdt_cli

Implement all three CLI commands with real functionality:

- **`mdt init`**: Creates a sample `template.t.md` file with a provider block and prints getting-started instructions. Skips if the file already exists.
- **`mdt check`**: Scans the project for provider and consumer blocks, verifies all consumers are up to date. Exits with non-zero status and prints diagnostics if any blocks are stale.
- **`mdt update`**: Scans the project and replaces stale consumer content with the latest provider content, applying any configured transformers. Supports `--dry-run` to preview changes without writing files.
- **Global options**: `--path` to specify the project root, `--verbose` for detailed output.

#### Implement the mdt language server (LSP) and add `mdt lsp` CLI subcommand.

**LSP features:**

- **Diagnostics:** Warns about stale consumer blocks (content out of date), missing providers (consumer references non-existent provider), and provider blocks in non-template files.
- **Hover:** Shows provider content preview when hovering over consumer tags. Shows consumer count and locations when hovering over provider tags. Displays transformer chain and source file path.
- **Completion:** Suggests block names when typing inside `{=`, `{@`, or `{/` tags. Suggests transformer names (`trim`, `indent`, `codeBlock`, etc.) after pipe `|` characters with usage descriptions.
- **Go to Definition:** Navigates from consumer tag to its provider definition in the template file. From provider tags, navigates to all consumer locations.
- **Document Symbols:** Lists all provider (`@name`) and consumer (`=name`) blocks in the editor outline/breadcrumb.
- **Code Actions:** Offers "Update block" quick-fix for stale consumer blocks that replaces content with the latest provider output.

**CLI integration:** Run `mdt lsp` to start the language server over stdin/stdout. Configure your editor's LSP client to use this command.

**Editor setup examples:**

- **VS Code** (with a generic LSP extension): Set the server command to `mdt lsp`
- **Neovim** (with nvim-lspconfig): `require('lspconfig').mdt.setup({ cmd = { 'mdt', 'lsp' } })`
- **Helix**: Add `[language-server.mdt] command = "mdt" args = ["lsp"]` to `languages.toml`

#### Add MCP server, security hardening, and quality improvements.

**MCP server (`mdt_mcp`):** New crate providing a Model Context Protocol server with 6 tools: `mdt_check`, `mdt_update`, `mdt_list`, `mdt_get_block`, `mdt_preview`, and `mdt_init`. The server communicates over stdin/stdout and can be launched via `mdt mcp`. Built on the `rmcp` SDK.

**Security hardening:**

- Add configurable file size limits (`max_file_size` in `mdt.toml`, default 10MB) to prevent denial-of-service from large files
- Add symlink cycle detection during directory walking to prevent infinite loops
- Normalize CRLF line endings (`\r\n` and `\r`) to LF (`\n`) on read for consistent cross-platform behavior
- Return proper errors for unconvertible float values (NaN, Infinity) in TOML and KDL data files instead of silently defaulting to 0

**Performance:**

- Optimize `offset_to_point` in the source scanner with a `LineTable` struct using binary search (O(log n) instead of O(n) per lookup)

**Quality:**

- Migrate LSP `DocumentSymbol` responses from deprecated `SymbolInformation` (flat) to hierarchical `DocumentSymbol` API
- Fix CLI exit codes: exit 1 for stale blocks (expected failure), exit 2 for actual errors

**Toolchain:**

- Bump `rust-toolchain.toml` from 1.87.0 to 1.88.0 (required by `rmcp` dependency)

### Fixes

#### Improve API surface and update all dependencies to latest versions.

**API improvements:**

- Add `ProjectContext` struct to bundle `Project` and data together, replacing loose `(Project, HashMap)` tuple passing through the engine API.
- Add `Display` implementations for `TransformerType` and `BlockType`.
- Reduce public API surface: `lexer`, `tokens`, and `patterns` modules are now `pub(crate)` — internal implementation details are no longer leaked.
- Remove dead code: `Blocks` newtype wrapper, unused `mdt_lsp::error` module, unused `memchr`/`optional`/`optional_group`/`get_bounds_index` functions, and `doc-comment` dependency from `mdt` crate.

**CLI improvements:**

- Consolidate scan + verbose output + missing provider warnings into shared `scan_and_warn()` helper, reducing code duplication between `check` and `update` commands.

**Dependency updates:**

- Bump `float-cmp` 0.9 → 0.10, `rstest` 0.25 → 0.26, `toml` 0.8 → 1.0.
- Remove unused workspace dependencies: `logos`, `readonly`, `typed-builder`, `vfs`.
- Update cargo bin versions: `cargo-insta` 1.46.3, `cargo-llvm-cov` 0.8.4, `cargo-nextest` 0.9.127, `cargo-semver-checks` 0.46.0, `knope` 0.22.3.

#### Improve CLI output, performance, error handling, and test coverage.

**CLI improvements:** Verbose mode now shows provider details including names and file paths. Both `check` and `update` commands now warn about consumer blocks referencing non-existent providers. Improved help text with description and quick-start examples.

**Performance:** Optimized `Point::advance` with a new `advance_str` method that avoids allocating via `Display::to_string()` on the hot path. The lexer now uses `advance_start_str` for string-based position tracking. Engine `compute_updates` uses pre-allocated `String::with_capacity` instead of `format!` for content replacement, and avoids unnecessary equality comparison by tracking updates with a boolean flag.

**Exclude patterns:** Added `[exclude]` section to `mdt.toml` configuration with glob pattern support for skipping directories or files during scanning. Uses the `globset` crate for pattern matching.

**Error handling:** Replaced a `panic!` in the lexer with a graceful `break` when the context stack is unexpectedly empty. Added missing provider warnings to CLI output.

**Test coverage:** More than doubled the test count from 57 to 133 tests. New tests cover: all transformer types (trim, indent, prefix, wrap, codeBlock, code, replace), transformer chaining, edge cases (empty content, unicode, numeric arguments), engine operations (check, compute_updates, write_updates, idempotency, multiple consumers per file, missing providers), project scanning (hidden dirs, node_modules, exclude patterns, sub-project boundaries, source files), config loading (all formats, multiple namespaces, missing files, exclude patterns, `.yml` extension), template rendering (undefined variables, arrays, conditionals), source scanner (multiple blocks, Python comments, position tracking), error messages, and CLI integration tests (verbose output, warnings, multi-block updates, surrounding content preservation, data interpolation).

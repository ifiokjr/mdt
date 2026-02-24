# Changelog

This file is maintained by `knope`.

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

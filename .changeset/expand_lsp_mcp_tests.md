---
mdt_lsp: patch
mdt_mcp: patch
---

Expand test coverage for LSP and MCP server crates with tests for error handling, edge cases, and tool functionality.

### `mdt_lsp`

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

### `mdt_mcp`

- Test error cases: invalid mdt.toml config for check, update, list, get_block, and preview tools.
- Test edge cases: empty projects, providers-only projects, consumers-only projects.
- Test `list` tool: consumer transformers, summary format, trimmed provider content, relative file paths.
- Test `update` tool: dry-run reports counts, empty project no-op, synced project dry-run, idempotent updates, multi-file updates.
- Test `get_block` tool: provider with no consumers, raw vs rendered content with data interpolation, multiple consumer files.
- Test `preview` tool: data interpolation, transformer display, multiple consumers.
- Test `check` tool: stale block names and file paths in output, multiple stale blocks, missing data files.
- Test `init` tool: nested directories.
- Test `scan_ctx` graceful handling of nonexistent paths.

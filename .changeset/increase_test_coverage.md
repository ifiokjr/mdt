---
mdt: patch
mdt_cli: patch
---

Increase test coverage across all crates.

**`mdt_core`:** Added 47 new tests covering config file parsing (TOML integers, floats, arrays, tables, datetime; KDL empty nodes, named entries, mixed entries, children, integer/float/bool/null values), project scanning (`scan_project_with_config` with data and pad_blocks, template directories, include patterns, CRLF normalization), diagnostic conversion (unclosed blocks, unknown transformers, invalid transformer args), validation options, error display formatting, and edge cases for `is_template_file`, `find_missing_providers`, and `validate_project`.

**`mdt_lsp`:** Added 28 new tests covering `WorkspaceState::rescan_project` (valid project, invalid config, data loading), `update_document_in_project` (non-file URI, non-template providers), template rendering failure paths for diagnostics/hover/code actions, stale consumers with transformers, provider hover with zero consumers, document symbols for both provider and consumer blocks, completion with multiple providers, `suggest_similar_names` edge cases, and `levenshtein_distance` edge cases.

**`mdt_mcp`:** Added 44 new tests (from 0) covering all MCP server tool methods (`check`, `update`, `list`, `get_block`, `preview`, `init`), helper functions (`resolve_root`, `make_relative`, `scan_ctx`), `get_info` server handler, `Default` trait, and various scenarios including stale consumers, dry-run mode, missing providers, data interpolation, and template file existence checks.

**`mdt_cli`:** Added 13 new snapshot tests covering orphan consumer display in `list` and `check`, verbose output for stale checks with diffs, verbose dry-run updates, verbose update with file listing, and verbose diagnostic warnings with ignore flags for both unclosed blocks and unknown transformers.

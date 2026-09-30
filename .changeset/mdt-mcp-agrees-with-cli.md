---
"mdt_mcp": change
---

# MCP tools now agree with the CLI and report every failure as a structured result

The MCP tools re-implemented parts of `mdt check`, `mdt update`, and `mdt init`, and drifted from them. They now call the same `mdt_core` functions, so an agent sees exactly what the CLI would report.

- **Staleness matches `mdt check`.** `mdt_list`, `mdt_get_block`, and `mdt_preview` take each block's status from `check_project`, so `[padding]`, lenient comparison, and `[[formatters]]` are honoured. Consumer entries gain `type` (`consumer` or `inline`), `line`, `column`, `arguments`, and `status` (`current`, `stale`, `render_error`, `orphan`); inline blocks get a status too.
- **Validation is no longer skipped.** `mdt_check`, `mdt_update`, and `mdt_list` return `diagnostics` (`kind`, `severity`, `file`, `line`, `column`, `message`), and `ok` is false when any is an error. `mdt_update` refuses to write while validation errors exist, like the CLI. All three accept `ignore_unclosed_blocks`, `ignore_unused_blocks`, `ignore_invalid_names`, and `ignore_invalid_transformers`, mirroring the `--ignore-*` flags.
- **Orphan consumers fail `mdt_check`** and are listed in `orphans` with suggested provider names.
- **Render errors are reported, not hidden.** `mdt_update` returns `render_errors` and syncs everything else instead of failing the request; `mdt_get_block` and `mdt_preview` report `render_error` with `ok: false` instead of falling back to the raw template.
- **`mdt_init` is `mdt init`.** It calls `mdt_core::init::init_project` and returns the `config`, `sample`, and `gitignore` outcomes, `written_files` relative to the initialized root, and `next_steps` with correctly written consumer tags.
- **Tool-level failures are `isError` results**, not JSON-RPC errors that many clients hide from the model: an invalid `mdt.toml`, a missing data file, duplicate providers, a failing formatter, or a bad `path` return `{ ok: false, action, summary, error: { code, message, help? } }` with the diagnostic code (for example `mdt::config_parse`). A `path` that does not exist or is not a directory is now an error instead of an empty, passing project.
- **Consistent responses.** Every tool returns `ok`, `action`, and `summary`. `mdt_get_block` returns one object shape, `{ ok, action, summary, block_name, provider, consumers }`, instead of a bare array for consumer lookups.
- **Smaller `mdt_list`.** Provider bodies are omitted unless `include_content` is true.
- **Better reuse search.** `mdt_find_reuse` ranks exact names, then names equal up to case and separators, prefixes, substrings, and close spellings, leaves unrelated providers out, reports the `match` kind, and accepts a `content_query` to search provider bodies. The `limit` schema now declares its 1–20 range.
- **Protocol.** The server identifies as `mdt` with the crate version, read-only tools carry `readOnlyHint`, tool descriptions say what each returns, and the instructions point agents at `mdt skill`.
- **Startup.** `run_server_in(root)` serves a chosen directory (`run_server()` still serves the current one), and `MDT_LOG=info mdt mcp` no longer panics when the CLI has already installed a tracing subscriber.

Tool handlers now return `CallToolResult` directly, and `PathParam` is replaced by `CheckParam` and `ListParam`.
- Diagnostics include the same `code` as `mdt check --format json` (for example `mdt::unclosed_block`), and `warnings` entries report `template_rendered`.

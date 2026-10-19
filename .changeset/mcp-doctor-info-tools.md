---
"mdt_core": minor
"mdt_cli": minor
"mdt_mcp": minor
"@m-d-t/skills": docs
---

# Add mdt_doctor and mdt_info MCP tools

The MCP server can now self-diagnose, exactly like the CLI. The doctor and info report building moved from `mdt_cli` into `mdt_core`, so both surfaces share one implementation and the CLI output is unchanged:

- `mdt_core::doctor` runs the same health checks as `mdt doctor` (config discovery and parsing, data sources, template layout, duplicate providers, orphan consumers, unused providers, parser diagnostics, consumer sync, cache artifact, hash mode, and efficiency) and returns a serializable `DoctorReport`.
- `mdt_core::info` builds the same report as `mdt info` (`InfoReport`) with block counts, data namespaces, template directories and files, diagnostic totals, and cache state.
- New `mdt_doctor` tool: read-only health checks with `counts` (`pass`/`warn`/`fail`/`skip`) and `checks` (each with `id`, `status`, `message`, and a fix `hint`); accepts the `ignore_*` options like `mdt_check`. A broken config is reported as a failing check, not a tool error.
- New `mdt_info` tool: read-only project inspection with the report sections (`project`, `blocks`, `data`, `templates`, `diagnostics`, `cache`) flattened at the top level.

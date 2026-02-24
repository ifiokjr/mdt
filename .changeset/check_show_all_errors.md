---
mdt: patch
mdt_cli: patch
---

Show all errors in `mdt check` instead of stopping at the first failure.

Previously, `check_project` would abort on the first template render error (e.g., invalid minijinja syntax). Now it collects all render errors alongside stale consumer entries and reports everything in a single pass.

The `CheckResult` struct has a new `render_errors` field containing `RenderError` entries. The CLI and MCP server both display these errors before the stale block list.

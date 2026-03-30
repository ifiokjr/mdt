---
mdt_mcp: minor
---

Make the MCP server more agent-friendly with structured JSON-first responses.

`mdt_check`, `mdt_update`, `mdt_preview`, and `mdt_init` now return consistent JSON-oriented envelopes in MCP `structured_content`, while still preserving text content for compatibility. `mdt_preview` now acts as an authoring workflow by returning per-consumer rendered output, including parameterized consumer previews and render/mismatch details. The MCP surface also exposes undefined-variable warnings in `check` and `update` results.

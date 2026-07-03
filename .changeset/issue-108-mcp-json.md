---
mdt_mcp: minor
---

# Return structured JSON envelopes from MCP tools

MCP tools now return more consistent JSON-oriented `structured_content` envelopes for `mdt_check`, `mdt_update`, `mdt_preview`, and `mdt_init`. Text content is still preserved for clients that display plain tool messages.

`mdt_preview` now behaves more like an authoring workflow by returning per-consumer rendered output, parameterized previews, render details, and mismatch information. Check and update responses also surface undefined-variable warnings so agents can reason about template problems without scraping text.

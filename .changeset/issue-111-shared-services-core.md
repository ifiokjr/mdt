---
mdt_core: patch
---

# Extract shared app-surface helpers into core

`mdt_core` now exposes shared helpers for project-root resolution, relative path display, and similar-name scoring. These utilities centralize behavior that was previously reimplemented by multiple application surfaces.

The extraction keeps user-facing behavior stable while making CLI, LSP, and MCP code easier to maintain consistently.

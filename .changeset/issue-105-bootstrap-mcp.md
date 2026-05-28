---
mdt_mcp: patch
---

Unify MCP initialization with the CLI bootstrap flow.

The MCP init flow now respects existing canonical and legacy template locations instead of always writing a new root-level `template.t.md`. Documentation and generated READMEs now consistently describe `.templates/template.t.md` as the canonical starter path.

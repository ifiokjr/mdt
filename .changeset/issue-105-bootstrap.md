---
mdt_cli: docs
mdt_mcp: patch
---

Align bootstrap documentation around the canonical `.templates/` layout and unify MCP initialization with the CLI.

`mdt_init` now creates the same starter files as `mdt init`: a sample `.templates/template.t.md` file plus `mdt.toml` when no config exists yet. The MCP init flow also respects existing canonical and legacy template locations instead of always writing a new root-level `template.t.md`.

Documentation and generated READMEs now consistently describe `.templates/template.t.md` as the canonical starter path.

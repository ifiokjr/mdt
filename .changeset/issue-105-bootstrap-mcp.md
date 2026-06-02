---
mdt_mcp: patch
---

# Respect existing template locations during MCP init

The MCP initialization flow now follows the same bootstrap rules as the CLI. It detects existing canonical and legacy template locations before writing starter content, avoiding unnecessary root-level `template.t.md` files in projects that already have a usable template directory.

Generated READMEs and initialization guidance now consistently describe `.templates/template.t.md` as the preferred starter path.

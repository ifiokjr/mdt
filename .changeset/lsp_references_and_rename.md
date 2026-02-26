---
mdt_lsp: minor
---

Add `references` and `rename` support to the LSP server. `textDocument/references` returns all provider and consumer blocks sharing the same name. `textDocument/rename` renames a block name across all provider and consumer tags (both opening and closing tags) in the workspace.

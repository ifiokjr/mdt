---
mdt_cli: minor
---

Implement the mdt language server (LSP) and add `mdt lsp` CLI subcommand.

**LSP features:**

- **Diagnostics:** Warns about stale consumer blocks (content out of date), missing providers (consumer references non-existent provider), and provider blocks in non-template files.
- **Hover:** Shows provider content preview when hovering over consumer tags. Shows consumer count and locations when hovering over provider tags. Displays transformer chain and source file path.
- **Completion:** Suggests block names when typing inside `{=`, `{@`, or `{/` tags. Suggests transformer names (`trim`, `indent`, `codeBlock`, etc.) after pipe `|` characters with usage descriptions.
- **Go to Definition:** Navigates from consumer tag to its provider definition in the template file. From provider tags, navigates to all consumer locations.
- **Document Symbols:** Lists all provider (`@name`) and consumer (`=name`) blocks in the editor outline/breadcrumb.
- **Code Actions:** Offers "Update block" quick-fix for stale consumer blocks that replaces content with the latest provider output.

**CLI integration:** Run `mdt lsp` to start the language server over stdin/stdout. Configure your editor's LSP client to use this command.

**Editor setup examples:**

- **VS Code** (with a generic LSP extension): Set the server command to `mdt lsp`
- **Neovim** (with nvim-lspconfig): `require('lspconfig').mdt.setup({ cmd = { 'mdt', 'lsp' } })`
- **Helix**: Add `[language-server.mdt] command = "mdt" args = ["lsp"]` to `languages.toml`

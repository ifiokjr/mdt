# Editor Setup

mdt ships a language server, `mdt lsp`, that brings template diagnostics into your editor: stale consumers, orphan blocks, unclosed tags, provider render failures, plus completions, hover, go to definition, references, and rename. It speaks standard LSP over stdin/stdout and uses the editor's workspace folder as the project root.

The quick fix rewrites a stale consumer exactly as `mdt update` would. The server does not run `[[formatters]]`, so files owned by a formatter show no stale blocks in the editor; run `mdt check` or `mdt update` for those.

## Neovim

Neovim 0.11 and later configure the server with `vim.lsp.config`. Add this to your config:

```lua
vim.lsp.config('mdt', {
  cmd = { 'mdt', 'lsp' },
  filetypes = { 'markdown' },
  root_markers = { 'mdt.toml', '.mdt.toml', '.git' },
})

vim.lsp.enable('mdt')
```

## Helix

Add the server to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "markdown"
language-servers = ["mdt"]

[language-server.mdt]
command = "mdt"
args = ["lsp"]
```

## Emacs

With `lsp-mode` and `markdown-mode`:

```elisp
(with-eval-after-load 'lsp-mode
  (add-to-list 'lsp-language-id-configuration '(markdown-mode . "mdt"))
  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection '("mdt" "lsp"))
    :activation-fn (lsp-activate-on "markdown")
    :server-id 'mdt)))
```

## Other editors

VS Code and Zed attach language servers through extensions rather than user configuration, so they need an extension that runs an arbitrary server. Any generic LSP extension works once it is told to spawn `mdt lsp` with no arguments for `markdown` files (including `*.t.md` templates). Until one is installed, `mdt check --watch` in a terminal gives the same diagnostics with automatic re-runs.

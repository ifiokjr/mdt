# VS Code extension scope

Resolves the long-standing placeholder issue: [#1 — create a placeholder `mdt` vscode extension](https://github.com/ifiokjr/mdt/issues/1).

## Why this is the biggest adoption lever

mdt currently gives feedback in two places: the CLI (`mdt check`) and CI. Both are _after the fact_ — you find out a block is stale after committing, pushing, and waiting for CI. The editor is where drift should be _impossible to miss_:

- a stale block shows a squiggle the moment the template changes
- hovering a block shows the rendered template without leaving the file
- a quick-fix syncs the block in place, without switching to a terminal

The critical asset already exists: `mdt lsp` implements diagnostics, completions, hover, go-to-definition, references, rename, symbols, and code actions over stdin/stdout. **The extension is a thin client over an existing server, not a new feature surface.** Most of the work is packaging, not protocol.

## Architecture

```
┌────────────────────┐   stdio   ┌──────────────┐   ┌───────────┐
│  VS Code extension │ ────────▶ │  mdt lsp     │──▶│ workspace │
│  (TypeScript)      │ ◀──────── │ (tower-lsp)  │   │  files    │
└────────────────────┘           └──────────────┘   └───────────┘
```

- Extension shell: TypeScript, packaged with `vsce`, activates on markdown, `.t.md`, and supported source-file languages.
- Server resolution order: workspace `node_modules/.bin/mdt` → bundled platform binary (reuse the `@m-d-t/cli-*` optional-dependency pattern from the npm CLI packages) → `mdt` on `PATH` → helpful error with install hint.
- Client library: `vscode-languageclient`, started with `{ run: ..., debug: ... }` stdio server options.
- A small "mdt" output channel surfaces `mdt lsp` stderr (the server logs via `tracing` with env-filter already).

## Phases

### Phase 0 — scaffold (placeholder) · ~half a day

- Extension repo/scaffold in `editors/vscode/` (workspace member directory, not a cargo crate)
- Activation events, status-bar item showing server state
- Manual server resolution + connection to `mdt lsp`
- Publish to Marketplace + Open VSX as `mdt` (id `ifiokjr.mdt`), versioned by monochange as a new npm package
- _Exit criteria:_ extension installs, connects to the server, output channel shows capabilities

### Phase 1 — make drift visible · ~2–3 days

- Diagnostics: stale blocks (error), unused sources (info), unclosed/invalid blocks (error) — all already emitted by `mdt lsp`
- Hover: provider source + rendered preview + transformer chain
- Completions: block names after `{=`, `{~`, `{@`, `{/`, transformer names after `|`
- _This phase alone delivers the core value: drift becomes visible at save time._

### Phase 2 — act on it · ~2–3 days

- Code action quick-fix (already in `mdt lsp`) wired to the client
- Commands: `mdt: Update All` (runs `mdt update` as a task), `mdt: Check` (runs `mdt check`, jumps to first stale block)
- Task provider so `mdt check` can run as a CI-parity task inside the editor
- Rename + go-to-def/references (server already implements them; mostly client plumbing)

### Phase 3 — polish (only after feedback)

- Sync-on-save toggle
- Inline preview decoration for source blocks in `.t.md` files (ghost text showing what consumers will render)
- Snippets for `{@}`, `{=}`, `{~}` block scaffolding
- Settings: server path override, diagnostic severity mapping

## Distribution & versioning

- Marketplace id: `ifiokjr.mdt`; also publish to Open VSX (free, covers VS Code forks like Cursor/VSCodium)
- Versioning: add to the monochange `mdt` group as an npm package so releases ride the existing pipeline; the `release-test` job already exercises release commits
- Platform binaries: reuse the existing `@m-d-t/cli-darwin-arm64`-style optional dependencies so the extension ships without requiring a global mdt install

## What we are deliberately not doing

- No custom UI panels or webviews — the LSP surface covers the value; the editor chrome is enough
- No reimplementation of block parsing in TypeScript — the server is the single source of truth, which keeps Rust and editor behavior identical
- No GitHub Actions / non-VS Code editor support in the first iteration (Neovim/others get mdt free via `mdt lsp` + any LSP client)

## Open questions

1. Marketplace publisher account (one-time setup, requires Azure DevOps org)
2. Should `mdt lsp` gain a `--workspace-root` flag for multi-root workspaces? (tower-lsp supports it; needs verification)
3. Icon — a 128×128 PNG derived from `assets/mdt-icon.svg`

---
"mdt_core": minor
"mdt_lsp": minor
---

# Remove dead shims ahead of 1.0

- `MdtError::SymlinkCycle` is gone. It was documented as "no longer returned" — scanning visits each directory once, so symlink aliases and cycles are skipped — and nothing constructed it.
- The `From<MdtError> for std::io::Error` impl ("for test compatibility") is removed; the one CLI test that relied on it now panics with the parse error instead.
- The LSP server no longer falls back to the deprecated `root_uri` in `initialize`. Clients that send only `root_uri` (no `workspace_folders`) get working-directory root discovery instead.
- The docs workspace crate is renamed `monochange_book` → `mdt_book` (scaffolding inherited from an unrelated project), and the `docs/agents/README.md` structure stub is deleted — `AGENTS.md` already links the real pages.

Not removed, deliberately: `LEGACY_TEMPLATE_PATHS` in `init`. Testing showed it is a robustness guard, not a compat shim — the existence check runs before any scan, so `mdt init` still succeeds on half-configured projects (invalid `mdt.toml`, or a `template.t.md` without parseable tags) instead of failing or writing a duplicate sample.

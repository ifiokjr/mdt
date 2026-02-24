---
mdt_core: major
mdt_cli: major
mdt_lsp: major
mdt_mcp: major
---

Rename the core library crate from `mdt` to `mdt_core` to resolve a crate name conflict on crates.io. The `mdt` name on crates.io is owned by a different project ("a markdown tool for writers").

This is a breaking change for any downstream code that depends on the `mdt` crate directly. All `use mdt::` imports must be updated to `use mdt_core::`.

The CLI binary name remains `mdt` (unchanged). The `mdt_cli`, `mdt_lsp`, and `mdt_mcp` crate names are also unchanged.

### Migration

Replace all occurrences of `use mdt::` with `use mdt_core::` in your code, and update your `Cargo.toml` dependency from `mdt` to `mdt_core`.

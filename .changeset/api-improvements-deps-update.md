---
mdt: minor
mdt_cli: patch
---

Improve API surface and update all dependencies to latest versions.

**API improvements:**

- Add `ProjectContext` struct to bundle `Project` and data together, replacing loose `(Project, HashMap)` tuple passing through the engine API.
- Add `Display` implementations for `TransformerType` and `BlockType`.
- Reduce public API surface: `lexer`, `tokens`, and `patterns` modules are now `pub(crate)` — internal implementation details are no longer leaked.
- Remove dead code: `Blocks` newtype wrapper, unused `mdt_lsp::error` module, unused `memchr`/`optional`/`optional_group`/`get_bounds_index` functions, and `doc-comment` dependency from `mdt` crate.

**CLI improvements:**

- Consolidate scan + verbose output + missing provider warnings into shared `scan_and_warn()` helper, reducing code duplication between `check` and `update` commands.

**Dependency updates:**

- Bump `float-cmp` 0.9 → 0.10, `rstest` 0.25 → 0.26, `toml` 0.8 → 1.0.
- Remove unused workspace dependencies: `logos`, `readonly`, `typed-builder`, `vfs`.
- Update cargo bin versions: `cargo-insta` 1.46.3, `cargo-llvm-cov` 0.8.4, `cargo-nextest` 0.9.127, `cargo-semver-checks` 0.46.0, `knope` 0.22.3.

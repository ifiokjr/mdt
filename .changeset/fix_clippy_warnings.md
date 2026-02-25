---
mdt_core: patch
mdt_cli: patch
mdt_lsp: patch
mdt_mcp: patch
---

Fix clippy warnings across the workspace.

- Replace `map().unwrap_or()` with `map_or()` in `engine.rs`.
- Suppress `too_many_arguments` on `scan_project_with_options` (to be refactored separately).
- Suppress `only_used_in_recursion` on `walk_dir`'s `root` parameter.
- Suppress `variant_size_differences` on `PaddingValue` enum.
- Suppress `unused_assignments` from thiserror-generated code in `MdtError`.
- Suppress `struct_excessive_bools` on `MdtCli`.
- Fix redundant closures in `mdt_lsp` (`map(|p| p.into_owned())` to `map(Cow::into_owned)`).
- Suppress deprecated `root_uri` field usage in LSP (separate migration PR).
- Suppress `disallowed_methods` false positives from `tokio::test` macro in `mdt_mcp` tests.
- Fix `cmp_owned` warning in `mdt_mcp` tests.
- Fix unnecessary qualifications, single-char string patterns, doc comment backticks, `approx_constant` errors, and `float_cmp` warnings in `mdt_core` tests.

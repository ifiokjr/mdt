---
mdt_core:
  bump: none
  type: refactor
mdt_cli:
  bump: none
  type: refactor
mdt_lsp:
  bump: none
  type: refactor
mdt_mcp:
  bump: none
  type: refactor
---

# Apply clippy fixes:

- Rename `matcher` to `compiled` in config.rs to avoid similar_names warning
- Use `std::io::Error::other()` instead of `Error::new()` in error.rs
- Rename shadowing variable `updated` to `updated_file` in __tests.rs
- Allow unnecessary_wraps on internal helper function in engine.rs

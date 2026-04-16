---
apply best practices to codebase (Rust) > lint/clippy fixes: resolve clippy warnings for variable naming and error handling
---

Apply clippy fixes:

- Rename `matcher` to `compiled` in config.rs to avoid similar_names warning
- Use `std::io::Error::other()` instead of `Error::new()` in error.rs
- Rename shadowing variable `updated` to `updated_file` in __tests.rs
- Allow unnecessary_wraps on internal helper function in engine.rs

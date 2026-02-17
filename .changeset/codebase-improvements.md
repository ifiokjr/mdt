---
mdt: minor
mdt_cli: patch
---

Improve CLI output, performance, error handling, and test coverage.

**CLI improvements:** Verbose mode now shows provider details including names and file paths. Both `check` and `update` commands now warn about consumer blocks referencing non-existent providers. Improved help text with description and quick-start examples.

**Performance:** Optimized `Point::advance` with a new `advance_str` method that avoids allocating via `Display::to_string()` on the hot path. The lexer now uses `advance_start_str` for string-based position tracking. Engine `compute_updates` uses pre-allocated `String::with_capacity` instead of `format!` for content replacement, and avoids unnecessary equality comparison by tracking updates with a boolean flag.

**Exclude patterns:** Added `[exclude]` section to `mdt.toml` configuration with glob pattern support for skipping directories or files during scanning. Uses the `globset` crate for pattern matching.

**Error handling:** Replaced a `panic!` in the lexer with a graceful `break` when the context stack is unexpectedly empty. Added missing provider warnings to CLI output.

**Test coverage:** More than doubled the test count from 57 to 133 tests. New tests cover: all transformer types (trim, indent, prefix, wrap, codeBlock, code, replace), transformer chaining, edge cases (empty content, unicode, numeric arguments), engine operations (check, compute_updates, write_updates, idempotency, multiple consumers per file, missing providers), project scanning (hidden dirs, node_modules, exclude patterns, sub-project boundaries, source files), config loading (all formats, multiple namespaces, missing files, exclude patterns, `.yml` extension), template rendering (undefined variables, arrays, conditionals), source scanner (multiple blocks, Python comments, position tracking), error messages, and CLI integration tests (verbose output, warnings, multi-block updates, surrounding content preservation, data interpolation).

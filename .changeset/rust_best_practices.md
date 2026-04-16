---
mdt_core: major > Apply Rust best practices for error handling, memory optimization, and API design
---

## Summary

Applied comprehensive Rust best practices to improve code quality, performance, and maintainability.

## Changes

### Error Handling (err-custom-type)

- Removed `AnyError`, `AnyResult`, and `AnyEmptyResult` type aliases
- Added `impl From<MdtError> for std::io::Error` for test compatibility
- Updated all test files to use `std::io::Result<()>`

### Documentation (err-doc-errors)

- Added `# Errors` sections to public functions returning `MdtResult`:
  - `check_project`, `compute_updates`, `write_updates`
  - `scan_project`, `scan_project_with_config`, `scan_project_with_options`
  - `render_template`

### API Design (api-must_use)

- Added `#[must_use]` to result-returning public functions
- Added `#[must_use]` with `#[inline]` to small accessor methods:
  - `CheckResult::is_ok()`, `has_errors()`, `has_warnings()`
  - `ProjectCacheInspection` boolean methods
  - `CodeBlockFilter::is_enabled()`, `should_skip()`
  - `Token::increment()`

### Memory Optimization (mem-with-capacity)

- Pre-allocated vectors with known sizes:
  - `TokenWalker`: `raw_tokens.len()` capacity for tokens, groups, stack
  - `project.rs`: providers/consumers use `blocks.len()` capacity
  - `parser.rs`: pending/blocks/diagnostics use `token_groups.len()` capacity
  - `source_scanner.rs`: nodes and ranges with estimated capacity

### Compiler Optimization (opt-inline-small)

- Added `#[inline]` to small hot functions for better performance

### Memory Optimization (own-cow-conditional)

- Changed `render_template` to return `Cow<'_, str>` instead of `String`
- Avoids allocations when no template syntax is present

### Existing Best Practices (Already Present)

- `Point` and `Position` types already have `Copy` derived
- No `&Vec<T>` patterns found - already using `&[T]`
- Well-structured error types throughout codebase

## Testing

All tests pass:

- mdt_core: 532 tests
- mdt_cli: 76 tests
- Full workspace: All packages compile

Clippy passes with `-D warnings` (warnings as errors).

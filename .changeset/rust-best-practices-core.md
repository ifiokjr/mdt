---
mdt_core: major
---

Apply Rust best practices for error handling, memory optimization, and API design.

- Remove `AnyError`, `AnyResult`, and `AnyEmptyResult` type aliases in favor of `MdtResult`
- Add `# Errors` sections to public functions returning `MdtResult`
- Add `#[must_use]` to result-returning public functions and small accessor methods
- Pre-allocate vectors with known sizes in hot paths
- Add `#[inline]` to small hot functions
- Change `render_template` to return `Cow<'_, str>` to avoid allocations when no template syntax is present

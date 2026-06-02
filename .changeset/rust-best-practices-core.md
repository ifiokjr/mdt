---
mdt_core: major
---

# Tighten core error handling and API ergonomics

`mdt_core` now applies a set of Rust API and implementation best practices across error handling, allocation behavior, and public function documentation. The changes remove broad `AnyError` style aliases in favor of `MdtResult`, add `# Errors` documentation to result-returning public functions, mark useful return values with `#[must_use]`, and pre-allocate vectors in hot paths.

This is a major release because public aliases were removed and `render_template` now returns `Cow<'_, str>` to avoid allocations when no template syntax is present. Downstream callers may need to adjust type annotations or convert borrowed results when an owned `String` is required.

```rust
let rendered = render_template(template, &data)?;
let owned: String = rendered.into_owned();
```

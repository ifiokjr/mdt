---
mdt: major
mdt_cli: major
---

Replace `pad_blocks` boolean with `[padding]` configuration section.

The top-level `pad_blocks = true` setting has been replaced with a `[padding]` section that provides fine-grained control over blank lines between block tags and their content:

```toml
[padding]
before = 0 # content on next line (no blank lines)
after = 0
```

The `before` and `after` values accept:

- `false` — Content appears inline with the tag (no newline separator).
- `0` — Content on the very next line (one newline, no blank lines). **Recommended for projects using formatters** like `rustfmt` or `dprint`, as it minimizes whitespace that formatters might alter.
- `1` — One blank line between the tag and content (equivalent to the old `pad_blocks = true` behavior for source files with comment prefixes).
- `2` — Two blank lines, and so on.

When `[padding]` is present but values are omitted, `before` and `after` default to `1`.

**Migration:** Replace `pad_blocks = true` with `[padding]` in your `mdt.toml`. For the same behavior as before, use `[padding]` with no values (defaults to `before = 1, after = 1`). For compatibility with code formatters, use `before = 0, after = 0`.

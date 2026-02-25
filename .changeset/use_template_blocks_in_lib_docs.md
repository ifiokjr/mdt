---
mdt: minor
mdt_cli: minor
---

Use mdt template blocks for `mdt_core` library documentation and fix formatter compatibility.

**Template blocks in lib docs:** Replace hand-written doc comments on `Block`, `Transformer`, `Argument` structs in `parser.rs` and the module-level doc comment in `lib.rs` with mdt consumer blocks that pull content from `template.t.md` provider blocks. This ensures documentation stays synchronized across the codebase.

**Formatter compatibility fixes:**

- Set `[padding] before = 0, after = 0` in project `mdt.toml` to eliminate blank lines between tags and content that formatters would modify.
- Disable `wrap_comments` and `format_code_in_doc_comments` in `rustfmt.toml` to prevent rustfmt from reflowing doc comment text and reformatting code blocks, which would break the `mdt update → dprint fmt → mdt check` cycle.
- Fix `linePrefix` and `lineSuffix` transformers to trim trailing/leading whitespace on empty lines. Previously, `linePrefix:"//! ":true` would produce `//!` (with trailing space) on empty lines; now it produces `//!` (no trailing space), matching what formatters expect.
- Fix `pad_content_with_config` to use trimmed prefix for blank padding lines, avoiding trailing whitespace on empty comment lines in before/after padding.
- Set `keep_trailing_newline(true)` on the minijinja environment to preserve trailing newlines in rendered template content, fixing a mismatch where minijinja would strip the final newline from provider content.

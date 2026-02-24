---
mdt_core: major
---

Add `pad_blocks` configuration setting to prevent content mangling in source code comments.

When `pad_blocks = true` is set in `mdt.toml`, mdt ensures a newline always separates the opening tag from the content and the content from the closing tag. This prevents transformers like `trim` from causing content to merge directly into surrounding comment tags, which would break code structure in languages like Rust (`//!`, `///`), TypeScript (JSDoc), Python, Go, Java, C/C++, C#, Kotlin, and Swift.

The `pad_content_preserving_suffix` function preserves the trailing comment prefix from the original consumer content (e.g., `//!` before a closing tag) so that closing tags remain properly formatted after updates.

Also fixes a bug in the `Token::String` Display implementation where the `u8` delimiter byte was formatted as its numeric value (e.g., `34`) instead of the corresponding character (`"`), causing incorrect position offsets for blocks with string arguments.

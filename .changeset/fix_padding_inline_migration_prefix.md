---
mdt: patch
---

Fix comment prefix loss when migrating inline blocks to padded layout.

When a project enables `[padding]` after previously rendering blocks without it, the closing tag was pushed to its own line without the comment prefix (e.g., `///`, `//!`), producing invalid source code. The closing-tag prefix is now recovered from the closing tag's line in the source file instead of from the block content, so the migration produces valid comments. Markdown files are unaffected.

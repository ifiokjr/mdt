---
"@m-d-t/skills": minor
---

# Rewrite the mdt skill around formatter conflicts, code files, and stale-doc prevention

The skill now teaches the `mdt update → formatter → mdt check` loop with the built-in escapes (`[[formatters]]`, `[check] comparison = "lenient"`, formatter-stable providers), adds complete TypeScript, Rust, and Dart consumer examples with `[padding]` semantics, documents the block-name charset and the silent-skip behavior for unscanned extensions, adds install/upgrade guidance for `@m-d-t/cli`, corrects the closing-tag prefix behavior, and fixes broken code fences in the data-interpolation reference.

---
"mdt_lsp": fix
---

# Fix completion panics on multi-byte lines and stale providers

- Completion context sliced the cursor line by byte index using a UTF-16 column, so a cursor after any multi-byte character panicked the whole language server. Columns are now converted to byte offsets (sharing the same conversion as the rest of the server), and incremental-change ranges that fail to map are logged instead of silently dropped.
- Rename/prepare-rename ranges measure tag prefixes and names in UTF-16 code units instead of bytes, so edits land correctly on non-ASCII documents.
- Saving a template file now removes that file's previous providers before re-adding the current ones — deleted or renamed providers no longer linger in completions, go-to-definition, and rename edits until the next full rescan.
- Project scans (on initialize and config saves) run on the blocking thread pool and apply their results under a short lock instead of blocking the async runtime while holding it.

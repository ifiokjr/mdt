---
mdt_lsp: patch
---

Switch LSP text document synchronization from full to incremental mode. The server now receives only changed text ranges instead of the entire document content on each edit, improving performance for large files. Includes proper UTF-16 offset handling for LSP position conversion.

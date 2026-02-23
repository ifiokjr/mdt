---
mdt: minor
---

Refactored the internal lexer to use the `logos` crate for tokenization. This is an internal implementation change with no public API changes. The lexer now uses logos for efficient flat tokenization of HTML nodes, with a simplified state machine for context-dependent token processing.

---
mdt_core: patch
---

# Update logos and remove the empty skip regex

`logos` has been updated to 0.16.1. The tokenizer no longer uses the `#[logos(skip r"")]` attribute because the newer release rejects empty regular expressions that can match the empty string.

This keeps the lexer compatible with the current `logos` API without changing tokenization behavior for valid input.

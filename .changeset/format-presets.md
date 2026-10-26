---
"mdt_core": minor
"mdt_cli": minor
"@m-d-t/skills": docs
---

# [format] preset and trailing-whitespace rule

`[format]` grows from the single `tables` switch to a small rule set with a named preset:

- `preset = "dprint-markdown"` turns on every rule mdt implements — today `tables` and the new `trailing_whitespace` — so one line replaces the `[[formatters]]` subprocess pipeline for the common case. An explicit rule key overrides the preset for that rule alone.
- `trailing_whitespace = true` trims trailing whitespace from every line of block content, outside fenced code blocks, in providers and consumers alike. This is the classic strict-check divergence: formatters trim trailing spaces, so `mdt check` failed forever on files a formatter touched inside a block. With the rule on, mdt and the formatter agree byte-for-byte.

Like `tables`, the new rule runs inside block boundaries only, before transformers and padding, is idempotent, and `mdt update` rewrites provider blocks in `*.t.md` files that need it. The repo's own `mdt.toml` now uses the preset (dogfooding).

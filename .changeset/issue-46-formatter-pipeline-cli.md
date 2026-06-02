---
mdt_cli: minor
---

# Run configured formatters during check and update

`mdt update` and `mdt check` now support opt-in `[[formatters]]` configuration. When a formatter matches a target file, `mdt` runs the formatter against the full updated file so template output converges with project tools such as dprint or Prettier.

This lets teams keep normal formatting workflows enabled while still detecting formatter-aware template drift during checks.

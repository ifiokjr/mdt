---
mdt_cli: patch
---

Improve `mdt check` text output color handling so diagnostics and stale block summaries render with ANSI colors when the terminal supports color or `CLICOLOR_FORCE` is set, while still honoring `--no-color`, `NO_COLOR`, and `CLICOLOR=0`.

---
mdt_cli: patch
---

# Respect terminal color settings in check output

`mdt check` now applies ANSI color handling consistently across diagnostics and stale-block summaries. Color is enabled when the terminal supports it or `CLICOLOR_FORCE` is set, and it remains disabled when users pass `--no-color`, set `NO_COLOR`, or set `CLICOLOR=0`.

The result is clearer interactive output without surprising color in scripts or environments that explicitly request plain text.

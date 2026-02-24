---
mdt_cli: patch
---

Improve error display using miette for rich, contextual diagnostics.

Errors from mdt now include error codes (e.g., `mdt::unclosed_block`), actionable help text, and visual formatting with Unicode markers when color is enabled. The miette handler respects `--no-color` and the `NO_COLOR` environment variable.

Validation diagnostics (unclosed blocks, unknown transformers, unused providers) are now rendered through miette with severity levels (error vs warning) and context-specific help messages.

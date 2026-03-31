---
mdt_cli: note
mdt_lsp: note
---

Improve automated coverage for the CLI npm distribution flow and add direct language-server lifecycle tests.

This adds integration coverage for the npm launcher and npm packaging scripts, updates the coverage workflow to include JavaScript coverage alongside Rust coverage, and exercises LSP initialize/open/change/close/shutdown paths more directly.

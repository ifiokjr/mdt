---
mdt_cli: none
---

# Expand CLI distribution and LSP coverage

The automated coverage suite now exercises more of the npm distribution path and language-server lifecycle. Integration tests cover the npm launcher, package-generation scripts, and direct LSP initialize/open/change/close/shutdown flows.

Coverage reporting also includes JavaScript coverage alongside Rust coverage, making regressions in the generated npm tooling easier to spot before release.

---
mdt_core: patch
mdt_cli: patch
---

Fix collapsed newlines in `mdtBadgeLinks` provider block in `template.t.md`. The multi-line link reference definitions were accidentally collapsed to a single line by an external markdown formatter (`dprint fmt`) that doesn't recognize `{{ }}` template syntax in URLs as valid link definitions.

Restored the template content to its correct multi-line format. Added unit tests and CLI integration tests to verify that `mdt update` preserves newlines in multi-line content through the full scan → render → update pipeline, including idempotency after write-back.

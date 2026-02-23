---
mdt: patch
mdt_cli: patch
---

Add `cargo-deny` for automated security auditing, license compliance checking, and dependency ban enforcement. Integrates with CI via `EmbarkStudios/cargo-deny-action` and adds `deny:check` to the local `lint:all` workflow.

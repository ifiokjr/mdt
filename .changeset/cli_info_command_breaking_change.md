---
mdt_cli: major
---

Add a new public `Commands::Info` variant to `mdt_cli` and improve human-readable CLI output formatting (`mdt check` and new `mdt info`).

This is marked major because `Commands` is a public enum and adding a variant is a breaking change for exhaustive matches in downstream crates.

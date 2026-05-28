---
mdt_core: minor
---

Add structured tracing instrumentation via the `tracing` crate.

`mdt_core` instruments key public API functions with `#[instrument]` spans and emits `debug!`, `trace!`, and `warn!` events at important processing boundaries.

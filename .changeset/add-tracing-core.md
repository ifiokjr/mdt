---
mdt_core: minor
---

# Instrument core template processing with tracing

`mdt_core` now emits structured tracing spans and events around important template-processing boundaries. Public API entry points are annotated with `#[instrument]`, and the engine records `debug!`, `trace!`, and `warn!` events while loading projects, resolving providers, rendering consumers, and reporting notable processing states.

This makes failures and performance issues easier to diagnose from CLI, LSP, and MCP callers without changing the core API or the rendered markdown output.

---
mdt_core: minor
mdt_cli: minor
mdt_lsp: minor
mdt_mcp: minor
---

Add structured tracing instrumentation via the `tracing` crate.

`mdt_core` instruments key public API functions with `#[instrument]` spans and emits `debug!`, `trace!`, and `warn!` events at important processing boundaries. CLI, LSP, and MCP binaries initialize `tracing-subscriber` with `EnvFilter` controlled by the `MDT_LOG` environment variable (e.g. `MDT_LOG=mdt_core=debug`). LSP and MCP output to stderr to avoid interfering with their stdio-based protocols.

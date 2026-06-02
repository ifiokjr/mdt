---
mdt_lsp: minor
---

# Expose LSP diagnostics through MDT_LOG tracing

The language server now initializes `tracing-subscriber` with an `EnvFilter` sourced from `MDT_LOG`. Logs are written to stderr so tracing never interferes with the JSON-RPC protocol carried over stdio.

This gives editor integrations a safe opt-in diagnostics path for initialization, document updates, and template checks while preserving the default quiet behavior expected by LSP clients.

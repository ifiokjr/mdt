---
mdt_cli: minor
---

# Expose structured CLI logs with MDT_LOG

The CLI now initializes `tracing-subscriber` with an `EnvFilter` sourced from `MDT_LOG`. This gives operators and contributors a consistent way to inspect command execution without adding ad-hoc debug output or changing normal terminal output.

The subscriber is installed at process startup and defaults to quiet behavior unless the environment variable is set. Users can opt into targeted diagnostics for parser, project-loading, update, or check flows while preserving the existing user-facing command experience.

---
mdt_mcp: security
---

Upgrade `rmcp` from 3.8.0 to 3.11.0 so `cargo deny` no longer fails on [RUSTSEC-2026-0462](https://rustsec.org/advisories/RUSTSEC-2026-0462). Before 3.9.0, rmcp's built-in Streamable HTTP clients (`transport-streamable-http-client-reqwest` and `transport-streamable-http-client-unix-socket`) read non-streaming response bodies into memory with no size limit. A malicious or compromised MCP server could send a body that never ends, including during `initialize` before authentication, and exhaust the host process. 3.9.0 caps successful JSON bodies at 16 MiB and error bodies at 64 KiB. 3.11.0 is the current release and keeps that limit.

mdt's server enables `server`, `transport-io`, and `macros` (stdio only) and does not turn those HTTP clients on. The advisory is still reported: `deny.toml` audits the locked crate with `all-features`, so the vulnerable version fails CI even though this binary does not link the affected transports. The version requirement is now `3.11.0`, which cargo treats as `^3.11.0` and will not resolve back to 3.8.0.

The unused `RUSTSEC-2024-0449` ignore is removed. That advisory covered unmaintained `unsafe-libyaml` through `serde_yaml_ng`. YAML data files already parse with `serde-saphyr`, and no crate in the graph matches the advisory anymore (`advisory-not-detected`).

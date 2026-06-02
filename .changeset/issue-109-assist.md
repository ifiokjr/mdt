---
mdt_cli: major
---

# Add official assistant setup profiles

The CLI now includes an `mdt assist` command that prints official assistant setup profiles. It focuses on practical adoption by producing ready-to-copy MCP configuration snippets and suggested repo-local guidance for Claude, Cursor, Copilot, Pi, and generic MCP clients.

This is a major release because the public CLI command model gains a new `Commands::Assist` variant. Downstream crates that exhaustively match command variants will need to handle the new case.

```rust
match command {
    Commands::Assist(args) => run_assist(args),
    other => run_existing_command(other),
}
```

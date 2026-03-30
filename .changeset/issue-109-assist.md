---
mdt_cli: major
---

Add an `mdt assist` command that prints official assistant setup profiles.

This first slice focuses on practical adoption rather than a plugin marketplace: the command prints ready-to-copy MCP configuration snippets and suggested repo-local guidance for assistants like Claude, Cursor, Copilot, Pi, and generic MCP clients.

This is marked major because it adds a new public `Commands::Assist` variant to `mdt_cli`'s public CLI model.

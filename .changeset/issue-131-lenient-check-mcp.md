---
mdt_mcp: minor
---

# Support lenient comparison in MCP checks

The MCP `mdt_check` tool now honors `[check] comparison = "lenient"`, matching the CLI behavior for whitespace-tolerant verification. Agent workflows can therefore distinguish real template drift from harmless formatter whitespace changes.

The MCP response still reports mismatches when normalized content differs, and update behavior remains exact.

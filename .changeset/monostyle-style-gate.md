---
@m-d-t/cli: patch
mdt_cli: patch
mdt_core: patch
mdt_mcp: patch
---

# Apply the monostyle style gate

Blank-line layout from `monostyle fix` (padding around control flow, a blank before returns, collapse of stacked blank runs), with `monostyle.toml` configuring the rules and a CI step that reports findings as inline PR annotations.

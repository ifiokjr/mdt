---
mdt_core: patch
mdt_mcp: patch
---

Update `logos` to 0.16.1 and `rmcp` to 1.3.0.

**logos 0.16.1**: Remove the `#[logos(skip r"")]` attribute which is now rejected because an empty regex matches the empty string. The default skip pattern is used instead; all meaningful bytes are already covered by explicit token variants.

**rmcp 1.3.0**: `ServerInfo` and `CallToolResult` are now `#[non_exhaustive]`. Use `ServerInfo::new().with_instructions()` builder pattern and `CallToolResult::success()` / `CallToolResult::error()` constructors instead of struct literals.

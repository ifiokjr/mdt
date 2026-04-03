---
mdt_cli: minor
mdt_core: minor
mdt_mcp: minor
---

Add `[check] comparison = "lenient"` config option for whitespace-tolerant `mdt check`.

When set to `"lenient"`, `mdt check` normalizes whitespace (blank line count, trailing spaces) before comparing expected vs actual block content. This makes the check tolerant of external formatter rewrites without needing to exclude template files from formatters.

`mdt update` always writes exact bytes regardless of this setting.

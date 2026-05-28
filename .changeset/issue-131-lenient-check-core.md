---
mdt_core: minor
---

Add `[check] comparison = "lenient"` config option for whitespace-tolerant block comparison.

When set to `"lenient"`, the engine normalizes whitespace (blank line count, trailing spaces) before comparing expected vs actual block content. `mdt update` always writes exact bytes regardless of this setting.

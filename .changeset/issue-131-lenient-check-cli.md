---
mdt_cli: minor
---

# Support lenient whitespace comparison in check

`mdt check` now honors `[check] comparison = "lenient"` for whitespace-tolerant verification. This mode allows projects to keep external formatters enabled without reporting stale blocks for harmless whitespace rewrites.

The command still reports meaningful content drift, while `mdt update` continues to write exact rendered bytes regardless of the comparison setting.

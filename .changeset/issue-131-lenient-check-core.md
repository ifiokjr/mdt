---
mdt_core: minor
---

# Add lenient block comparison to configuration

`mdt_core` now supports `[check] comparison = "lenient"` for whitespace-tolerant block comparison. In lenient mode, the engine normalizes blank-line counts and trailing whitespace before comparing expected and actual consumer content.

This reduces false-positive stale-block reports after external formatter rewrites. Update operations remain exact and continue to write the rendered provider output byte-for-byte.

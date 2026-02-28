---
mdt_core: minor
mdt_cli: minor
---

Add cache observability across core and CLI diagnostics.

- Persist cache telemetry counters in the project index cache (scan count, full-hit count, cumulative reused/reparsed file counts, and last scan details).
- Expose cache inspection APIs from `mdt_core::project` for diagnostics surfaces.
- Extend `mdt info` with a cache section in text and JSON output (artifact health, schema/key compatibility, hash mode, cumulative metrics, and last scan summary).
- Extend `mdt doctor` with cache checks for artifact validity, hash mode guidance, and efficiency trend heuristics.
- Add unit/e2e/snapshot coverage and docs updates for the new observability output.

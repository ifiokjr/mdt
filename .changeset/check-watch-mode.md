---
mdt_cli: minor
---

Add `--watch` flag to `mdt check` command. When enabled, the check command monitors the project directory for file changes and automatically re-runs the check whenever files are modified or created. Uses 200ms debouncing to avoid redundant checks during rapid file changes. Unlike single-run mode, watch mode does not exit with a non-zero status code on stale consumers -- it prints the results and continues watching.

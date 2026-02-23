---
mdt_cli: patch
---

Fix docs-pages workflow by enabling automatic GitHub Pages configuration. The `enablement: true` flag on `actions/configure-pages@v5` auto-enables Pages via the GitHub API, resolving the "Get Pages site failed" error.

---
mdt_cli: patch
---

Fix release and docs-pages CI workflows.

**Release workflow:** Remove the strict version verification step that caused failures when tags were created by knope before version bumps. Add `workflow_dispatch` trigger with a `tag` input so release builds can be manually triggered for any `mdt_cli` tag. Check out the tag ref directly instead of `main` so binaries are built from the tagged commit.

**Docs-pages workflow:** Fix cancellation issue where multiple simultaneous releases caused the valid `mdt_cli` run to be cancelled by a subsequent non-matching release. Changed `cancel-in-progress` to `false` so runs queue instead of cancelling. Add `workflow_dispatch` trigger with an optional `ref` input (tag, branch, or commit SHA) for manually building and deploying docs. Check out the specified ref for both release and manual triggers.

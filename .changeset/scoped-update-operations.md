---
"mdt_core": minor
"mdt_cli": minor
"mdt_mcp": minor
"@m-d-t/skills": docs
---

# Scoped update operations

`mdt update` can now sync a subset of the project instead of everything, for focused edits and agent workflows:

- `mdt update --block <NAME>` (repeatable) updates only consumers of the named blocks. An unknown name exits 2 with the closest existing block names.
- `mdt update --file <PATH>` (repeatable) updates only consumers in the given files. Paths resolve against the current directory and must stay inside the project root; a path matching no scanned file with consumers is an error.
- The MCP `mdt_update` tool accepts `block_names` for the same block scoping; an unknown name is an `isError` result with code `mdt::block_not_found` and suggested provider names. `dry_run` keeps its meaning.
- Output states the scope (`scoped to 1 block(s): intro`), and counts plus file lists cover only the scoped work.

Design notes:

- The scan stays whole-project (cheap, cached), so missing-provider and orphan semantics are unchanged; only the write is scoped.
- Validation still gates project-wide, exactly like `mdt update` — any validation error refuses the scoped write.
- Render failures in out-of-scope consumers never block a scoped run; in-scope failures are still reported and counted. `[[formatters]]` only run for files the scope touches.
- `--watch` composes with scoping and re-runs the same scope; `--block` and `--file` combine as an intersection.
- `mdt_core` exposes this as `UpdateScope` (`compute_updates(&ctx, &scope)`), with `UpdateScope::all()` matching the previous behavior.

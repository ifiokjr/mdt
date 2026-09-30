---
"mdt_cli": feat
---

# Add `mdt skill` to load the agent skill straight from the CLI

The mdt agent skill (published separately as `@m-d-t/skills`) is now embedded in the `mdt` binary, so any AI coding agent can learn mdt without installing the skill package, and the instructions always match the installed version.

- `mdt skill` prints `SKILL.md`, the entrypoint an agent reads first.
- `mdt skill --reference` prints `REFERENCE.md`, the full syntax, transformer, and configuration reference.
- `mdt skill --install <DIR>` writes both files to `<DIR>/mdt/` (for example `mdt skill --install .claude/skills`), replacing an older copy.

```sh
mdt skill | head
mdt skill --install .claude/skills
```

`mdt --help` lists the new command so agents discover it on their own.

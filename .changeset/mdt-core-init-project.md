---
"mdt_core": feat
---

# Add `init::init_project`, the shared implementation behind `mdt init` and `mdt_init`

`mdt_core::init::init_project(root)` adds only what a project is missing and reports what it did through `InitReport` (`SampleOutcome`, `ConfigOutcome`, `GitignoreOutcome`):

- writes the annotated starter `mdt.toml` unless `mdt.toml`, `.mdt.toml`, or `.config/mdt.toml` exists;
- writes a sample `greeting` provider to `.templates/template.t.md` unless a sample template or any provider already exists, so it never introduces a duplicate provider into a project that uses mdt without a config;
- writes `readme.md` with the sample consumer already synced through the engine (so an existing `[padding]` is honoured) when the project has no README of any case or extension, and never modifies an existing README;
- adds `.mdt/` to `.gitignore` in git repositories so the local cache is not committed;
- creates the root directory when it does not exist.

The CLI and the MCP server both call it, so they produce the same files and always leave the project passing `mdt check`.

---
"@m-d-t/skills": minor
---

# Rewrite the mdt skill from end-to-end agent evaluations and ship it inside the CLI

The skill was rebuilt from evaluations where agents adopted mdt in polyglot monorepos using only the skill. It is now also embedded in the `mdt` binary: agents without the skill installed can run `mdt skill` (and `mdt skill --reference`), and `mdt skill --install <dir>` installs it for Claude Code, Pi, Copilot, or any agent that reads `.agents/skills`.

- A shorter `SKILL.md` covering the workflow, block rules, transformers, per-language comment prefixes (including indented class methods and `impl` blocks), data, formatter recipes verified with dprint, prettier, and rustfmt, CI and exit codes, monorepos with shared providers, and a table mapping each `mdt check` failure to its fix.
- A complete `REFERENCE.md`: tag syntax, block arguments, the transformer table, exact padding semantics, data formats and script caching, every `mdt.toml` key, scanning and git-ignore rules, diagnostic codes, CLI exit codes and JSON output, and MCP tool contracts.
- Corrects claims that were wrong: `[include]` and `[templates] paths` add to the scan, `lenient` comparison only handles trailing whitespace and blank lines, unclosed tags are errors, blocks cannot be nested, formatter placeholders must stay double-quoted, and a `replace` example no longer closes the comment it sits in.

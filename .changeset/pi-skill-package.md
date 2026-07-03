---
mdt_cli: minor
---

# Publish official agent skills for mdt

`mdt` now publishes an official `@m-d-t/skills` npm package for Pi and other harnesses that support the Agent Skills standard. The package includes quick-start instructions, MCP tool guidance, and a detailed reference for template syntax, transformers, interpolation, inline blocks, configuration, CLI commands, MCP tools, and source-file patterns.

The release tooling now generates and publishes the skills package alongside the CLI, and `mdt assist pi` points users toward the packaged skill.

```sh
pi install npm:@m-d-t/skills
pi -e npm:@m-d-t/skills
```

---
mdt_cli: patch
---

# Rename npm packages under the m-d-t scope

The npm distribution has moved from the `@ifi` scope to the `@m-d-t` organization. The top-level CLI package is now `@m-d-t/cli`, the skills package is `@m-d-t/skills`, and all platform-specific binary packages now use the `@m-d-t/cli-*` naming pattern.

This aligns npm package names with the project name and makes the distribution easier to recognize in package registries and install commands.

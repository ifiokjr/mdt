# Release process

## Release tooling

This repo uses `knope` for changesets and releases.

Common commands:

- `knope document-change`
- `knope release`
- `knope publish`

## Changeset requirement

Any PR that changes code in a publishable crate must include at least one `.changeset/*` file.

A changeset can use these change types:

- `major`
- `minor`
- `patch`
- `docs`
- `note`

Publishable packages:

- `mdt_core`
- `mdt_cli`
- `mdt_lsp`
- `mdt_mcp`

After creating or editing changesets, run:

```sh
dprint fmt .changeset/* --allow-no-files
```

## Release notes guidance

- Use detailed, concrete changeset descriptions.
- Conventional commit scopes should match the affected package when possible.

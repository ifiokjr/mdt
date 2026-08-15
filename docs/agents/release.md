# Release process

## Release tooling

This repo uses `monochange` (the `mc` CLI) for changesets and releases.

Common commands:

- `mc create` — create a changeset file
- `mc check` — validate config, changesets, and manifest lints
- `mc preview` — preview planned version bumps and changelogs
- `mc step publish-readiness` / `mc step publish-packages` — publish steps run by CI

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

## npm publishing

npm publishing is handled by the `publish` workflow, which runs on a `v*` tag push (with `workflow_dispatch` retained for manual retries).

- The `publish` workflow builds and uploads the GitHub release binaries (8 platform targets), attests them, then publishes the crates and npm packages and flips the draft release to public.
- The `release-pr` workflow creates the release PR, and on merge pushes the release tag and creates the draft release. The tag push fires `publish`.
- The top-level package is `@m-d-t/cli`.
- The agent skill package is `@m-d-t/skills` (a pi-compatible skill package).
- Platform packages are published first (for Linux, macOS, and Windows targets).
- The skills package is published after platform packages.
- The top-level package is published last and depends on those platform packages through `optionalDependencies`.
- The `publish` workflow can also be run manually with a `tag` input to republish or recover a specific release.
- Re-running publish is safe: packages that are already published at the target version are skipped.

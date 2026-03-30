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

## npm publishing

npm publishing is handled by a separate `npm-publish` workflow when `NPM_TOKEN` is configured.

- The `release` workflow builds and uploads the GitHub release binaries, then publishes a small metadata artifact containing the release tag.
- The `npm-publish` workflow runs after the `release` workflow completes successfully, downloads that metadata artifact, then repackages the exact release binaries into npm packages.
- `npm-publish` checks out the default branch tooling rather than the release tag itself, so manual reruns can publish older release tags even if the npm packaging scripts were added later.
- The top-level package is `@ifi/mdt`.
- Platform packages are published first (for Linux, macOS, and Windows targets).
- The top-level package is published last and depends on those platform packages through `optionalDependencies`.
- The `npm-publish` workflow can also be run manually with a `tag` input to republish or recover a specific release.
- Re-running npm publish is safe: packages that are already published at the target version are skipped.

# Release process

## Release tooling

This repo uses `monochange` (`mc`) for changesets and releases.

Common commands:

- `mc create --interactive`
- `mc step:prepare-release --dry-run`
- `mc step:publish-release`

## Changeset requirement

Any PR that changes code in a publishable crate must include at least one `.changeset/*` file.

A changeset can use these change types:

- `major`
- `breaking`
- `minor`
- `feat`
- `change`
- `patch`
- `refactor`
- `test`
- `fix`
- `none`
- `docs`
- `security`
- `perf`

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

## Publishing

A single `publish` workflow handles the whole release pipeline, and the `release-pr` workflow dispatches it at two points:

- While the release PR is open, each push to `main` with pending changesets dispatches a dry run against the release PR branch itself (`dry_run=true` with `checkout_ref` set to the release branch). The dry run builds every release archive and validates the publish pipeline but creates no tag, no draft release, and publishes nothing.
- After the release PR merges, the tag job pushes the release tags, creates the draft GitHub release, then dispatches the workflow for real. It uploads the archives to the draft release, attests them, checks publish readiness, publishes the cargo and npm packages via `monochange` with trusted publishing, and finally publishes the draft release.

The publish workflow can also be dispatched manually with a `tag` input to retry a release. Re-running npm publish is safe: packages that are already published at the target version are skipped.

- The top-level package is `@m-d-t/cli`.
- The agent skill package is `@m-d-t/skills` (a pi-compatible skill package).
- Platform packages are published first (for Linux, macOS, and Windows targets).
- The skills package is published after platform packages.
- The top-level package is published last and depends on those platform packages through `optionalDependencies`.

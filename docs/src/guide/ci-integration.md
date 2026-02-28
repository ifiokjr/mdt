# CI Integration

mdt's `check` command is designed for CI pipelines. It verifies that all consumer blocks are up to date and exits with a non-zero status code if any are stale.

## Basic CI check

Add a step to your CI workflow that runs `mdt check`:

```yaml
- name: check documentation is up to date
  run: mdt check
```

If any consumer blocks are out of date, the step fails and the pipeline reports which blocks need updating.

## CI diagnostics triage

When `mdt check` fails in CI, add diagnostics commands so logs include root-cause context:

```yaml
- name: diagnostics
  run: |
    mdt info
    mdt doctor
```

This gives you:

- Project/config resolution details (`mdt.toml`, `.mdt.toml`, `.config/mdt.toml`)
- Provider/consumer linkage summary (orphans, missing providers, duplicates)
- Cache artifact health and reuse/reparse telemetry
- Actionable doctor hints for config/data/layout/cache issues

## GitHub Actions

### Full workflow example

```yaml
name: docs
on:
  pull_request:
    branches: [main]

jobs:
  check-docs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: install mdt
        run: cargo install mdt_cli

      - name: check documentation sync
        run: mdt check
```

### GitHub Actions annotations

Use `--format github` to produce GitHub Actions annotation output. This adds inline warnings on the pull request diff showing exactly which files have stale blocks:

```yaml
- name: check documentation sync
  run: mdt check --format github
```

This produces output like:

```
::warning file=readme.md::Consumer block `install` is out of date
```

GitHub renders these as yellow warning annotations directly on the affected lines in the PR diff.

### With diff output

Use `--diff` to include a unified diff in the CI output showing what changed:

```yaml
- name: check documentation sync
  run: mdt check --diff
```

## JSON output

For integration with other tools, use `--format json`:

```yaml
- name: check documentation sync
  run: mdt check --format json
```

Output when everything is up to date:

```json
{ "ok": true, "stale": [] }
```

Output when blocks are stale:

```json
{
	"ok": false,
	"stale": [
		{ "file": "readme.md", "block": "install" },
		{ "file": "src/lib.rs", "block": "docs" }
	]
}
```

## Pre-commit hook

You can also use mdt as a pre-commit check to prevent committing stale docs:

```bash
#!/bin/sh
# .git/hooks/pre-commit

mdt check --format text
if [ $? -ne 0 ]; then
  echo ""
  echo "Documentation is out of date. Run 'mdt update' before committing."
  exit 1
fi
```

## Automated fixes

If you prefer to auto-fix in CI rather than just check, run `mdt update` and commit the result:

```yaml
- name: update documentation
  run: mdt update

- name: check for changes
  run: |
    if [ -n "$(git status --porcelain)" ]; then
      echo "mdt update produced changes. Please run 'mdt update' locally and commit."
      git diff
      exit 1
    fi
```

## Publish mdBook on release

This repository publishes the mdBook when an `mdt_cli` release is published on GitHub (or via manual `workflow_dispatch`). Other crate releases (e.g., `mdt_core`, `mdt_lsp`, `mdt_mcp`) do not trigger a docs deploy.

The workflow lives at `.github/workflows/docs-pages.yml` and:

1. Filters on `mdt_cli` release tags (or manual dispatch)
2. Builds the book with `mdbook build docs`
3. Uploads `docs/book` as a Pages artifact
4. Deploys to GitHub Pages

Equivalent workflow structure:

```yaml
name: docs-pages

on:
  release:
    types: [published]
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

jobs:
  build:
    # Only deploy docs on mdt_cli releases (not library-only releases).
    if: >-
      github.event_name == 'workflow_dispatch' ||
      startsWith(github.event.release.tag_name, 'mdt_cli')
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: taiki-e/install-action@v2
        with:
          tool: mdbook
      - uses: actions/configure-pages@v5
      - run: mdbook build docs
      - uses: actions/upload-pages-artifact@v3
        with:
          path: docs/book

  deploy:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/deploy-pages@v4
```

## Benchmark CI (this repository)

This repository also runs `.github/workflows/benchmark.yml` on `pull_request` and `push` to `main`.

The benchmark job:

1. Builds `mdt` for a baseline ref and the candidate ref.
2. Runs both binaries against the same deterministic workload.
3. Compares medians per scenario with relative and absolute thresholds.
4. Uploads raw benchmark artifacts and posts a PR comment report.

When regressions exceed threshold, pull requests must include a `## Benchmark Justification` section in the PR description to document the tradeoff.

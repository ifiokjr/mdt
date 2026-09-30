# CI Integration

`mdt check` is the whole gate. It exits non-zero when anything is wrong:

- `0`: every consumer is linked to a provider and up to date.
- `1`: a consumer is stale, an orphan (no provider), or its provider fails to render, or a file drifted from its formatter output.
- `2`: validation errors (unclosed, unmatched, nested, or invalid tags, unknown transformers, duplicate providers) or config and data errors.

Warnings, such as unused providers, are printed but never fail the check.

## Install mdt in CI

Use the prebuilt binary from npm and pin the version you use locally (`mdt --version`):

```sh
npx -y @m-d-t/cli@0.9.5 check --format github
```

Or install it once for several steps:

```sh
npm install -g @m-d-t/cli@0.9.5
```

`cargo install mdt_cli` also works, but it builds from source and takes minutes.

## GitHub Actions

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

      - name: check documentation sync
        run: npx -y @m-d-t/cli@0.9.5 check --format github
```

### Annotations

`--format github` prints one annotation per problem, which GitHub shows inline on the pull request. Failures are `::error`, warnings are `::warning`:

```text
::error file=readme.md,line=3,col=1::Consumer block `install` is out of date; run `mdt update`
::error file=readme.md,line=3,col=1::consumer `installguide` at readme.md:3:1 has no provider (did you mean `installGuide`?)
::error file=readme.md,line=6,col=1::Template render failed for block `badges`: argument count mismatch: provider `badges` declares 1 parameter(s), but consumer passes 2
::error file=readme.md::Formatter-normalized file output is out of date; run `mdt update`
::warning file=.templates/docs.t.md,line=1,col=1::provider block `installGuide` has no consumers
```

Validation errors, such as `mdt::invalid_tag` or `mdt::nested_block`, are annotated the same way. A one-line summary goes to stderr.

Limitations:

- Annotation paths are relative to the mdt project root. For a sub-project (checked with `--path` or from inside its directory), GitHub reads them relative to the repository root, so the inline annotation can land on the wrong file. The message text still names the right file.
- Errors that stop the scan before any file is checked (an invalid `mdt.toml`, duplicate providers, an unreadable file) print a full report on stderr instead of annotations. The step still fails with exit 2.

### Show the diff

`--diff` adds a unified diff for each stale consumer to the text output, so the log shows exactly what changed:

```sh
mdt check --diff
```

### Diagnostics on failure

`mdt info` and `mdt doctor` explain config resolution, orphan and unused blocks, parser diagnostics, and cache health. Run them only when the check fails:

```yaml
- name: mdt diagnostics
  if: failure()
  run: |
    npx -y @m-d-t/cli@0.9.5 info
    npx -y @m-d-t/cli@0.9.5 doctor
```

## JSON output

For other tools, use `--format json`:

```sh
mdt check --format json
```

<!-- {=mdtCheckJsonOutput} -->

`mdt check --format json` prints one object with every key present:

| Key           | Entries                                                                      |
| ------------- | ---------------------------------------------------------------------------- |
| `ok`          | `true` when every consumer is linked and current                             |
| `stale`       | Stale consumers: `file`, `block`, `line`, `column`                           |
| `stale_files` | Files a formatter would change: `file`                                       |
| `orphans`     | Consumers with no provider: `file`, `block`, `line`, `column`, `suggestions` |
| `errors`      | Render errors: `file`, `block`, `line`, `column`, `message`                  |
| `diagnostics` | Errors and warnings: `severity`, `code`, `file`, `line`, `column`, `message` |

A clean project:

```json
{
	"diagnostics": [],
	"errors": [],
	"ok": true,
	"orphans": [],
	"stale": [],
	"stale_files": []
}
```

A stale consumer, an orphan, and an unused-provider warning:

```json
{
	"diagnostics": [
		{
			"code": "mdt::unused_provider",
			"column": 1,
			"file": ".templates/template.t.md",
			"line": 11,
			"message": "provider block `unused` has no consumers",
			"severity": "warning"
		}
	],
	"errors": [],
	"ok": false,
	"orphans": [
		{
			"block": "intor",
			"column": 1,
			"file": "readme.md",
			"line": 7,
			"suggestions": ["intro"]
		}
	],
	"stale": [{ "block": "intro", "column": 1, "file": "readme.md", "line": 3 }],
	"stale_files": []
}
```

When validation errors (such as an unclosed or nested block) stop the check, the same object is printed with `ok: false` and the errors in `diagnostics`, and the exit status is 2. Errors that stop the scan itself (config and data errors, duplicate providers, unreadable files) print only the error report on stderr, also with exit status 2.

<!-- {/mdtCheckJsonOutput} -->

## Formatters in CI

If `mdt.toml` has `[[formatters]]`, `mdt check` runs those commands. Install the same formatters at the same versions you use locally, or CI reports formatter drift that does not exist on your machine. A formatter that fails or is missing is an error (exit 2).

## Monorepos

Each directory with its own `mdt.toml`, `.mdt.toml`, or `.config/mdt.toml` is a separate project that the root scan skips. Check each one:

```yaml
- name: check docs
  run: |
    npm install -g @m-d-t/cli@0.9.5
    mdt check
    mdt check --path packages/lib-a
    mdt check --path packages/lib-b
```

See [Monorepo setups](../advanced/monorepos.md) for a loop that finds every sub-project.

## Pre-commit hook

```sh
#!/bin/sh
# .git/hooks/pre-commit
if ! mdt check; then
  echo "mdt check failed. Run 'mdt update' or fix the errors above before committing."
  exit 1
fi
```

## This repository's workflows

mdt's own docs site is built and deployed by [`docs-pages.yml`](https://github.com/ifiokjr/mdt/blob/main/.github/workflows/docs-pages.yml) on every push to `main`. Performance regressions are caught by [`benchmark.yml`](https://github.com/ifiokjr/mdt/blob/main/.github/workflows/benchmark.yml); see [Benchmarking and regressions](../advanced/benchmarking-and-regressions.md).

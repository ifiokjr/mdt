# Troubleshooting

This page covers common errors, debugging techniques, and solutions for issues you might encounter with mdt.

## Common errors

### Consumer references a missing source

```
warning: consumer `installGuide` in readme.md has no matching source
```

**Cause:** The target tag references a source name that doesn't exist in any `*.t.md` file.

**Solutions:**

- Check for typos in the block name. Names are case-sensitive — `installGuide` and `installguide` are different.
- Verify the source is in a `*.t.md` file. Source tags in regular `.md` files are ignored.
- If you're in a monorepo, confirm the source is in the same project scope. Providers from a parent or sibling project are not visible across `mdt.toml` boundaries. See [Monorepo setups](./advanced/monorepos.md).
- Run `mdt list` to see all discovered sources and targets.

### Argument count mismatch

```
error: argument count mismatch: provider `badges` declares 1 parameter(s),
       but consumer passes 2 argument(s)
```

**Cause:** The target passes a different number of arguments than the source declares.

**Solutions:**

- Count the `:"value"` segments on both the source and target tags.
- If the source declares `<!-- {@badges:"crate_name"} -->` (1 parameter), every consumer must pass exactly 1 argument: `<!-- {=badges:"mdt_core"} -->`.
- See [Block Arguments](./advanced/block-arguments.md) for details.

### Duplicate source name

```
error: duplicate source `install`: defined in `docs.t.md` and `api.t.md`
```

**Cause:** Two `*.t.md` files define a source with the same name. Source names must be unique within a project scope.

**Solution:** Rename one of the sources, or consolidate them into a single template file.

### Stale blocks after editing templates

After editing a source's content in a template file, all targets referencing that provider become stale. `mdt check` reports them:

```
Target block `install` in readme.md is out of date.
Target block `install` in src/lib.rs is out of date.
```

**Solution:** Run `mdt update` to sync all targets. During development, use `mdt update --watch` to auto-sync on file changes.

## Debugging techniques

### Use `mdt check --verbose`

Verbose mode shows the full scan results — how many files were scanned, which sources and targets were found, and which blocks are stale:

```sh
mdt check --verbose
```

### Use `mdt list` to see all blocks

`mdt list` displays every source and target in the project, their file locations, and their link status:

```sh
mdt list
```

```
Sources:
  @installGuide .templates/template.t.md (2 target(s))
  @apiDocs .templates/template.t.md (3 target(s))

Targets:
  =installGuide readme.md [linked]
  =installGuide crates/my-lib/readme.md [linked]
  =apiDocs readme.md [linked]
  =orphanBlock docs/old.md [orphan]
```

Orphaned consumers (`[orphan]`) indicate missing sources. Providers with `(0 target(s))` might be unused.

### Use `mdt check --diff`

When blocks are stale, `--diff` shows exactly what changed:

```sh
mdt check --diff
```

This produces a unified diff for each stale block, making it easy to see whether the change is expected.

### Use `mdt update --dry-run`

Preview what `mdt update` would change without modifying any files:

```sh
mdt update --dry-run
```

```
Dry run: would update 3 block(s) in 2 file(s):
  readme.md
  src/lib.rs
```

## Cache observability and diagnostics

If cache behavior looks suspicious (unexpected reparses, stale cache artifact, inconsistent local vs CI behavior), use:

```sh
mdt info
mdt doctor
```

`mdt info` shows cache telemetry:

- Artifact path and schema support
- Hash verification mode
- Cumulative reused vs reparsed file totals
- Last scan summary (`full cache hit` vs `incremental reuse`)

`mdt doctor` adds cache health checks:

- `Cache Artifact` validates readability/schema/key compatibility
- `Cache Hash Mode` explains current fingerprint mode and troubleshooting toggle
- `Cache Efficiency` warns when reparses dominate over time

For strict cache-key validation during investigation:

```sh
MDT_CACHE_VERIFY_HASH=1 mdt check
```

This includes content hashes in cache fingerprints (in addition to size/mtime). Disable it again for baseline behavior comparisons.

## Formatter interference

Code formatters like dprint, Prettier, and rustfmt can reformat content inside target files, which used to create `mdt update → formatter → mdt check` loops.

### Symptoms

- `mdt check` reports stale blocks after running a formatter.
- Running `mdt update` followed by the formatter followed by `mdt check` always shows stale blocks.
- Whitespace, wrapping, or markdown table changes keep reappearing.

### Recommended fix: configure `[[formatters]]`

Use formatter integration so mdt formats the full updated file before comparing or writing it.

```toml
[[formatters]]
command = "dprint fmt --stdin \"$MDT_FORMAT_FILE\""
patterns = ["**"]
```

For per-language tools, add multiple entries:

```toml
[[formatters]]
command = "prettier --stdin-filepath \"$MDT_FORMAT_FILE\""
patterns = ["**/*.ts", "**/*.tsx"]

[[formatters]]
command = "eslint_d --fix-to-stdout --stdin --stdin-filename \"$MDT_FORMAT_FILE\""
patterns = ["**/*.ts", "**/*.tsx"]
```

This makes `mdt update` and `mdt check` use the same formatter-aware full-file pipeline.

### Additional mitigations

#### Set padding to minimize whitespace differences

Use `[padding]` in `mdt.toml` to control the exact whitespace between tags and content. This reduces the surface area for formatter conflicts:

```toml
[padding]
before = 0
after = 0
```

See [Configuration](./guide/configuration.md) for details on padding values.

#### Match transformer output to formatter expectations

If a formatter enforces specific indentation, configure your transformers to produce output that already matches. For example, if your formatter expects tabs:

```
<!-- {=docs|trim|indent:"\t"} -->
<!-- {/docs} -->
```

#### Exclude template files as a temporary workaround

If you cannot enable `[[formatters]]` yet, excluding `*.t.md` from your formatter can still reduce drift. This is a workaround, not the preferred long-term setup.

**dprint:** Add to `dprint.json`:

```json
{
	"excludes": ["**/*.t.md"]
}
```

**Prettier:** Add to `.prettierignore`:

```
*.t.md
```

#### Use ignore comments for especially formatter-sensitive blocks

If a formatter is still rewriting a specific target block in an undesirable way, use your formatter's ignore mechanism around that block where appropriate.

## CI integration issues

### `mdt` command not found

If your CI environment doesn't have mdt installed globally, install it first:

```yaml
- name: install mdt
  run: cargo install mdt_cli
```

Or run directly from your workspace without installing:

```yaml
- name: check docs
  run: cargo run --bin mdt -- check
```

The `cargo run` approach is slower (it compiles on every run) but avoids installation steps. For faster CI, cache the cargo install or use a pre-built binary.

### Check fails but works locally

**Common causes:**

- **Different working directory.** mdt resolves paths relative to where it's run. Use `--path` to be explicit:

  ```yaml
  - run: mdt check --path ./my-project
  ```

- **Files not checked out.** If your CI does a shallow clone, data files referenced in `mdt.toml` might be missing. Ensure a full checkout:

  ```yaml
  - uses: actions/checkout@v4
    with:
      fetch-depth: 0
  ```

- **Formatter ran after templates changed.** If CI runs `dprint fmt` before `mdt check`, the formatter might alter target content. Run `mdt update` after formatting, or exclude template content from the formatter.

### Recommended CI order

When both formatting and mdt checks are in your pipeline, run them in this order:

```yaml
- name: format
  run: dprint fmt

- name: sync templates
  run: mdt update

- name: verify everything is clean
  run: |
    mdt check
    git diff --exit-code
```

This ensures formatting and template sync are both applied, and the final `git diff` catches any uncommitted changes.

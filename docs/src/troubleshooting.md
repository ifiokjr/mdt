# Troubleshooting

Common errors, what mdt prints for them, and how to fix them.

`mdt check` exits `0` when every consumer is linked and current, `1` when a consumer is stale, orphaned, or fails to render (or a file drifted from its formatter output), and `2` when validation or config errors stop the check. Warnings never change the exit code. See the [CLI reference](./reference/cli.md) for every command's exit codes.

## Common errors

### Orphan consumer (no matching provider)

```text
Check failed.
  orphan consumers: 1

Orphan consumers:
  consumer `installguide` at readme.md:3:1 has no provider (did you mean `installGuide`?)

1 consumer block(s) have no provider. Rename those consumers or define the providers in `*.t.md` files.
```

**Cause:** A consumer's name matches no provider. `mdt check` fails (exit 1), `mdt update` prints a `warning:` and leaves the block untouched (reporting `All linked consumer blocks are already up to date.` when nothing else changed), and `mdt list` marks it `[orphan]`.

**Fix:**

- Use the suggested name. Names are case-sensitive: `installGuide` and `installguide` are different blocks.
- Make sure the provider lives in a `*.t.md` file. Providers anywhere else are ignored (see [Provider outside a template file](#provider-outside-a-template-file)).
- Check that the provider file is committed and not excluded by `.gitignore` or `[exclude] patterns`.
- In a monorepo, providers do not cross sub-project boundaries. Share them with `[templates] paths` (see [Monorepo setups](./advanced/monorepos.md)).

### Unused provider (warning)

```text
mdt::unused_provider

  ! [.templates/docs.t.md:1:1] provider block `installGuide` has no consumers
  help: add a consumer block `<!-- {=installGuide} -->...<!-- {/installGuide}
        -->`, remove the unused provider, or pass `--ignore-unused-blocks`
```

A provider with no consumers is a warning, not an error: `mdt check` still passes if everything else is current. Add a consumer, delete the provider, or pass `--ignore-unused-blocks` to silence it. Providers read from a `[templates] paths` directory outside the project (a shared library) are never reported as unused.

### Provider outside a template file

```text
mdt::provider_outside_template

  ! [readme.md:3:1] provider block `notes` is ignored because providers are
  | only read from `*.t.md` files
  help: move this block into a `*.t.md` template file, or reference an
        existing provider here with `<!-- {=notes} -->`
```

Providers (`{@name}`) are only read from `*.t.md` files. Move the block into a template file, or change the tag to a consumer (`{=name}`).

### Argument count mismatch

```text
Render errors:
  block `badges` at readme.md:6:1: argument count mismatch: provider `badges` declares 1 parameter(s), but consumer passes 2
```

The consumer passes a different number of arguments than the provider declares. This is a render error: `mdt check` exits 1 and `mdt update` skips that consumer, updates the rest, and exits 1. Match the `:"value"` segments on both tags. See [Block Arguments](./advanced/block-arguments.md).

### Duplicate provider

```text
mdt::duplicate_provider

  x duplicate provider `install`: defined in `.templates/api.t.md:1` and
  | `.templates/docs.t.md:1`
  help: each provider block name must be unique across the project
```

Two `*.t.md` files define the same provider name. Every command stops with exit 2 until you rename or merge one of them. Names only need to be unique within one project, so sibling sub-projects in a monorepo can reuse a name.

### Invalid tag

```text
mdt::invalid_tag

  x [readme.md:6:1] `<!-- {=my.block} -->` looks like an mdt tag but cannot be
  | parsed, so it is ignored
  help: write the sigil directly after `{` (`{@name}`, `{=name}`, `{~name}`,
        `{/name}`) and use a name matching `[A-Za-z_][A-Za-z0-9_-]*`; pass
        `--ignore-invalid-names` to skip this check
```

A markdown comment starts like a tag (`{` followed by `@`, `=`, `~`, or `/`) but does not parse. Common causes:

- A space between `{` and the sigil: `<!-- { =intro } -->`.
- A name with other characters: `my.block`, `my block`, `1starts`. Names match `[A-Za-z_][A-Za-z0-9_-]*`.

This is an error (exit 2). If the comment is not meant to be a tag, pass `--ignore-invalid-names`.

### Nested blocks

```text
mdt::nested_block

  x [readme.md:5:1] block `outro` is inside consumer `intro`; `mdt update`
  | would overwrite it
  help: move `outro` outside `intro`: everything between a consumer's tags is
        replaced by `mdt update`
```

`mdt update` replaces everything between a consumer's tags, so a block inside a consumer (or an inline block) would be destroyed. Move it out and place the consumers one after another.

### Unclosed or unmatched tags

```text
mdt::unclosed_block

  x [readme.md:3:1] missing closing tag for block `intro`
  help: add `<!-- {/intro} -->` to close this block, or, if a nearby closing
        tag has a different name, fix the misspelled tag so the names match
```

```text
mdt::unmatched_closing_tag

  x [readme.md:15:1] closing tag `{/stray}` has no matching opening tag
  help: if an opening tag nearby is misspelled, rename one of the two tags so
        they match; otherwise remove `<!-- {/stray} -->`
```

A missing closing tag and a closing tag with no opening tag are both errors (exit 2), in markdown and source files alike. Both usually mean one of the two tag names is misspelled. `--ignore-unclosed-blocks` lets the check continue past both; the unmatched tag then becomes a warning that only `--verbose` shows.

### A config key does nothing

Unknown keys in `mdt.toml` are rejected, so a typo stops every command with exit 2 instead of being silently ignored:

```text
mdt::config_parse

  x failed to parse config file: /path/to/project/mdt.toml: TOML parse error at line 2, column 1
  |   |
  | 2 | befor = 0
  |   | ^^^^^
  | unknown field `befor`, expected `before` or `after`
  |
  help: mdt.toml must be valid TOML using only supported keys (unknown keys
        are rejected to catch typos): max_file_size, disable_gitignore,
        [data], [padding], [check], [exclude], [include], [templates], and
        [[formatters]]
```

If a valid key still seems to have no effect, check the semantics in the [configuration reference](./reference/configuration.md). For example, `[include] patterns` only adds files to the default scan and never narrows it.

### Template variables are copied literally

```text
warning: provider block `install` in .templates/docs.t.md uses template variable(s) pkg.version, but this project has no `[data]`, so the text is copied without rendering; declare the namespace(s) under `[data]` in this project's mdt.toml
```

Providers are only rendered as templates when the project has a `[data]` section (or the block takes [arguments](./advanced/block-arguments.md)). Without it, `{{ pkg.version }}` lands in consumers verbatim. Declare the namespace:

```toml
[data]
pkg = "package.json"
```

In a monorepo, a sub-project that reads shared providers through `[templates] paths` needs its own `[data]`; the root's config does not apply. Paths may point outside the sub-project, such as `pkg = "../../package.json"`.

### Stale consumers after editing a provider

```text
Check failed.
  stale consumers: 2

Stale consumers:
  block `install` at readme.md:3:1
  block `install` at src/lib.rs:1:5

2 consumer block(s) are out of date. Run `mdt update`.
```

Run `mdt update`. While editing, `mdt update --watch` re-syncs on every change.

### A block is never discovered

Most mistakes now produce a diagnostic, but these cases are still silent:

- **Unscanned file type.** Only markdown and the [supported source extensions](./guide/source-files.md) are scanned. Add others with `[include] patterns = ["**/*.rb"]`.
- **Malformed tags in source files.** `mdt::invalid_tag` only checks markdown. In a code comment, `// <!-- {=my.block} -->` is ignored without a warning.
- **Tags in code.** In markdown, tags inside fenced code blocks and inline code spans are inert by design.
- **Skipped paths.** Hidden directories (such as `.github/`), gitignored files, `[exclude]` matches, and sub-projects with their own config are not scanned. Read providers from a hidden directory with `[templates] paths`.

Run `mdt list` after each fix to confirm the block is found.

## Debugging techniques

### `mdt list`

Lists every provider and consumer with its location, transformers, and link status:

```text
Providers:
  @install .templates/docs.t.md:1 (2 consumer(s))
  @usage .templates/docs.t.md:7 (0 consumer(s))

Consumers:
  =install readme.md:3 [linked]
  =usgae readme.md:6 [orphan]
  =install src/lib.rs:1 |trim|linePrefix:"//! ":true [linked]

2 provider(s), 3 consumer(s)
```

`[orphan]` marks a consumer with no provider (here a typo of `usage`), and `(0 consumer(s))` an unused provider. When files have validation errors (invalid, nested, or unclosed tags), `mdt list` prints the diagnostics, still prints the listing, and exits 2. Duplicate providers and config errors stop it before the listing.

### `mdt check --diff`

Shows a unified diff for each stale consumer so you can see whether the change is expected.

### `mdt update --dry-run`

Previews an update without writing anything:

```text
Dry run: would update 2 block(s) in 2 file(s):
  readme.md
  src/lib.rs
```

### `--verbose` and `MDT_LOG`

`--verbose` adds a scan summary (provider and consumer counts) and shows warnings that `--ignore-*` flags silenced. For internals, `MDT_LOG=debug mdt check` prints debug logs to stderr, including the config found and the number of files collected.

## Cache diagnostics

If scans look inconsistent (unexpected reparses, different results locally and in CI), run:

```sh
mdt info
mdt doctor
```

The `Cache` section of `mdt info` shows the artifact path and status, schema version, whether the project key matches, hash verification, reuse totals, and the last scan mode (`full cache hit` or `incremental reuse`). `mdt doctor` adds three checks: `Cache Artifact`, `Cache Hash Mode`, and `Cache Efficiency`.

To add content hashes to cache fingerprints while investigating:

```sh
MDT_CACHE_VERIFY_HASH=1 mdt check
```

The cache lives in `.mdt/`; add it to `.gitignore` (`mdt init` does this in git repositories). Deleting `.mdt/` is always safe.

## Formatter loops

**Symptom:** `mdt update`, then your formatter, then `mdt check` reports the same consumers as stale, every time. The formatter rewrites the generated text (table padding, indentation, wrapping) and mdt sees drift.

### Fix: configure `[[formatters]]`

Tell mdt to run the same formatter on each file it writes or checks, so both sides agree:

```toml
[[formatters]]
command = "prettier --stdin-filepath \"{{ filePath }}\""
patterns = ["**/*.md"]
ignore = ["**/*.t.md"]
```

The command reads the file on stdin and writes the formatted file to stdout. Keep placeholders in double quotes. Keep `*.t.md` out of scope both here and in the formatter's own config (`.prettierignore`, dprint `excludes`), because markdown formatters rewrite provider text. CI must install the same formatter versions you use locally.

<!-- {=mdtFormatterOnlyStaleDocs} -->

With formatters configured, `mdt check` can also report **stale files**: files that contain a consumer and that the formatter would change, even though every consumer block is current. `mdt update` rewrites the whole file, so the drift can be anywhere in it, not only inside a block. Run `mdt update` to normalize them. JSON output and MCP responses list these files in `stale_files`, separate from stale consumers in `stale`.

<!-- {/mdtFormatterOnlyStaleDocs} -->

### Nested source-file consumers

A consumer inside an `impl`, class, or other indented scope must include the indentation in its prefix, matching what your formatter produces. Otherwise mdt writes the lines at column 0 and the formatter re-indents them:

```text
impl Answer {
    /// <!-- {=method|trim|linePrefix:"    /// ":true} -->
    /// Returns the answer.
    /// <!-- {/method} -->
    pub fn get() -> u32 {
        42
    }
}
```

Alternatively, configure the formatter in `[[formatters]]` (for example `command = "rustfmt --edition 2021"` with `patterns = ["**/*.rs"]`), and mdt applies the indentation itself.

## CI issues

### `mdt` command not found

Install the prebuilt binary from npm and pin the version you use locally:

```yaml
- name: check docs
  run: npx -y @m-d-t/cli@0.9.5 check --format github
```

`cargo install mdt_cli` also works but builds from source, which is slow. See [CI Integration](./guide/ci-integration.md).

### Check passes locally but fails in CI

- **Uncommitted files.** mdt scans the working tree. A new `*.t.md`, data file, or consumer that exists locally but is not committed shows up in CI as an orphan consumer or a data error. Run `git status`.
- **Different project root.** Without `--path`, mdt walks up from the current directory, staying inside the git repository, to the nearest `mdt.toml`, `.mdt.toml`, or `.config/mdt.toml` (outside a git repository, or when there is none, it uses the current directory) and prints `note: using the mdt project at <path>` when that is not the current directory. If CI runs from a different directory than you do, pass `--path` explicitly. In a monorepo, run `mdt check --path <dir>` for each sub-project.
- **Different formatter versions.** With `[[formatters]]`, a different formatter version produces different output. Pin the same versions in CI.
- **Missing tools for data commands.** Script data sources run their `command` in CI too, so the tools they call must be installed.

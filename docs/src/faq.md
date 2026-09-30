# FAQ

## Can I use mdt with non-markdown files?

Yes. mdt scans source files for consumer tags inside code comments: Rust, TypeScript, JavaScript, Python, Go, Java, Kotlin, Swift, C/C++, C#, and Dart. The tags are the same as in markdown; they sit inside the file's comments.

Re-apply the comment prefix with `linePrefix`, or the synced lines are not comments and the file stops compiling:

```rust
//! <!-- {=packageDocs|trim|linePrefix:"//! ":true} -->
//! Documentation content injected here.
//! <!-- {/packageDocs} -->
```

See [Source File Support](./guide/source-files.md) for every language and its prefix.

## What happens if a provider is deleted?

Its consumers become **orphans**. mdt never clears or rewrites them:

- `mdt check` fails (exit 1) and lists each orphan with its `file:line:col` and a did-you-mean suggestion when a similar provider exists.
- `mdt update` prints a warning, leaves orphans untouched, and updates everything else.
- `mdt list` marks them `[orphan]`.

Restore the provider, or remove the consumer tags.

## Can multiple providers have the same name?

No. Provider names must be unique within a project, and a duplicate stops every command with exit 2:

```text
mdt::duplicate_provider

  x duplicate provider `install`: defined in `.templates/api.t.md:1` and
  | `.templates/docs.t.md:1`
  help: each provider block name must be unique across the project
```

In a monorepo, each sub-project (a directory with its own `mdt.toml`, `.mdt.toml`, or `.config/mdt.toml`) is a separate namespace, so two sub-projects can both define `{@install}`.

## How do I keep formatters from fighting mdt?

1. **Configure `[[formatters]]`** so mdt runs your formatter on every file it writes or checks. This is the real fix for `mdt update`, formatter, `mdt check` loops.
2. **Keep `*.t.md` out of the formatter's scope**, both in its own config and in the formatter entry's `ignore`, so provider text is never rewritten.
3. **Include indentation in the prefix** for consumers nested inside a class or `impl` (`linePrefix:"    /// ":true`), or the formatter re-indents them.

See [Troubleshooting: Formatter loops](./troubleshooting.md#formatter-loops).

## Can I use conditional logic in templates?

Yes, once `[data]` is configured. Provider content is then rendered with [minijinja](https://docs.rs/minijinja), which supports conditionals, loops, and filters. Without `[data]` (and without block arguments), `{{ ... }}` and `{% ... %}` stay literal text, and mdt warns when a used provider references a namespaced variable such as `{{ pkg.version }}`.

```toml
[data]
package = "package.json"
```

A conditional:

```text
<!-- {@install} -->

{% if not package.private -%}
npm install {{ package.name }}
{% endif %}
<!-- {/install} -->
```

A loop:

```text
<!-- {@keywords} -->

{% for keyword in package.keywords -%}
- {{ keyword }}
{% endfor %}
<!-- {/keywords} -->
```

The `-%}` trims the newline after a tag so the output has no stray blank lines. minijinja's built-in filters work too, for example `{{ package.name | upper }}`. See [Data Interpolation](./guide/data-interpolation.md).

## Can blocks be nested?

No. A block inside another block is the error `mdt::nested_block` (exit 2): `mdt update` replaces everything between a consumer's tags, and a provider's content — tags included — is copied into every consumer. Place consumers one after another instead:

```markdown
<!-- {=header} -->
<!-- {/header} -->

<!-- {=body} -->
<!-- {/body} -->
```

## Do tags affect rendered markdown?

No. Tags are HTML comments, which markdown renderers hide. Some tools show HTML comments verbatim in source-file docs (`go doc`, Python `help()`); rustdoc, dartdoc, and TSDoc hide them.

## Can I use mdt without a config file?

Yes. Without a config, mdt scans the project with the defaults: `*.t.md` files are providers, and markdown plus supported source files can hold consumers. Add `mdt.toml` (or `.mdt.toml`, `.config/mdt.toml`) when you need:

- `[data]` for template variables
- `[exclude]` to skip files, or `[include]` to add file types to the scan
- `[templates] paths` to read providers from extra directories, such as hidden ones
- `[padding]` to control blank lines around consumer content
- `[check]` for lenient comparison, or `[[formatters]]` for formatter integration

Unknown keys are rejected, so a typo is reported instead of ignored.

## How does mdt handle binary and large files?

The default scan only picks markdown and supported source extensions, and reads them as UTF-8 text. If an `[include]` pattern matches a binary file, the scan fails with `mdt::read_file` naming the file, so keep include globs to text extensions (`**/*.rb`, not `src/**`). A scanned file above `max_file_size` (default 10 MB) is the error `mdt::file_too_large`; exclude it or raise the limit.

## Can I run mdt on a subset of files?

No. mdt always scans the whole project so every consumer can find its provider. You can shape the project instead:

- `--path <dir>` runs mdt on a different project root, such as one sub-project in a monorepo. Without it, mdt uses the nearest directory with an mdt config, starting from the current one and walking up, so running inside a sub-project targets that sub-project.
- `[exclude] patterns` skips files and directories.
- `[include] patterns` and `[templates] paths` only add files to the scan; they never narrow it.

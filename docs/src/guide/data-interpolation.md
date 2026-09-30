# Data Interpolation

mdt can pull values from project files and commands into providers: `package.json`, `Cargo.toml`, YAML configs, a `VERSION` file, `git` output, and more. Version numbers, package names, and other metadata stay in one place and flow into your docs.

## Setup

Add a `[data]` section to `mdt.toml`. Each key becomes a namespace:

```toml
[data]
package = "package.json"
release = { path = "release-info", format = "json" }
version = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
```

- A string is a file path; the parser comes from the extension.
- `{ path, format }` names the parser explicitly, for files without a useful extension.
- `{ command, format, watch }` runs a command and parses its stdout. See [Script-backed data sources](#script-backed-data-sources).

Paths are relative to the project root. With this `package.json`:

```json
{
	"name": "my-lib",
	"version": "1.2.3",
	"description": "A great library"
}
```

a provider can use the values:

```text
<!-- {@install} -->

Install `{{ package.name }}` version {{ package.version }}:

    npm install {{ package.name }}@{{ package.version }}

{{ package.description }}.

<!-- {/install} -->
```

After `mdt update`, every `install` consumer contains:

```text
Install `my-lib` version 1.2.3:

    npm install my-lib@1.2.3

A great library.
```

## Supported formats

| Format                               | Extensions      | Notes                                                    |
| ------------------------------------ | --------------- | -------------------------------------------------------- |
| `json`                               | `.json`         |                                                          |
| `toml`                               | `.toml`         | Integers stay integers (`8080`, not `8080.0`)            |
| `yaml`, `yml`                        | `.yaml`, `.yml` |                                                          |
| `kdl`                                | `.kdl`          | Repeated nodes with the same name become an array        |
| `ini`                                | `.ini`          |                                                          |
| `text` (also `string`, `raw`, `txt`) | `.txt`          | The whole file as one string, minus one trailing newline |

Every format becomes the same nested structure, so you access values with dot notation whatever the source. A missing file, an unsupported format, or a failing command stops the run with exit status 2.

## Script-backed data sources

<!-- {=mdtScriptDataSourcesGuide} -->

A `[data]` entry can run a shell command and parse its stdout. Use it for values that come from tooling, such as git metadata, Nix, or a generated version file.

```toml
[data]
release = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
```

- `command`: shell command, run from the project root.
- `format`: parser for stdout: `text` (also `string`, `raw`, `txt`), `json`, `toml`, `yaml`, `yml`, `kdl`, or `ini`. Text drops one trailing newline, so `cat VERSION` renders `1.2.3`, not `1.2.3` plus a line break.
- `watch`: files whose changes invalidate the cached output.

<!-- {/mdtScriptDataSourcesGuide} -->

<!-- {=mdtScriptDataSourcesNotes} -->

- Output is cached in `.mdt/cache/data-v1.json`, keyed by namespace, command, format, and watch list. mdt reuses it while the watched files are unchanged.
- Caching needs every `watch` entry to be an existing file. Without `watch`, or when an entry is missing, a glob, or a directory, the command runs on every mdt run.
- A command that exits non-zero fails the run with `mdt::data_script` (exit status 2).

<!-- {/mdtScriptDataSourcesNotes} -->

## Examples

### TOML

```toml
# mdt.toml
[data]
cargo = "Cargo.toml"
```

```toml
# Cargo.toml
[package]
name = "my-crate"
version = "0.1.0"
edition = "2024"
```

```text
<!-- {@crateInfo} -->

**{{ cargo.package.name }}**: Rust edition {{ cargo.package.edition }}

<!-- {/crateInfo} -->
```

### YAML

```toml
# mdt.toml
[data]
config = "config.yaml"
```

```yaml
# config.yaml
app:
  name: My App
  port: 8080
features:
  - auth
  - logging
```

```text
<!-- {@appConfig} -->

{{ config.app.name }} runs on port {{ config.app.port }}.

<!-- {/appConfig} -->
```

### Several namespaces in one provider

Each namespace is independent, and one provider can use several:

```text
<!-- {@versions} -->

| Package | Version                     |
| ------- | --------------------------- |
| npm     | {{ package.version }}       |
| crate   | {{ cargo.package.version }} |

<!-- {/versions} -->
```

## Template syntax

Providers are rendered with [minijinja](https://docs.rs/minijinja), so its variables, conditionals, loops, and built-in filters are available.

Variables use dot notation:

```text
{{ namespace.key }}
{{ namespace.nested.deeply.value }}
```

Conditionals and loops:

```text
{% if package.private %}
This is a private package.
{% else %}
Available on npm.
{% endif %}

{%- for feature in config.features %}
- {{ feature }}
{%- endfor %}
```

Tags such as `{% if %}` leave their line breaks in the output. Use `{%-` and `-%}` to trim the whitespace around a tag, as in the loop above, which renders one list item per line.

Filters:

```text
{{ package.name | upper }}
{{ package.homepage | default("https://example.com") }}
{{ config.features | join(", ") }}
```

Only minijinja's built-in filters are available, such as `upper`, `lower`, `title`, `trim`, `replace`, `join`, `length`, and `default`. Other filters, such as `truncate`, are not available; an unknown filter is a render error for that provider.

An undefined variable renders as empty text, and `mdt check` and `mdt update` print a warning that names it.

## Literal braces

Once `[data]` is configured, **every** provider is rendered as a template, including ones that never mention your data. Text that looks like template syntax, such as GitHub Actions expressions or Handlebars examples, is then interpreted. Wrap it in a raw block:

```text
{% raw %}token: ${{ secrets.NPM_TOKEN }}{% endraw %}
```

This renders `token: ${{ secrets.NPM_TOKEN }}`.

## When rendering happens

Template variables are rendered before transformers run:

```text
Provider content
  → render {{ variables }} with minijinja
  → apply |transformers
  → replace consumer content
```

Transformers see the rendered text. If `{{ package.name }}` renders to `my-lib`, a `|trim` transformer trims the rendered result.

## No data, no rendering

Without a `[data]` section, mdt skips template rendering entirely and `{{ ... }}` text is copied unchanged. Projects that do not use data interpolation never need to escape braces.

If a provider that has consumers uses namespaced variables anyway, `mdt check` and `mdt update` warn:

```text
warning: provider block `a` in .templates/t.t.md uses template variable(s) pkg.version, but this project has no `[data]`, so the text is copied without rendering; declare the namespace(s) under `[data]` in this project's mdt.toml
```

This usually means a sub-project reuses shared providers without declaring the data they need. Add the namespaces to that project's `[data]`; paths may point outside the project, such as `pkg = "../../package.json"`.

## Inline interpolation patterns

Inline blocks render one value from your data in place, with no provider.

<!-- {=mdtInlineBlocksExamples} -->

### Inline value in prose

```markdown
Install version <!-- {~releaseVersion:"{{ pkg.version }}"} -->0.0.0<!-- {/releaseVersion} --> today.
```

### Inline value in a table cell

```markdown
| Package | Version                                                               |
| ------- | --------------------------------------------------------------------- |
| mdt     | <!-- {~mdtVersion:"{{ pkg.version }}"} -->0.0.0<!-- {/mdtVersion} --> |
```

### Inline value with a transformer

```markdown
CLI version: <!-- {~cliVersionCode:"{{ pkg.version }}"|code} -->`0.0.0`<!-- {/cliVersionCode} -->
```

### Inline value from a script-backed data source

```toml
[data]
release = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
```

```markdown
Release: <!-- {~releaseValue:"{{ release }}"} -->0.0.0<!-- {/releaseValue} -->
```

The text format drops the file's trailing newline, so the value stays on one line. While `VERSION` is unchanged, mdt reuses the cached output in `.mdt/cache/data-v1.json`.

<!-- {/mdtInlineBlocksExamples} -->

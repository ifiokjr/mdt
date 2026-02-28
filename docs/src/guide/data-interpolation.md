# Data Interpolation

mdt can pull values from project files — `package.json`, `Cargo.toml`, YAML configs, and more — into your templates. This means version numbers, package names, and other metadata stay in one place and flow into your documentation automatically.

## Setup

Add a `[data]` section to your `mdt.toml`:

```toml
[data]
package = "package.json"
release = { path = "release-info", format = "json" }
version = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
```

This maps the file `package.json` to the namespace `package`.

- String values are backward-compatible and infer format from extension.
- Typed values (`{ path, format }`) let you explicitly declare a format for files without extensions.
- Script values (`{ command, format, watch }`) execute commands and optionally cache stdout based on watched files.

If your `package.json` contains:

```json
{
	"name": "my-lib",
	"version": "1.2.3",
	"description": "A great library"
}
```

Then in your template files you can write:

```
<!-- {@install} -->

Install `{{ package.name }}` version {{ package.version }}:

  npm install {{ package.name }}@{{ package.version }}

{{ package.description }}.

<!-- {/install} -->
```

After `mdt update`, consumers of `install` will contain:

```
Install `my-lib` version 1.2.3:

  npm install my-lib@1.2.3

A great library.
```

## Supported data formats

| Format / Extension | Parser          |
| ------------------ | --------------- |
| `text`, `.txt`     | Raw text string |
| `json`, `.json`    | JSON            |
| `toml`, `.toml`    | TOML            |
| `yaml`, `.yaml`    | YAML            |
| `yml`, `.yml`      | YAML            |
| `kdl`, `.kdl`      | KDL             |
| `ini`, `.ini`      | INI             |

All formats are converted to a common structure internally. You access values using dot notation regardless of the source format.

## Script-backed data sources

<!-- {=mdtScriptDataSourcesGuide} -->

`[data]` entries can run shell commands and use stdout as template data. This is useful for values that come from tooling (for example Nix, git metadata, or generated version files).

```toml
[data]
release = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
```

- `command`: shell command executed from the project root.
- `format`: parser for stdout (`text`, `json`, `toml`, `yaml`, `yml`, `kdl`, `ini`).
- `watch`: files that control cache invalidation.

When `watch` files are unchanged, mdt reuses cached script output from `.mdt/cache/data-v1.json` instead of re-running the command.

<!-- {/mdtScriptDataSourcesGuide} -->

<!-- {=mdtScriptDataSourcesNotes} -->

- Script outputs are cached per namespace, command, format, and watch list.
- If `watch` is empty, mdt re-runs the script every load (no cache hit).
- A non-zero script exit status fails data loading with an explicit error.

<!-- {/mdtScriptDataSourcesNotes} -->

### TOML example

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

Template usage:

```
<!-- {@crateInfo} -->

**{{ cargo.package.name }}** — Rust edition {{ cargo.package.edition }}

<!-- {/crateInfo} -->
```

### YAML example

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

Template usage:

```
<!-- {@appConfig} -->

{{ config.app.name }} runs on port {{ config.app.port }}.

<!-- {/appConfig} -->
```

## Multiple data sources

You can map as many files as you need:

```toml
[data]
package = "package.json"
cargo = "Cargo.toml"
config = "config.yaml"
meta = "metadata.kdl"
```

Each namespace is independent. Use them together in the same template:

```
<!-- {@versions} -->

| Package | Version                     |
| ------- | --------------------------- |
| npm     | {{ package.version }}       |
| crate   | {{ cargo.package.version }} |

<!-- {/versions} -->
```

## Template syntax

mdt uses [minijinja](https://docs.rs/minijinja) for template rendering. The full minijinja syntax is available:

### Variables

```
{{ namespace.key }}
{{ namespace.nested.deeply.value }}
```

Undefined variables render as empty strings (mdt uses minijinja's "chainable" undefined behavior).

### Conditionals

```
{% if package.private %}
This is a private package.
{% else %}
Available on npm.
{% endif %}
```

### Loops

```
{% for feature in config.features %}
- {{ feature }}
{% endfor %}
```

### Filters

minijinja's built-in filters work alongside mdt's transformers:

```
{{ package.name | upper }}
{{ package.description | truncate(50) }}
```

## When rendering happens

Template variables are rendered **before** transformers are applied. The flow is:

```
Provider content
  → Render {{ variables }} via minijinja
  → Apply |transformers
  → Replace consumer content
```

This means transformers operate on the already-rendered content. For example, if `{{ package.name }}` renders to `my-lib`, then a `|trim` transformer trims the rendered result.

## No data, no rendering

If your project has no `mdt.toml` or no `[data]` section, template variable rendering is skipped entirely. Content containing `{{ }}` syntax passes through unchanged. This keeps mdt fully backwards-compatible for projects that don't need data interpolation.

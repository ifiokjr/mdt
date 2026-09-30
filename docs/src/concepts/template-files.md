# Template Files

Template files hold the providers: the single source of truth for shared content.

## Naming

Any file ending in `.t.md` is a template file (`t` for template):

```text
.templates/template.t.md
.templates/api.t.md
docs/shared.t.md
```

Only `*.t.md` files can define providers. A provider tag in any other file is ignored with a `mdt::provider_outside_template` warning, so there is one place to look for content definitions.

## Structure

A template file is regular markdown with one or more providers:

```markdown
<!-- {@installGuide} -->

Install the package:

    npm install my-lib

<!-- {/installGuide} -->

<!-- {@contributing} -->

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

<!-- {/contributing} -->
```

Text outside provider blocks is not distributed. Use it for notes about the templates.

## Template variables

When `mdt.toml` has a `[data]` section, provider content is rendered with [minijinja](https://docs.rs/minijinja) and can reference project data:

```markdown
<!-- {@installGuide} -->

Install `{{ package.name }}` version {{ package.version }}:

    npm install {{ package.name }}@{{ package.version }}

<!-- {/installGuide} -->
```

With `package = "package.json"` under `[data]`, `{{ package.version }}` becomes the version from `package.json`. Without `[data]`, provider content is copied as written, and `check` and `update` warn about each used provider that contains variables:

```text
warning: provider block `installGuide` in .templates/template.t.md uses template variable(s) package.name, package.version, but this project has no `[data]`, so the text is copied without rendering; declare the namespace(s) under `[data]` in this project's mdt.toml
```

See [Data Interpolation](../guide/data-interpolation.md).

## Where to put template files

`*.t.md` files anywhere in the project are providers, except in skipped locations such as hidden directories, `node_modules/`, and `target/`. `.templates/` is the one hidden directory that is scanned, and it is the recommended home:

```text
my-project/
  .templates/
    template.t.md
    api.t.md
  readme.md
```

A `templates/` directory or a single `template.t.md` at the root works too.

To read providers from somewhere the scan does not reach, such as another hidden directory or a shared directory outside a sub-project, list it under `[templates] paths`:

```toml
[templates]
paths = [".github/templates", "../../.templates"]
```

Each path is relative to the project root and may point outside it. mdt reads only the `*.t.md` files from listed directories; other files there are not treated as consumers. `paths` adds directories and never restricts where else providers are found. Providers read from a directory outside the project are never reported as unused. A listed path that is not a directory is a `mdt::templates_path` error.

## Multiple template files

A project can have any number of template files. Provider names must be unique across all of them; a second provider with the same name stops `check` and `update` with exit code 2:

```text
mdt::duplicate_provider

  x duplicate provider `installGuide`: defined in `.templates/api.t.md:3` and
  | `.templates/docs.t.md:1`
  help: each provider block name must be unique across the project
```

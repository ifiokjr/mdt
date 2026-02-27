# Template Files

Template files are the single source of truth for your shared content. They contain **provider blocks** that define the content distributed to consumers throughout your project.

## Naming convention

Template files use the `.t.md` extension:

```
template.t.md
docs.t.md
shared/api-docs.t.md
```

Any file ending in `.t.md` is treated as a template file. The `t` stands for "template."

Only `*.t.md` files can contain provider blocks. Provider tags (`{@name}`) in other files are ignored. This is intentional — it prevents accidental content injection from arbitrary files and gives you a clear place to look for content definitions.

## Structure

A template file is regular markdown containing one or more provider blocks:

```
<!-- {@installGuide} -->

Install the package:

  npm install my-lib

<!-- {/installGuide} -->

<!-- {@contributing} -->

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

<!-- {/contributing} -->
```

Content outside of provider blocks is ignored by mdt. You can use it for notes, organization, or documentation about the templates themselves.

## Template variables

Provider content can include [minijinja](https://docs.rs/minijinja) template variables that reference data from project files. This requires an mdt config file (`mdt.toml`, `.mdt.toml`, or `.config/mdt.toml`). See [Data Interpolation](../guide/data-interpolation.md) for details.

```
<!-- {@installGuide} -->

Install `{{ package.name }}` version {{ package.version }}:

  npm install {{ package.name }}@{{ package.version }}

<!-- {/installGuide} -->
```

When mdt renders this provider, `{{ package.name }}` and `{{ package.version }}` are replaced with actual values from `package.json` (or whichever file is mapped to the `package` namespace).

## Where to place template files

Template files can live anywhere in your project directory.

Canonical recommendation: use `.templates/` at the project root.

**Canonical layout (`.templates/`):**

```
my-project/
  .templates/
    template.t.md
    docs.t.md
  readme.md
```

**Compatible alternative (`templates/`):**

```
my-project/
  templates/
    docs.t.md
    examples.t.md
  readme.md
```

**Legacy single template at the root (still supported):**

```
my-project/
  template.t.md
  readme.md
```

You can also configure explicit template paths in `mdt.toml`:

```toml
[templates]
paths = ["shared/templates"]
```

## Multiple template files

A project can have multiple template files. Provider names must be unique across **all** template files. If two files define `{@installGuide}`, mdt reports an error:

```
error: duplicate provider `installGuide`: defined in `docs.t.md` and `api.t.md`
```

This ensures there's always one unambiguous source of truth for each piece of content.

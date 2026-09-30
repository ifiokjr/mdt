# Monorepo & Multi-Project Setups

In a monorepo, each package can be its own mdt project with its own providers, consumers, data, and configuration. The mechanism is **sub-project boundaries**.

## How sub-project boundaries work

Any directory below the project root that contains `mdt.toml`, `.mdt.toml`, or `.config/mdt.toml` is a separate project. The parent's scan skips it entirely. This applies at any depth, including direct children of the root.

```text
my-monorepo/
  mdt.toml                  # root project
  .templates/
    shared.t.md             # root providers
  readme.md                 # root consumers
  docs/
    .mdt.toml               # docs/ is a separate project
    readme.md
  packages/
    lib-a/
      mdt.toml              # lib-a is a separate project
      readme.md
    lib-b/
      .config/
        mdt.toml            # lib-b is a separate project
      readme.md
    lib-c/
      readme.md             # no config: part of the root project
```

Running `mdt update` at the root updates `readme.md` and `packages/lib-c/readme.md`, but nothing in `docs/`, `lib-a/`, or `lib-b/`. Run those with `--path`:

```sh
mdt update --path packages/lib-a
```

Without `--path`, mdt starts from the current directory and walks up, within the git repository, to the nearest directory with a config, like `git` and `cargo` do. Running `mdt check` inside `packages/lib-a/` therefore checks `lib-a`, and running it inside `packages/lib-c/` (no config) checks the root project.

## Setting up a sub-project

An empty config file is enough to create a boundary:

```sh
touch packages/lib-a/mdt.toml
```

Each project resolves its config, `*.t.md` files, and `[data]` paths relative to its own root. Provider names only need to be unique within one project, so `lib-a` and `lib-b` can both define `{@install}`.

```toml
# packages/lib-a/mdt.toml
[data]
package = "package.json" # packages/lib-a/package.json, not the root's
```

## Sharing providers across projects

A project cannot see another project's providers by default. To share them, point `[templates] paths` at the shared directory. Paths are relative to the sub-project and may leave it:

```toml
# packages/lib-a/mdt.toml
[templates]
paths = ["../../.templates"]
```

Now `lib-a`'s consumers can use every provider in `my-monorepo/.templates/`. [Block arguments](./block-arguments.md) fill in per-package values:

```markdown
<!-- .templates/shared.t.md -->

<!-- {@badge:"crate_name"} -->

[![crates.io](https://img.shields.io/crates/v/{{ crate_name }})](https://crates.io/crates/{{ crate_name }})

<!-- {/badge} -->
```

```markdown
<!-- packages/lib-a/readme.md -->

<!-- {=badge:"lib-a"} -->
<!-- {/badge} -->
```

Things to know:

- Only `*.t.md` files are read from a listed directory. Other files there, such as the root `readme.md`, are never treated as the sub-project's consumers.
- Keep shared template files provider-only. A consumer block inside a shared `*.t.md` file is checked and updated by every project that lists the directory.
- Providers from a directory outside the project are a shared library: the ones a sub-project does not use are never reported as unused.
- The sub-project needs its own `[data]` for any variables the shared providers use (see below).
- A listed path that is not a directory is an error (`mdt::templates_path`).

### Data for shared providers

A shared provider is rendered with the **consuming** project's data, and the root's `[data]` does not apply to sub-projects. If a shared provider uses `{{ package.version }}` and the sub-project has no `[data]`, the text is copied verbatim and mdt warns:

```text
warning: provider block `version` in /path/to/my-monorepo/.templates/shared.t.md uses template variable(s) package.version, but this project has no `[data]`, so the text is copied without rendering; declare the namespace(s) under `[data]` in this project's mdt.toml
```

Declare the namespace in the sub-project. Data paths may point outside it:

```toml
# packages/lib-a/mdt.toml
[templates]
paths = ["../../.templates"]

[data]
package = "../../package.json"
```

## CI checks

Run `mdt check` once per project:

```yaml
- name: check docs
  run: |
    mdt check
    mdt check --path docs
    mdt check --path packages/lib-a
    mdt check --path packages/lib-b
```

Or loop over `packages/*/`, failing if any check fails:

```sh
status=0
for dir in . packages/*/; do
  if [ "$dir" = . ] || [ -f "$dir/mdt.toml" ] || [ -f "$dir/.mdt.toml" ] || [ -f "$dir/.config/mdt.toml" ]; then
    echo "Checking $dir"
    mdt check --path "$dir" || status=1
  fi
done
exit $status
```

Output paths are relative to each project's root. With `--format github`, this means inline annotations for a sub-project can point at the wrong file; see [CI Integration](../guide/ci-integration.md#annotations).

# Monorepo & Multi-Project Setups

mdt supports monorepos where each package manages its own templates independently. The key mechanism is **sub-project boundaries**: any directory containing its own `mdt.toml` is treated as a separate mdt project.

## How sub-project boundaries work

When mdt scans a directory tree, it stops descending into any subdirectory that contains an `mdt.toml` file. That subdirectory becomes its own isolated scope with its own providers, consumers, data files, and configuration.

```
my-monorepo/
  mdt.toml              # root project
  template.t.md         # root providers
  readme.md             # root consumers
  packages/
    lib-a/
      mdt.toml          # lib-a is a separate project
      template.t.md     # lib-a providers
      readme.md         # lib-a consumers
    lib-b/
      mdt.toml          # lib-b is a separate project
      template.t.md     # lib-b providers
      readme.md         # lib-b consumers
    lib-c/
      readme.md         # NO mdt.toml — belongs to root project
```

Running `mdt update` from the monorepo root updates consumers in `readme.md` and `packages/lib-c/readme.md`, but **not** in `packages/lib-a/` or `packages/lib-b/`. Those are separate projects.

To update `lib-a`, run `mdt update` from inside `packages/lib-a/`, or use the `--path` flag:

```sh
mdt update --path packages/lib-a
```

## Setting up a monorepo

### Step 1: Create an `mdt.toml` in each package

Each package that needs its own template scope gets an `mdt.toml`. Even an empty file is enough to establish a boundary:

```toml
# packages/lib-a/mdt.toml
```

Add configuration as needed:

```toml
# packages/lib-a/mdt.toml
[data]
cargo = "Cargo.toml"
```

### Step 2: Create template files per package

Each sub-project has its own `*.t.md` files with its own provider blocks:

```
<!-- packages/lib-a/template.t.md -->

<!-- {@install} -->

cargo add lib-a

<!-- {/install} -->
```

```
<!-- packages/lib-b/template.t.md -->

<!-- {@install} -->

cargo add lib-b

<!-- {/install} -->
```

Provider names only need to be unique **within** a project scope. Both `lib-a` and `lib-b` can have an `{@install}` provider without conflict.

### Step 3: Run updates per package or use a script

Update each package individually:

```sh
mdt update --path packages/lib-a
mdt update --path packages/lib-b
```

Or use a script to update all packages:

```sh
#!/bin/sh
for dir in packages/*/; do
  if [ -f "$dir/mdt.toml" ]; then
    mdt update --path "$dir"
  fi
done
```

## Shared templates across packages

Sub-project boundaries are strict. A provider in the root `template.t.md` is **not visible** to consumers inside `packages/lib-a/`. Each scope is fully isolated.

If you need shared content across packages, you have a few options:

### Option 1: Use block arguments for parameterized content

Define a parameterized provider at the root level and use it for files that belong to the root scope:

```
<!-- {@badge:"crate_name"} -->

[![crates.io](https://img.shields.io/crates/v/{{ crate_name }})](https://crates.io/crates/{{ crate_name }})

<!-- {/badge} -->
```

For sub-projects, duplicate the provider in each sub-project's template file. This is intentional — each project is self-contained.

### Option 2: Duplicate providers where needed

Copy the provider block into each sub-project's template file. While this creates duplication in template files, the consumer blocks throughout each project stay in sync with their local provider — which is mdt's primary guarantee.

### Option 3: Keep shared content at the root scope

If files consuming shared content don't live inside a sub-project directory, they can all reference the root-level providers. Structure your project so that shared docs live outside sub-project boundaries.

## CI checks in a monorepo

Run `mdt check` for each sub-project in CI:

```yaml
- name: check root docs
  run: mdt check

- name: check lib-a docs
  run: mdt check --path packages/lib-a

- name: check lib-b docs
  run: mdt check --path packages/lib-b
```

Or iterate over all directories that contain `mdt.toml`:

```yaml
- name: check all mdt projects
  run: |
    for dir in . packages/*/; do
      if [ -f "$dir/mdt.toml" ]; then
        echo "Checking $dir"
        mdt check --path "$dir"
      fi
    done
```

## Data isolation

Each sub-project loads its own data files relative to its own `mdt.toml`. A `[data]` section in `packages/lib-a/mdt.toml` resolves paths relative to `packages/lib-a/`:

```toml
# packages/lib-a/mdt.toml
[data]
cargo = "Cargo.toml" # resolves to packages/lib-a/Cargo.toml
package = "package.json" # resolves to packages/lib-a/package.json
```

This means `{{ cargo.package.name }}` in `lib-a`'s templates refers to `lib-a`'s `Cargo.toml`, not the root workspace `Cargo.toml`.

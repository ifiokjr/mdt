# Configuration

mdt is configured through an `mdt.toml` file placed in the project root. Configuration is optional — mdt works without it using sensible defaults.

## Creating a config file

Create `mdt.toml` in your project root:

```toml
[data]
package = "package.json"

[exclude]
patterns = ["vendor/**", "dist/**"]
```

## Sections

### `[data]` — Data file mappings

Maps namespace names to data files. Each entry makes the file's contents available as template variables under that namespace.

```toml
[data]
package = "package.json"
cargo = "Cargo.toml"
config = "config.yaml"
```

This creates three namespaces:

- `{{ package.name }}` reads from `package.json`
- `{{ cargo.package.version }}` reads from `Cargo.toml`
- `{{ config.database.host }}` reads from `config.yaml`

Paths are relative to the project root (where `mdt.toml` lives).

**Supported formats:** JSON, TOML, YAML (`.yaml`/`.yml`), and KDL.

See [Data Interpolation](./data-interpolation.md) for full details.

### `[exclude]` — Exclude patterns

Glob patterns for files and directories to skip during scanning:

```toml
[exclude]
patterns = [
	"vendor/**",
	"dist/**",
	"generated/**",
	"**/*.generated.md",
]
```

These patterns are checked relative to the project root. In addition to your explicit patterns, mdt always skips hidden directories (`.git`, `.vscode`, etc.), `node_modules/`, and `target/`.

### `[include]` — Include patterns

Restrict scanning to only files matching these patterns:

```toml
[include]
patterns = ["docs/**/*.rs", "src/**/*.ts"]
```

When set, only files matching at least one include pattern are scanned (in addition to markdown and template files which are always included).

### `[templates]` — Template search paths

By default, mdt finds `*.t.md` files anywhere in the project. You can restrict where it looks:

```toml
[templates]
paths = ["templates", "shared/docs"]
```

When set, only `*.t.md` files within these directories are recognized as template files.

### `pad_blocks` — Block content padding

When enabled, mdt ensures a blank line separates the opening tag from the content and the content from the closing tag. In source code files, the extra blank lines use the same comment prefix as the surrounding lines (e.g., `//!`, `///`, `*`). This is recommended when using consumer blocks in source code files.

```toml
pad_blocks = true
```

Without this setting, transformers like `trim` can cause content to merge directly into the surrounding tags, breaking the structure of code comments.

If omitted, defaults to `false`.

### `max_file_size` — Safety limit for scanned files

Set the maximum file size (in bytes) that mdt will scan. Files larger than this limit return an error.

```toml
max_file_size = 10485760 # 10 MB
```

If omitted, mdt uses a default of `10 MB`.

## Sub-project boundaries

If mdt encounters a directory containing its own `mdt.toml`, it treats that directory as a separate project and skips it. This is useful in monorepos where each package manages its own templates:

```
my-monorepo/
  mdt.toml              # root project config
  template.t.md
  packages/
    lib-a/
      mdt.toml          # lib-a is a separate mdt project
      template.t.md
    lib-b/
      mdt.toml          # lib-b is a separate mdt project
      template.t.md
```

Running `mdt update` from the root updates only the root project's consumers. Each sub-project is managed independently.

## Full example

```toml
# mdt.toml

# Ensure proper padding between tags and content in source files
pad_blocks = true

[data]
package = "package.json"
cargo = "crates/my-lib/Cargo.toml"

[exclude]
patterns = ["vendor/**", "dist/**", "*.generated.md"]

[include]
patterns = ["src/**"]

[templates]
paths = ["templates"]

max_file_size = 10485760
```

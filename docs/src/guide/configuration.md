# Configuration

mdt is configured through an `mdt.toml` file placed in the project root. Configuration is optional — mdt works without it using sensible defaults.

## Creating a config file

Create `mdt.toml` in your project root:

```toml
[data]
package = "package.json"

[exclude]
patterns = ["vendor/", "dist/"]
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

Patterns for files and directories to skip during scanning. Uses **gitignore-style syntax** — the same pattern format as `.gitignore` files, including negation (`!`), directory markers (`/`), wildcards (`*`, `**`), and character classes.

```toml
[exclude]
patterns = [
	"vendor/",
	"dist/",
	"generated/",
	"**/*.generated.md",
	"!generated/keep-this.md",
]
```

These patterns are checked relative to the project root. In addition to your explicit patterns, mdt always skips hidden directories (`.git`, `.vscode`, etc.), `node_modules/`, and `target/`.

#### `markdown_codeblocks` — Skip tags in code blocks

Controls whether mdt tags inside fenced code blocks in **source-file comments** are processed. This is useful when doc comments contain fenced examples that show mdt tag syntax but should not be treated as real tags.

```toml
[exclude]
# Skip tags inside ALL fenced code blocks
markdown_codeblocks = true

# Skip only code blocks whose info string contains "ignore"
markdown_codeblocks = "ignore"

# Skip code blocks whose info string contains any of these
markdown_codeblocks = ["ignore", "example"]
```

The default is `false`, meaning tags in fenced source-comment code blocks are processed normally.

#### `blocks` — Exclude specific block names

Array of block names to exclude. Any block (provider or consumer) whose name appears in this list is completely ignored.

```toml
[exclude]
blocks = ["draft-section", "deprecated-api"]
```

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

### `[padding]` — Block content padding

Controls blank lines between block tags and their content. This is recommended when using consumer blocks in source code files.

```toml
[padding]
before = 0
after = 0
```

`before` and `after` accept `false` (inline), `0` (next line), `1` (one blank line), `2`, etc. When `[padding]` is present but values are omitted, they default to `1`. In source code files, blank lines use the same comment prefix as surrounding lines (e.g., `//!`, `///`, `*`).

Without this setting, transformers like `trim` can cause content to merge directly into the surrounding tags, breaking the structure of code comments.

**Recommended for projects with formatters:** Use `before = 0, after = 0` to minimize whitespace that formatters might alter.

### `max_file_size` — Safety limit for scanned files

Set the maximum file size (in bytes) that mdt will scan. Files larger than this limit return an error.

```toml
max_file_size = 10485760 # 10 MB
```

If omitted, mdt uses a default of `10 MB`.

### `disable_gitignore` — Disable `.gitignore` integration

By default, mdt respects `.gitignore` rules when scanning for files, skipping anything that git would ignore. Set `disable_gitignore = true` to turn off this behavior:

```toml
disable_gitignore = true
```

When this option is enabled, mdt scans all files regardless of `.gitignore` rules. You can still control which files are scanned using the `[exclude]` and `[include]` sections.

**When to use this:**

- **Generated files with mdt blocks** — If your build output or generated files contain consumer blocks that need updating, those files are typically listed in `.gitignore` but still need to be scanned by mdt.
- **Working outside a git repository** — If the project is not a git repo, `.gitignore` resolution can cause unnecessary overhead or errors. Disabling it avoids those issues.
- **Full control over scanning** — When you prefer to manage file inclusion/exclusion entirely through `[exclude]` and `[include]` patterns rather than relying on `.gitignore`.

If omitted, defaults to `false` (`.gitignore` rules are respected).

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

max_file_size = 10485760
disable_gitignore = false

# Ensure content is properly separated from tags in source files
[padding]
before = 0
after = 0

[data]
package = "package.json"
cargo = "crates/my-lib/Cargo.toml"

[exclude]
patterns = ["vendor/", "dist/", "*.generated.md"]
blocks = ["draft-section"]
# Applies to fenced code blocks inside source-file comments.
markdown_codeblocks = true

[include]
patterns = ["src/**"]

[templates]
paths = ["templates"]
```

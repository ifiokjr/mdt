# FAQ

## Can I use mdt with non-markdown files?

Yes. mdt scans source code files for target tags inside code comments. Supported languages include Rust, TypeScript, JavaScript, Python, Go, Java, Kotlin, Swift, C/C++, and C#. The target tag syntax (`<!-- {=name} -->` / `<!-- {/name} -->`) is the same — it just appears within the file's comment syntax.

For example, in a Rust file:

```rust
//! <!-- {=packageDocs|trim} -->
//! Documentation content injected here.
//! <!-- {/packageDocs} -->
```

See [Source File Support](./guide/source-files.md) for the full list of languages and examples.

## What happens if a source is deleted?

Consumers referencing the deleted source become **orphaned**. Their content is left unchanged — mdt does not clear or modify orphaned targets.

- `mdt check` warns about orphaned targets.
- `mdt list` shows orphaned targets with the `[orphan]` status.
- `mdt update` skips orphaned targets and proceeds with the rest.

To fix orphaned targets, either restore the source or remove the target tags from the files that referenced it.

## Can multiple sources have the same name?

No. Source names must be unique within a project scope. If two `*.t.md` files define a source with the same name, mdt reports an error:

```
error: duplicate source `install`: defined in `docs.t.md` and `api.t.md`
```

In a monorepo, source names only need to be unique within each sub-project (each directory with its own `mdt.toml`). Two different sub-projects can both have an `{@install}` provider without conflict.

## How do I keep formatters from mangling template content?

Formatters can interfere with mdt by reformatting content inside target blocks. The main strategies are:

1. **Exclude `*.t.md` files** from your formatter so the source-of-truth content is never altered.
2. **Use ignore comments** (e.g., `<!-- dprint-ignore -->`) before target blocks in markdown files.
3. **Set `[padding]`** in `mdt.toml` to control whitespace precisely, reducing formatter conflicts.
4. **Match transformer output** to what the formatter expects (e.g., use the same indentation style).

See [Troubleshooting > Formatter interference](./troubleshooting.md#formatter-interference) for detailed solutions.

## Can I use conditional logic in templates?

Yes. mdt uses [minijinja](https://docs.rs/minijinja) for template rendering, which supports conditionals, loops, and filters.

### Conditionals

```
<!-- {@platformInstall} -->

{% if cargo.package.name %}
cargo add {{ cargo.package.name }}
{% endif %}

{% if package.name %}
npm install {{ package.name }}
{% endif %}

<!-- {/platformInstall} -->
```

### Loops

```
<!-- {@featureList} -->

{% for feature in config.features %}
- {{ feature }}
{% endfor %}

<!-- {/featureList} -->
```

### Filters

minijinja's built-in filters work in source content:

```
{{ package.name | upper }}
{{ package.description | truncate(80) }}
```

See [Data Interpolation](./guide/data-interpolation.md) for full details on template syntax.

## Can targets appear inside other targets?

No. mdt does not support nested blocks. Each target block is a flat, non-overlapping region. If you need to compose content, define separate providers and place their consumers sequentially:

```markdown
<!-- {=header} -->
<!-- {/header} -->

<!-- {=body} -->
<!-- {/body} -->

<!-- {=footer} -->
<!-- {/footer} -->
```

## Do tags affect rendered markdown?

No. mdt tags are HTML comments (`<!-- ... -->`), which are invisible when markdown is rendered to HTML. Readers of your documentation never see the template machinery.

## Can I use mdt without a config file?

Yes. `mdt.toml` is optional. Without it, mdt still scans for `*.t.md` template files and processes provider/target blocks. You only need a config file for:

- Data interpolation (`[data]` section)
- Custom exclude/include patterns
- Template search path restrictions
- Block padding configuration

## How does mdt handle binary files?

mdt only scans text files with recognized extensions (`.md`, `.mdx`, `.markdown`, `.t.md`, and supported source code extensions). Binary files and unrecognized file types are ignored. A `max_file_size` limit (default 10 MB) prevents accidentally reading very large files.

## Can I run mdt on a subset of files?

Not directly — mdt always scans the full project to build the source map. However, you can control the scan scope:

- Use `--path` to target a specific sub-project directory.
- Use `[include]` patterns in `mdt.toml` to restrict which source files are scanned.
- Use `[exclude]` patterns to skip specific files or directories.
- Use `[templates] paths` to limit where mdt looks for `*.t.md` files.

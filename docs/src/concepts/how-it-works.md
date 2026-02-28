# How mdt Works

mdt follows a straightforward pipeline: scan your project for template tags, match providers to consumers, render any template variables, apply transformers, and replace content.

## The Pipeline

```
1. Scan project directory
   ├── Find *.t.md files → extract provider blocks
   ├── Find *.md files → extract consumer blocks
   └── Find source files (.rs, .ts, .py, ...) → extract consumer blocks from comments

2. Load configuration (mdt.toml)
   └── Read data files (package.json, Cargo.toml, ...) into template context

3. For each consumer:
   ├── Find its matching provider by name
   ├── Render template variables in provider content ({{ package.version }})
   ├── Apply transformers (|trim|indent:"  ")
   └── Replace the consumer's content if it differs
```

## Tag anatomy

All mdt tags live inside HTML comments. This means they're invisible when markdown is rendered — your docs look clean to readers.

A tag has three parts:

```
<!-- {sigil name | transformers} -->
       │      │    │
       │      │    └── Optional: pipe-delimited content filters
       │      └─────── The block name
       └────────────── @ provider, = consumer, ~ inline, / close
```

## File conventions

mdt determines how to treat files based on their names:

| Pattern                              | Role                                                           |
| ------------------------------------ | -------------------------------------------------------------- |
| `*.t.md`                             | **Template files** — only these can contain provider blocks    |
| `*.md`, `*.mdx`, `*.markdown`        | **Markdown files** — scanned for consumer and inline blocks               |
| `*.rs`, `*.ts`, `*.py`, `*.go`, etc. | **Source files** — scanned for consumer and inline blocks inside comments |

Provider blocks found in non-template files are ignored. This prevents accidental content injection from arbitrary files.

## What gets skipped

The scanner automatically ignores:

- Hidden directories (starting with `.`)
- `node_modules/`
- `target/` (Rust build output)
- Directories with their own `mdt.toml` (treated as separate projects)
- Files matching gitignore-style patterns in the `[exclude]` config section
- Blocks whose names appear in `[exclude] blocks`
- Tags inside fenced code blocks when `[exclude] markdown_codeblocks` is configured

## Matching rules

- Each provider name must be **unique** across all template files. Duplicate names produce an error.
- A consumer references a provider by name. If no matching provider exists, mdt emits a warning but continues.
- Multiple consumers can reference the same provider. They all receive the same content (after their own transformers are applied).
- A single file can contain multiple consumer blocks.

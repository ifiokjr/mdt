# How mdt Works

mdt follows a straightforward pipeline: scan your project for template tags, match sources to targets, render any template variables, apply transformers, and replace content.

## The Pipeline

```
1. Scan project directory
   ├── Find *.t.md files → extract source blocks
   ├── Find *.md files → extract target blocks
   └── Find source files (.rs, .ts, .py, ...) → extract target blocks from comments

2. Load configuration (mdt.toml)
   └── Read data files (package.json, Cargo.toml, ...) into template context

3. For each target:
   ├── Find its matching source by name
   ├── Render template variables in source content ({{ package.version }})
   ├── Apply transformers (|trim|indent:"  ")
   └── Replace the target's content if it differs
```

## Tag anatomy

All mdt tags live inside HTML comments. This means they're invisible when markdown is rendered — your docs look clean to readers.

A tag has three parts:

```
<!-- {sigil name | transformers} -->
       │      │    │
       │      │    └── Optional: pipe-delimited content filters
       │      └─────── The block name
       └────────────── @ source, = target, ~ inline, / close
```

## File conventions

mdt determines how to treat files based on their names:

| Pattern                              | Role                                                                      |
| ------------------------------------ | ------------------------------------------------------------------------- |
| `*.t.md`                             | **Template files** — only these can contain source blocks               |
| `*.md`, `*.mdx`, `*.markdown`        | **Markdown files** — scanned for target and inline blocks               |
| `*.rs`, `*.ts`, `*.py`, `*.go`, etc. | **Source files** — scanned for target and inline blocks inside comments |

Source blocks found in non-template files are ignored. This prevents accidental content injection from arbitrary files.

## What gets skipped

The scanner automatically ignores:

- Hidden directories (starting with `.`)
- `node_modules/`
- `target/` (Rust build output)
- Directories with their own `mdt.toml` (treated as separate projects)
- Files matching gitignore-style patterns in the `[exclude]` config section
- Blocks whose names appear in `[exclude] blocks`
- Tags inside fenced code blocks in source-file comments when `[exclude] markdown_codeblocks` is configured

## Matching rules

- Each source name must be **unique** across all template files. Duplicate names produce an error.
- A target references a source by name. If no matching source exists, mdt emits a warning but continues.
- Multiple targets can reference the same source. They all receive the same content (after their own transformers are applied).
- A single file can contain multiple target blocks.

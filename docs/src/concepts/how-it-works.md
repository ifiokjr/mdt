# How mdt Works

mdt finds named blocks in your project, matches each consumer to its provider, and rewrites the consumer's content when it differs.

## The pipeline

```text
1. Find the project root and load its configuration (optional)
   └── Read [data] sources (package.json, Cargo.toml, scripts, ...)

2. Scan the project
   ├── *.t.md files            → provider blocks
   ├── markdown files          → consumer and inline blocks
   └── source files (.rs, ...) → consumer and inline blocks in comments

3. Validate: unclosed, unmatched, nested, and invalid tags, unknown
   transformers, duplicate providers, orphan consumers, unused providers

4. For each consumer:
   ├── Find the provider with the same name
   ├── Render template variables ({{ pkg.version }}) when [data] is set
   ├── Apply the consumer's transformers (|trim|linePrefix:"/// ":true)
   └── Replace the content between the tags if it differs
```

`mdt update` writes the result. `mdt check` runs the same steps without writing and fails if any file would change.

## Project root

With `--path <dir>`, that directory is the project root. Without it, every command except `mdt init` walks up from the current directory, inside the enclosing git repository, to the nearest directory containing `mdt.toml`, `.mdt.toml`, or `.config/mdt.toml`, so you can run mdt from any subdirectory. Outside a git repository, or with no config found, the current directory is the root.

## Tag anatomy

Every tag is an HTML comment, so readers of the rendered markdown never see it.

```text
<!-- {=name|trim|codeBlock:"sh"} -->
      ││    └─────────────────── optional transformers, separated by |
      │└──────────────────────── block name
      └───────────────────────── sigil: @ provider, = consumer, ~ inline, / close
```

The sigil must directly follow `{`. See the [Template Syntax Reference](../reference/template-syntax.md) for the full rules.

## File roles

| Files                                      | Role                                                   |
| ------------------------------------------ | ------------------------------------------------------ |
| `*.t.md`                                   | Template files: the only place providers are read from |
| `*.md`, `*.mdx`, `*.markdown`              | Scanned for consumer and inline blocks                 |
| Source files (`*.rs`, `*.ts`, `*.py`, ...) | Scanned for consumer and inline blocks inside comments |

A provider tag outside a `*.t.md` file is ignored with a `mdt::provider_outside_template` warning. See [Source File Support](../guide/source-files.md) for the full extension list.

## What gets skipped

- Files ignored by git: `.gitignore` files in the project and its parent directories up to the repository root, plus `.git/info/exclude`. Outside a git repository, only the `.gitignore` files inside the project apply. `disable_gitignore = true` turns this off.
- Hidden directories other than `.templates/`, plus `node_modules/` and `target/`, even with `disable_gitignore`. List other directories under `[templates] paths` to read their providers.
- Subdirectories with their own `mdt.toml`, `.mdt.toml`, or `.config/mdt.toml`. Each is a separate project.
- Files with extensions mdt does not scan, unless added with `[include]`.
- Files matching `[exclude] patterns`, and blocks named in `[exclude] blocks` (their consumers are never filled or checked).
- Tags inside fenced code blocks and inline code spans in markdown, and inside fenced code blocks in source comments when `[exclude] markdown_codeblocks` is set.

## Matching rules

- Provider names are unique across the project. Two providers with the same name are a `mdt::duplicate_provider` error.
- A consumer whose name matches no provider is an orphan. `mdt check` fails and lists it with a suggestion; `mdt update` warns and leaves it unchanged:

  ```text
  consumer `instalGuide` at readme.md:3:1 has no provider (did you mean `installGuide`?)
  ```

- A provider with no consumers is a `mdt::unused_provider` warning.
- Any number of consumers can use one provider, and a file can contain any number of consumers. Each consumer applies its own transformers.

# Introduction

**mdt** (manage **m**ark**d**own **t**emplates) is a tool that keeps documentation synchronized across your project. Define content once in a template file, reference it from anywhere — READMEs, code comments, mdbook docs — and mdt keeps everything in sync.

## The Problem

Documentation gets duplicated. Your README has installation instructions. Your library's doc comment has the same instructions. Your mdbook repeats them again. When something changes, you update one place and forget the others. They drift apart. Users find conflicting information.

This happens constantly in real projects:

- A CLI usage section repeated in the root README and the crate README
- API documentation duplicated between doc comments and a docs site
- Version numbers, package names, or install commands scattered across multiple files
- Code examples that appear in both documentation and source files

Manual synchronization doesn't scale. Copy-pasting is error-prone. The larger the project, the worse it gets.

## The Solution

mdt uses HTML comments as invisible template tags. You define content once in a **provider** block inside a template file. Then you place **consumer** tags wherever that content should appear. Running `mdt update` replaces the content between consumer tags with the provider's content.

```markdown
<!-- In template.t.md (the provider) -->
<!-- {@install} -->

npm install my-lib

<!-- {/install} -->
```

```markdown
<!-- In readme.md (a consumer) -->

## Installation

<!-- {=install} -->

This content gets replaced automatically.

<!-- {/install} -->
```

After running `mdt update`, every consumer named `install` has identical content — sourced from the single provider definition.

## Key Features

- **Comment-based tags** — HTML comments are invisible in rendered markdown, so your docs look clean
- **Data interpolation** — Pull values from `package.json`, `Cargo.toml`, or any data file into your templates using `{{ variable }}` syntax
- **Source file support** — Consumer tags work inside code comments too (Rust, TypeScript, Python, Go, and more)
- **Transformers** — Pipe content through filters like `trim`, `indent`, `prefix`, `codeBlock` to adapt it for each context
- **CI-friendly** — `mdt check` exits non-zero when docs are stale, with JSON and GitHub Actions output formats
- **Watch mode** — `mdt update --watch` auto-syncs on file changes during development
- **LSP support** — Language server for editor integration with diagnostics, completions, and hover

# Why mdt?

Documentation drift is a familiar problem, and there are several ways to address it. This page explains where mdt fits and when it's the right choice.

## vs. Copy-paste

The simplest approach: copy content between files by hand.

|                    | Copy-paste      | mdt                                              |
| ------------------ | --------------- | ------------------------------------------------ |
| Setup effort       | None            | Minimal (`mdt init`)                             |
| Drift detection    | Manual review   | `mdt check` in CI                                |
| Sync effort        | Edit every copy | Edit one source block, run `mdt update`          |
| Cross-surface      | Manual          | README + source docs + docs site from one source |
| Scales with copies | Gets worse      | Stays constant                                   |

Copy-paste works for a single duplication. Once the same content lives in three or more places, manual synchronization becomes the dominant maintenance cost.

## vs. Docs framework includes

Frameworks like mdBook, Docusaurus, and MkDocs support file includes or content transclusion within their own ecosystem.

|                              | Framework includes | mdt                            |
| ---------------------------- | ------------------ | ------------------------------ |
| Works in README.md           | No                 | Yes                            |
| Works in source-doc comments | No                 | Yes                            |
| Works across frameworks      | No                 | Yes                            |
| Works without a build step   | Sometimes          | Yes (tags are HTML comments)   |
| Data interpolation           | Framework-specific | Built-in (`{{ pkg.version }}`) |
| CI verification              | Framework-specific | `mdt check` exits non-zero     |

Framework includes solve the problem within one surface. mdt solves it across surfaces — your README, your crate/package docs, and your docs site all stay in sync from the same source blocks.

## vs. Custom scripts

A common approach is writing a script that reads a source file and injects content into targets.

|                    | Custom script         | mdt                                                     |
| ------------------ | --------------------- | ------------------------------------------------------- |
| Maintenance        | You maintain it       | Community-maintained                                    |
| Declarative        | No — imperative logic | Yes — tag-based                                         |
| Caching            | You build it          | Built-in (file fingerprinting)                          |
| Editor support     | None                  | LSP (diagnostics, completions, hover, go-to-definition) |
| AI integration     | None                  | MCP server for assistants                               |
| Watch mode         | You build it          | `mdt update --watch`                                    |
| Transformers       | You build them        | Built-in (`trim`, `indent`, `linePrefix`, etc.)         |
| Data interpolation | You build it          | Built-in (JSON, TOML, YAML, KDL, INI, scripts)          |

Scripts work when you have one specific use case. mdt provides the same capability as a general-purpose tool with editor integration, CI support, and a growing feature set.

## vs. Template engines (Tera, Handlebars, Jinja)

General-purpose template engines are powerful but solve a different problem.

|                               | Template engines              | mdt                                |
| ----------------------------- | ----------------------------- | ---------------------------------- |
| Target use case               | Generate files from templates | Sync content across existing files |
| Preserves surrounding content | No — replaces entire file     | Yes — only replaces tagged regions |
| Works in source comments      | Not designed for it           | Built-in                           |
| Learning curve                | Template language + config    | HTML comment tags                  |
| Invisible in rendered docs    | N/A — generates output        | Yes — tags are HTML comments       |

Template engines generate files. mdt synchronizes regions within files. If your docs are already written and you want to keep specific sections in sync, mdt fits without restructuring your project.

## When mdt is the right choice

mdt is a good fit when:

- The **same content** appears in **2+ places** (README, source docs, docs site)
- You want **CI to catch drift** before it reaches users
- You need **data interpolation** (versions, package names) across doc surfaces
- You want **editor support** for navigating and maintaining template relationships
- Your docs are already written and you want to **adopt incrementally**

## When something else might be better

- **Single-surface docs** — If all your docs live in one framework (e.g., only Docusaurus), framework-native includes may be simpler.
- **Full file generation** — If you're generating entire files from data, a template engine like Tera or Handlebars is more appropriate.
- **One-time migration** — If you just need to copy content once and it won't change, copy-paste is fine.

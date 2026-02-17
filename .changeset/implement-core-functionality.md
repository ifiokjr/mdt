---
mdt: minor
mdt_cli: minor
---

### mdt

Implement the core template management engine:

- **Parser**: Complete the `parse()` function that converts markdown content into structured `Block` types (provider and consumer) by wiring the lexer output through pattern matching into block construction. Extracts block names, types, and transformer/filter chains from token groups.
- **Project scanner** (`project` module): Walk a directory tree to discover `*.t.md` template definition files (providers) and other markdown files containing consumer blocks. Builds a map of provider name to content and collects all consumer entries with their file paths.
- **Content replacement engine** (`engine` module): Implement `check_project()` to verify all consumer blocks are up to date with their providers, and `compute_updates()` / `write_updates()` to replace stale consumer content. Supports all transformer types: `trim`, `trimStart`, `trimEnd`, `indent`, `prefix`, `wrap`, `codeBlock`, `code`, and `replace`.
- **New `Prefix` transformer type**: Added to support prefixing content with a string.
- **New error variants**: `MissingProvider` and `StaleConsumer` for better diagnostics.
- **Removed debug `println!`** from the lexer that was accidentally left in.

### mdt_cli

Implement all three CLI commands with real functionality:

- **`mdt init`**: Creates a sample `template.t.md` file with a provider block and prints getting-started instructions. Skips if the file already exists.
- **`mdt check`**: Scans the project for provider and consumer blocks, verifies all consumers are up to date. Exits with non-zero status and prints diagnostics if any blocks are stale.
- **`mdt update`**: Scans the project and replaces stale consumer content with the latest provider content, applying any configured transformers. Supports `--dry-run` to preview changes without writing files.
- **Global options**: `--path` to specify the project root, `--verbose` for detailed output.

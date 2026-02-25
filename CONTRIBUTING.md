# Contributing to mdt

Thank you for your interest in contributing to mdt. This guide covers everything you need to get started.

## Getting Started

This project uses [devenv](https://devenv.sh/) (Nix-based) for reproducible development environments. After cloning the repository:

```sh
# Enter the dev shell (automatic with direnv, or manually):
devenv shell

# Install all tooling (cargo binaries):
install:all
```

## Building and Testing

```sh
# Build all crates
cargo build --all-features

# Run tests with nextest (preferred)
cargo nextest run

# Run all tests including doc tests
cargo test

# Code coverage
cargo llvm-cov
```

## Formatting

Formatting is handled by **dprint**, not `cargo fmt`. dprint orchestrates `rustfmt` for Rust files, `nixfmt` for Nix files, and `shfmt` for shell scripts.

```sh
# Check formatting
dprint check

# Apply formatting
dprint fmt
```

## Linting

```sh
cargo clippy --all-features
```

## Code Style

- **Hard tabs** for indentation.
- **100 character** max line width.
- **One import per line** (`imports_granularity = "Item"`).
- Imports grouped by `StdExternalCrate`.

## Pull Request Workflow

Every change must be submitted via a pull request. Do not commit directly to `main`.

1. Create a feature branch from `main`.
2. Make your changes.
3. Ensure all checks pass locally (`cargo nextest run`, `dprint check`, `cargo clippy --all-features`).
4. Create a PR with a descriptive title and summary.
5. Wait for CI to pass, then request review.

## Changeset Requirement

**Every PR that modifies code in any crate must include at least one changeset file in `.changeset/`.** This ensures changes are tracked in changelogs and version bumps are applied correctly.

To create a changeset interactively:

```sh
knope document-change
```

Or create one manually by adding a markdown file in `.changeset/` with the following format:

```markdown
---
package_name: change_type
---

Detailed description of the change.
```

**Change types:** `major`, `minor`, `patch`, `docs`, `note`

**Package names:** `mdt_core`, `mdt_cli`, `mdt_lsp`, `mdt_mcp`

A single changeset file can reference multiple packages. After creating a changeset, format it:

```sh
dprint fmt .changeset/* --allow-no-files
```

## Security Constraints

The following constraints are enforced workspace-wide and in CI:

- `unsafe_code` is **denied**. Do not use `unsafe` blocks.
- `unstable_features` is **denied**.
- `clippy::correctness` is **denied** (not just warned).
- `clippy::wildcard_dependencies` is **denied**.
- `Result::expect` is a **disallowed method**. Use `unwrap_or_else` with an explicit panic message instead.

## Test Requirements

- Every test must have a clear purpose. No redundant or trivial tests.
- Tests should cover edge cases, error paths, and real-world usage patterns.

### Logic Bug Testing Protocol

When a logic bug is discovered, it **must** be reproduced with a failing test before fixing:

1. Write a test that reliably reproduces the bug.
2. Verify the test fails for the right reason.
3. Implement the fix.
4. Verify the test passes along with all other tests.

Never fix a bug without first having a failing test that proves the bug exists.

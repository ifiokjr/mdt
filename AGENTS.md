# AGENTS.md

mdt is a Rust workspace for defining markdown template blocks once and synchronizing them across markdown files and source-code comments.

## Essentials

- Package manager: Cargo.
- Enter the dev environment with `devenv shell` before running repo commands.
- Preferred repo commands:
  - `build:all`
  - `test:all`
  - `lint:all`
  - `fix:all`
- Use `fix:format` / `dprint`; do not run `rustfmt` directly.
- Use `lint:clippy` (or `cargo clippy --workspace --all-features --all-targets`) for clippy checks.
- All code changes go through a PR.
- Code changes in publishable crates require at least one `.changeset/*` entry.

## Read more only when needed

- [Commands](docs/agents/commands.md)
- [Architecture](docs/agents/architecture.md)
- [Template system](docs/agents/template-system.md)
- [Quality rules](docs/agents/quality.md)
- [Release process](docs/agents/release.md)
- [Git and PR workflow](docs/agents/workflow.md)

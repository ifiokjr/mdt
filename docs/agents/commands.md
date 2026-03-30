# Commands

## Environment setup

- Use `devenv shell` before running project commands.
- `install:all` installs cargo-managed helper binaries.

## Preferred repo commands

Use repo scripts first when they exist:

- `build:all` — build all crates with all features
- `build:book` — build the mdBook docs
- `test:all` — run cargo tests and doc tests
- `test:cargo` — run tests with `cargo nextest run`
- `test:docs` — run doc tests
- `lint:all` — run clippy, formatting, deny, and `mdt check`
- `lint:clippy` — run `cargo clippy --workspace --all-features --all-targets`
- `lint:format` — run `dprint check`
- `fix:all` — run clippy fixes, `mdt update`, and formatting
- `fix:clippy` — run clippy fixes for the workspace
- `fix:format` — format with dprint
- `coverage:all` — generate coverage with `cargo llvm-cov`
- `deny:check` — run `cargo deny check`
- `snapshot:review` — review insta snapshots
- `snapshot:update` — rerun tests and accept snapshots
- `update:deps` — run `cargo update` and `devenv update`

## Direct cargo commands

Use these when repo scripts are not enough:

- `cargo build --all-features`
- `cargo test`
- `cargo nextest run`
- `cargo llvm-cov`
- `cargo semver-checks`

## Toolchain guidance

- Treat the project as requiring stable Rust `>= 1.88`.
- MSRV is `1.86.0`.
- When in doubt, use the toolchain pinned by `rust-toolchain.toml`.

## Formatting

- Use `dprint` for formatting.
- Do not run `rustfmt` directly.
- dprint delegates to `rustfmt`, `nixfmt`, and `shfmt` as needed.

## Cargo aliases

Defined in `.cargo/config.toml`:

- `cargo deny`
- `cargo insta`
- `cargo llvm-cov`
- `cargo nextest`
- `cargo semver-checks`
- `cargo workspaces`

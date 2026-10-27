# mdt fuzz targets

[cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) targets for the parsers that run on third-party input: cloned repositories, pull requests, and user configuration.

- `parse_markdown` — markdown scanning: HTML-comment extraction, lexing, block parsing, and content extraction between tags (`.md` files).
- `parse_source` — source-comment scanning: comment-prefix stripping, fenced code-block filtering, and block parsing inside comments (`.rs`, `.ts`, `.py`, …).
- `parse_config` — `mdt.toml` deserialization via serde/TOML.

The `fuzz` crate is detached from the repository workspace, so workspace builds and the release pipeline never compile it. CI runs a short smoke session per target on every pull request (`.github/workflows/fuzz.yml`).

## Running locally

```sh
rustup toolchain install nightly
cargo install cargo-fuzz --locked --version 0.13.2

# From the fuzz directory:
cargo +nightly fuzz run parse_markdown -s none -- -max_total_time=60

# With sanitizers for deeper checking:
cargo +nightly fuzz run parse_markdown -- -max_total_time=60
```

A crashing input is written to `fuzz/artifacts/<target>/`; minimize and replay it with `cargo +nightly fuzz fmt` and `cargo +nightly fuzz run <target> <artifact>`.

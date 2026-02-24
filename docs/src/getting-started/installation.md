# Installation

## From crates.io

Install the CLI with cargo:

```sh
cargo install mdt_cli
```

This installs the `mdt` binary.

## From source

Clone the repository and build:

```sh
git clone https://github.com/ifiokjr/mdt.git
cd mdt
cargo install --path crates/mdt_cli
```

## As a library

To use the mdt core library in your own Rust project:

```toml
[dependencies]
mdt_core = "0.2.0"
```

## Verify installation

```sh
mdt --help
```

You should see the available commands: `init`, `check`, `update`, `list`, `lsp`, and `mcp`.

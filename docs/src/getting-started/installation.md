# Installation

## Recommended for Node.js users

Install the CLI from npm:

```sh
npm install -g @ifi/mdt
```

This installs the `mdt` command and pulls in the prebuilt binary package that matches your platform.

You can also run it without a global install:

```sh
npx @ifi/mdt --help
```

This path is ideal for JavaScript and TypeScript projects that already use npm and do not want to install the Rust toolchain.

## Recommended for most non-Rust users

Download the prebuilt binary for your platform from the [latest GitHub release](https://github.com/ifiokjr/mdt/releases/latest) and place the `mdt` binary somewhere on your `PATH`.

This is the simplest option if you want to use mdt in a Python, Go, or other non-Rust project without installing the Rust toolchain first.

## If you already use Cargo

Install the CLI from crates.io:

```sh
cargo install mdt_cli
```

This installs the `mdt` binary.

## From source

Clone the repository and build from the workspace:

```sh
git clone https://github.com/ifiokjr/mdt.git
cd mdt
cargo install --path mdt_cli
```

## As a library

To use the core engine in your own Rust project:

```toml
[dependencies]
mdt_core = "0.7.0"
```

## Verify installation

```sh
mdt --help
```

You should see the available commands: `init`, `check`, `update`, `list`, `info`, `doctor`, `assist`, `lsp`, and `mcp`.

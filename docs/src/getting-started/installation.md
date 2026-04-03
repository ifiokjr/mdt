# Installation

## Recommended for Node.js users

Install the CLI from npm:

```sh
npm install -g @m-d-t/cli
```

This installs the `mdt` command and pulls in the prebuilt binary package that matches your platform.

You can also run it without a global install:

```sh
npx @m-d-t/cli --help
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

## Agent skill package

If you use [Pi](https://github.com/badlogic/pi) or another agent harness that supports the [Agent Skills standard](https://agentskills.io), install the official mdt skill package:

```sh
pi install npm:@m-d-t/skills
```

This teaches your coding agent how to work with mdt template syntax, MCP tools, CLI commands, transformers, and configuration. See [Assistant Setup](./assistant-setup.md) for more details.

## Verify installation

```sh
mdt --help
```

You should see the available commands: `init`, `check`, `update`, `list`, `info`, `doctor`, `assist`, `lsp`, and `mcp`.

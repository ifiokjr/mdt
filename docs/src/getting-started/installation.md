# Installation

## npm (recommended)

Install the CLI globally:

```sh
npm install -g @m-d-t/cli
```

This installs the `mdt` command with a prebuilt binary for your platform (macOS, Linux, and Windows on x64 and arm64). It needs Node.js 18 or later, but no Rust toolchain.

Or run it without installing:

```sh
npx -y @m-d-t/cli --help
```

Pin the version in CI so every run uses the same binary:

```sh
npx -y @m-d-t/cli@<version> check
```

## Prebuilt binary

Download the archive for your platform from the [latest GitHub release](https://github.com/ifiokjr/mdt/releases/latest) and put the `mdt` binary on your `PATH`. This works in any project without Node.js or Rust.

## Cargo

If you already have a Rust toolchain:

```sh
cargo install mdt_cli
```

This compiles `mdt` from source, which is much slower than the prebuilt options. Prefer npm or a prebuilt binary when speed matters, for example in CI.

To build the latest unreleased code from the repository:

```sh
git clone https://github.com/ifiokjr/mdt.git
cd mdt
cargo install --path mdt_cli
```

## As a library

To use the core engine in your own Rust project:

<!-- {=mdtCoreInstall} -->

```toml
[dependencies]
mdt_core = "0.9.5"
```

<!-- {/mdtCoreInstall} -->

## For coding agents

The CLI ships its own agent skill, so any coding agent can learn mdt from the installed version:

```sh
mdt skill                          # print the skill
mdt skill --install .claude/skills # or write it to an agent skills directory
```

See [Assistant Setup](./assistant-setup.md) for per-agent instructions and MCP configuration.

## Verify installation

```sh
mdt --version
mdt --help
```

`mdt --help` lists the commands: `init`, `check`, `update`, `list`, `info`, `doctor`, `assist`, `skill`, `lsp`, and `mcp`.

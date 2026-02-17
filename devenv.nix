{
  pkgs,
  lib,
  config,
  ...
}:

{
  packages =
    with pkgs;
    [
      cargo-binstall
      cargo-run-bin
      deno
      dprint
      mdbook
      nixfmt-rfc-style
      rustup
      shfmt
    ]
    ++ lib.optionals stdenv.isDarwin [
      coreutils
    ];

  enterShell = ''
    set -e
    # Ensure the nightly toolchain is available for rustfmt (used by dprint)
    rustup toolchain install nightly --component rustfmt --no-self-update 2>/dev/null || true
  '';

  # disable dotenv since it breaks the variable interpolation supported by `direnv`
  dotenv.disableHint = true;

  scripts = {
    "knope" = {
      exec = ''
        set -e
        cargo bin knope $@
      '';
      description = "The `knope` executable";
      binary = "bash";
    };
    "install:all" = {
      exec = ''
        set -e
        install:cargo:bin
      '';
      description = "Install all packages.";
      binary = "bash";
    };
    "install:cargo:bin" = {
      exec = ''
        set -e
        cargo bin --install
      '';
      description = "Install cargo binaries locally.";
      binary = "bash";
    };
    "update:deps" = {
      exec = ''
        set -e
        cargo update
        devenv update
      '';
      description = "Update dependencies.";
      binary = "bash";
    };
    "build:all" = {
      exec = ''
        set -e
        if [ -z "$CI" ]; then
          echo "Building project locally"
          cargo build --all-features
        else
          echo "Building in CI"
          cargo build --all-features --locked
        fi
      '';
      description = "Build all crates with all features activated.";
      binary = "bash";
    };
    "build:book" = {
      exec = ''
        set -e
        mdbook build docs
      '';
      description = "Build the mdbook documentation.";
      binary = "bash";
    };
    "test:all" = {
      exec = ''
        set -e
        test:cargo
        test:docs
      '';
      description = "Run all tests across the crates.";
      binary = "bash";
    };
    "test:cargo" = {
      exec = ''
        set -e
        cargo nextest run
      '';
      description = "Run cargo tests with nextest.";
      binary = "bash";
    };
    "test:docs" = {
      exec = ''
        set -e
        cargo test --doc
      '';
      description = "Run documentation tests.";
      binary = "bash";
    };
    "coverage:all" = {
      exec = ''
        set -e
      '';
      description = "Run coverage across the crates.";
      binary = "bash";
    };
    "fix:all" = {
      exec = ''
        set -e
        fix:clippy
        fix:format
      '';
      description = "Fix all autofixable problems.";
      binary = "bash";
    };
    "fix:format" = {
      exec = ''
        set -e
        dprint fmt --config "$DEVENV_ROOT/dprint.json"
      '';
      description = "Format files with dprint.";
      binary = "bash";
    };
    "fix:clippy" = {
      exec = ''
        set -e
        cargo clippy --fix --allow-dirty --allow-staged --all-features
      '';
      description = "Fix clippy lints for rust.";
      binary = "bash";
    };
    "lint:all" = {
      exec = ''
        set -e
        lint:clippy
        lint:format
      '';
      description = "Run all checks.";
      binary = "bash";
    };
    "lint:format" = {
      exec = ''
        set -e
        dprint check
      '';
      description = "Check that all files are formatted.";
      binary = "bash";
    };
    "lint:clippy" = {
      exec = ''
        set -e
        cargo clippy --all-features
      '';
      description = "Check that all rust lints are passing.";
      binary = "bash";
    };
    "snapshot:review" = {
      exec = ''
        set -e
        cargo insta review
      '';
      description = "Review insta snapshots.";
      binary = "bash";
    };
    "snapshot:update" = {
      exec = ''
        set -e
        cargo nextest run
        cargo insta accept
      '';
      description = "Update insta snapshots.";
      binary = "bash";
    };
    "setup:vscode" = {
      exec = ''
        set -e
        rm -rf .vscode
        cp -r $DEVENV_ROOT/setup/editors/vscode .vscode
      '';
      description = "Setup the environment for vscode.";
      binary = "bash";
    };
    "setup:helix" = {
      exec = ''
        set -e
        rm -rf .helix
        cp -r $DEVENV_ROOT/setup/editors/helix .helix
      '';
      description = "Setup for the helix editor.";
      binary = "bash";
    };
  };
}

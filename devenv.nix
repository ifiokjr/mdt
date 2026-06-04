{
  pkgs,
  lib,
  inputs,
  config,
  ...
}:

let
  currentDir = builtins.dirOf __curPos.file;
  extra = inputs.ifiokjr-nixpkgs.packages.${pkgs.stdenv.system};
in

{
  packages =
    with pkgs;
    [
      actionlint
      cargo-binstall
      cargo-deny
      cargo-insta
      cargo-llvm-cov
      cargo-nextest
      cargo-run-bin
      deno
      dprint
      extra.monochange
      extra.pnpm
      mdbook
      nixfmt
      nodejs
      pnpm
      rustup
      shfmt
      taplo
      zizmor
    ]
    ++ lib.optionals stdenv.isDarwin [
      coreutils
    ];

  enterShell = ''
    set -e
    # Ensure the nightly toolchain is available and healthy for rustfmt (used by dprint).
    if ! rustup run nightly rustfmt --version >/dev/null 2>&1; then
      rustup toolchain install nightly --component rustfmt --no-self-update --force 2>/dev/null \
        || { rustup toolchain uninstall nightly >/dev/null 2>&1 && rustup toolchain install nightly --component rustfmt --no-self-update 2>/dev/null; } \
        || true
    fi
    # Ensure stable is at least 1.88 (required by rmcp/darling for mdt_mcp)
    rustup update stable --no-self-update 2>/dev/null || true
    eval "$(pnpm-activate-env)"
  '';

  # disable dotenv since it breaks the variable interpolation supported by `direnv`
  dotenv.disableHint = true;

  # Disable devenv's Cachix integration so unauthenticated FlakeHub cache lookups
  # do not warn during local validation commands.
  cachix.enable = false;

  git-hooks.hooks = {
    mdt-pre-commit = {
      enable = true;
      name = "mdt pre-commit autofix";
      description = "Apply autofixable formatting and lint updates to staged changes.";
      entry = "${config.env.DEVENV_PROFILE}/bin/lint:format";
      language = "system";
      pass_filenames = true;
      require_serial = true;
      always_run = true;
      stages = [ "pre-commit" ];
      extraPackages = with pkgs; [
        bash
        devenv
        git
      ];
    };

    mdt-pre-push = {
      enable = true;
      name = "mdt pre-push CI checks";
      description = "Run the local CI-equivalent checks before pushing.";
      entry = "${config.env.DEVENV_PROFILE}/bin/lint:push";
      language = "system";
      pass_filenames = false;
      require_serial = true;
      always_run = true;
      stages = [ "pre-push" ];
      extraPackages = with pkgs; [
        bash
        devenv
        git
      ];
    };
  };

  scripts = {
    "mdt" = {
      exec = ''
        set -e
        cargo run --quiet --bin mdt -- $@
      '';
      description = "The `mdt` executable";
      binary = "bash";
    };
    "lint:push" = {
      exec = ''
        set -euo pipefail

        ${currentDir}/.devenv/profile/bin/lint:clippy
        ${currentDir}/.devenv/profile/bin/lint:format
        ${currentDir}/.devenv/profile/bin/lint:actions
        ${currentDir}/.devenv/profile/bin/lint:npm
        pnpm node --import tsx --test scripts/npm/tests/*.test.ts
        ${currentDir}/.devenv/profile/bin/deny:check
      '';
      description = "Run pre-push CI-aligned checks.";
      binary = "bash";
    };
    "lint:npm" = {
      exec = ''
        set -euo pipefail

        pnpm check
        pnpm lint
      '';
      description = "Run npm package type and lint checks.";
      binary = "bash";
    };
    "install:all" = {
      exec = ''
        set -e
        install:cargo:bin

        if [ -z "$CI" ]; then
          pnpm install
        else
          pnpm install --frozen-lockfile
        fi
      '';
      description = "Install cargo binaries and pnpm modules.";
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
        cargo llvm-cov nextest --lcov --output-path lcov.info
      '';
      description = "Run coverage across the crates.";
      binary = "bash";
    };
    "fix:all" = {
      exec = ''
        set -e
        fix:clippy
        mdt update
        fix:format
        fix:actions
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
        cargo clippy --workspace --fix --allow-dirty --allow-staged --all-features --all-targets
      '';
      description = "Fix clippy lints for rust.";
      binary = "bash";
    };
    "deny:check" = {
      exec = ''
        set -e
        cargo deny check
      '';
      description = "Run cargo-deny checks for security advisories and license compliance.";
      binary = "bash";
    };
    "lint:all" = {
      exec = ''
        set -e
        lint:clippy
        lint:format
        lint:actions
        deny:check
        mdt check
      '';
      description = "Run all checks.";
      binary = "bash";
    };
    "lint:workflows" = {
      exec = ''
        set -euo pipefail
        zizmor .github/workflows/ .github/actions/
      '';
      description = "Scan GitHub Actions workflows and actions for security vulnerabilities with zizmor.";
      binary = "bash";
    };
    "fix:workflows" = {
      exec = ''
        set -euo pipefail
        zizmor --fix .github/workflows/ .github/actions/
      '';
      description = "Auto-fix zizmor findings in GitHub Actions workflows where possible.";
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
    "lint:actions" = {
      exec = ''
        set -e
        actionlint
      '';
      description = "Lint GitHub Actions workflows.";
      binary = "bash";
    };
    "fix:actions" = {
      exec = ''
        set -e
        lint:actions
      '';
      description = "Check GitHub Actions workflows (actionlint has no autofix mode).";
      binary = "bash";
    };
    "lint:clippy" = {
      exec = ''
        set -e
        cargo clippy --workspace --all-features --all-targets
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
  };
}

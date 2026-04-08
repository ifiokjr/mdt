#!/usr/bin/env bash
set -euo pipefail

if [[ "${MDT_GIT_HOOK_IN_DEVENV:-0}" != "1" ]]; then
	exec devenv shell -- env MDT_GIT_HOOK_IN_DEVENV=1 bash "$0" "$@"
fi

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

echo "pre-push: running CI-aligned checks"
lint:all
test:all
node --test npm/tests/*.test.mjs scripts/npm/tests/*.test.mjs
cargo build --locked
cargo build --all-features --locked

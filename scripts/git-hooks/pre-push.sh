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
pnpm build
node --import tsx --test scripts/npm/tests/*.test.ts
cargo build --locked
cargo build --all-features --locked

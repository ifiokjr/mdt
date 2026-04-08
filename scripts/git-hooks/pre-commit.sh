#!/usr/bin/env bash
set -euo pipefail

if [[ "${MDT_GIT_HOOK_IN_DEVENV:-0}" != "1" ]]; then
	exec devenv shell -- env MDT_GIT_HOOK_IN_DEVENV=1 bash "$0" "$@"
fi

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

staged_files=("$@")
existing_staged_files=()
staged_rust_files=()

for file in "${staged_files[@]}"; do
	if [[ -e "$file" ]]; then
		existing_staged_files+=("$file")
	fi

	if [[ "$file" == *.rs ]]; then
		staged_rust_files+=("$file")
	fi
done

echo "pre-commit: updating generated markdown targets"
mdt update

if ((${#staged_rust_files[@]} > 0)); then
	echo "pre-commit: applying clippy fixes"
	cargo clippy --workspace --fix --allow-dirty --allow-staged --all-features --all-targets
else
	echo "pre-commit: no staged Rust files; skipping clippy fixes"
fi

format_targets=()
if ((${#existing_staged_files[@]} > 0)); then
	format_targets+=("${existing_staged_files[@]}")
fi

while IFS= read -r file; do
	[[ -n "$file" && -e "$file" ]] || continue
	format_targets+=("$file")
done < <(git diff --name-only --diff-filter=ACMR)

if ((${#format_targets[@]} > 0)); then
	echo "pre-commit: formatting changed files"
	if ! dprint fmt --config "$DEVENV_ROOT/dprint.json" --allow-no-files "${format_targets[@]}"; then
		echo "pre-commit: staged-file formatting failed; retrying with full repository formatting"
		fix:format
	fi
else
	echo "pre-commit: no changed files to format"
fi

echo "pre-commit: staging autofixed changes"
git add -u -- .
if ((${#existing_staged_files[@]} > 0)); then
	git add -- "${existing_staged_files[@]}"
fi

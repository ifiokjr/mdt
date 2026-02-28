#!/usr/bin/env bash
set -euo pipefail

usage() {
	cat <<'USAGE'
Generate a deterministic benchmark workload for mdt.

Usage:
  scripts/benchmark/generate_workload.sh --output <dir> [--files <count>]

Options:
  --output <dir>   Target directory for generated workload (required).
  --files <count>  Number of markdown files to generate (default: 180).
USAGE
}

output_dir=""
file_count=180

while [[ $# -gt 0 ]]; do
	case "$1" in
		--output)
			output_dir="${2:-}"
			shift 2
			;;
		--files)
			file_count="${2:-}"
			shift 2
			;;
		-h|--help)
			usage
			exit 0
			;;
		*)
			echo "unknown option: $1" >&2
			usage >&2
			exit 1
			;;
	esac
done

if [[ -z "$output_dir" ]]; then
	echo "--output is required" >&2
	usage >&2
	exit 1
fi

if ! [[ "$file_count" =~ ^[0-9]+$ ]]; then
	echo "--files must be a positive integer" >&2
	exit 1
fi

if (( file_count < 1 )); then
	echo "--files must be >= 1" >&2
	exit 1
fi

rm -rf "$output_dir"
mkdir -p "$output_dir/.templates" "$output_dir/modules"

cat > "$output_dir/package.json" <<'JSON'
{
	"name": "mdt-benchmark-fixture",
	"version": "1.0.0",
	"description": "Deterministic fixture used for CI benchmark comparisons"
}
JSON

cat > "$output_dir/mdt.toml" <<'TOML'
[templates]
paths = [".templates"]

[data]
package = "package.json"

[padding]
before = 0
after = 0
TOML

cat > "$output_dir/.templates/template.t.md" <<'TEMPLATE'
<!-- {@overview} -->

{{ package.name }} v{{ package.version }}

{{ package.description }}

<!-- {/overview} -->

<!-- {@usage} -->

Run `mdt check` to verify all consumer blocks are synchronized.
Run `mdt update` to rewrite stale content.

<!-- {/usage} -->

<!-- {@api} -->

pub fn benchmark_fixture_example() -> &'static str {
    "ok"
}

<!-- {/api} -->

<!-- {@notes} -->

This fixture is intentionally deterministic.
Do not manually edit generated files in CI.

<!-- {/notes} -->
TEMPLATE

for ((i = 1; i <= file_count; i++)); do
	module_id=$(printf "%04d" "$i")
	group_id=$(printf "%02d" "$(((i - 1) / 20 + 1))")
	group_dir="$output_dir/modules/group-$group_id"
	mkdir -p "$group_dir"

	cat > "$group_dir/module-$module_id.md" <<EOF_MODULE
# Module $module_id

<!-- {=overview|trim} -->
stale overview $module_id
<!-- {/overview} -->

<!-- {=usage|trim} -->
stale usage $module_id
<!-- {/usage} -->

<!-- {=api|trim} -->
stale api $module_id
<!-- {/api} -->

<!-- {=notes|trim} -->
stale notes $module_id
<!-- {/notes} -->
EOF_MODULE
done

echo "generated workload at $output_dir with $file_count markdown files"

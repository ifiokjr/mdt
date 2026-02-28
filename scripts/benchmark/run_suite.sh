#!/usr/bin/env bash
set -euo pipefail

usage() {
	cat <<'USAGE'
Run the mdt benchmark suite and export JSON results.

Usage:
  scripts/benchmark/run_suite.sh \
    --binary <path> \
    --output <path> \
    [--label <name>] \
    [--iterations <count>] \
    [--warmup <count>] \
    [--files <count>] \
    [--workdir <path>]

Options:
  --binary <path>      Path to the mdt binary to benchmark (required).
  --output <path>      JSON output path (required).
  --label <name>       Logical label for this result set (default: unnamed).
  --iterations <count> Timed iterations per scenario (default: 9).
  --warmup <count>     Warmup iterations per scenario (default: 2).
  --files <count>      Number of generated fixture files (default: 180).
  --workdir <path>     Working directory for temporary benchmark fixtures.
USAGE
}

json_escape() {
	local value="$1"
	value=${value//\\/\\\\}
	value=${value//\"/\\\"}
	value=${value//$'\n'/\\n}
	value=${value//$'\r'/\\r}
	value=${value//$'\t'/\\t}
	printf '%s' "$value"
}

now_ns() {
	perl -MTime::HiRes=time -e 'printf("%.0f\n", time() * 1000000000)'
}

join_by() {
	local delimiter="$1"
	shift
	local first=true
	for token in "$@"; do
		if [[ "$first" == true ]]; then
			printf '%s' "$token"
			first=false
		else
			printf '%s%s' "$delimiter" "$token"
		fi
	done
}

calc_stats_json() {
	local values=("$@")
	if (( ${#values[@]} == 0 )); then
		echo '{"min_ms":0,"max_ms":0,"mean_ms":0,"median_ms":0,"p95_ms":0,"samples_ms":[]}'
		return
	fi

	local sorted
	sorted=$(printf '%s\n' "${values[@]}" | sort -n)
	local count
	count=$(printf '%s\n' "$sorted" | awk 'NF {count += 1} END {print count}')

	local min_ms
	min_ms=$(printf '%s\n' "$sorted" | awk 'NF {print; exit}')
	local max_ms
	max_ms=$(printf '%s\n' "$sorted" | awk 'NF {value=$1} END {print value}')
	local mean_ms
	mean_ms=$(printf '%s\n' "$sorted" | awk 'NF {sum += $1; count += 1} END {if (count == 0) {print 0} else {printf "%.3f", sum / count}}')

	local median_ms
	median_ms=$(printf '%s\n' "$sorted" | awk '
		NF { values[count] = $1; count += 1 }
		END {
			if (count == 0) {
				print 0;
				exit;
			}
			mid = int(count / 2);
			if (count % 2 == 1) {
				printf "%.3f", values[mid];
			} else {
				printf "%.3f", (values[mid - 1] + values[mid]) / 2;
			}
		}
	')

	local p95_ms
	p95_ms=$(printf '%s\n' "$sorted" | awk '
		NF { values[count] = $1; count += 1 }
		END {
			if (count == 0) {
				print 0;
				exit;
			}
			p95_index = int((count * 95 + 99) / 100) - 1;
			if (p95_index < 0) {
				p95_index = 0;
			}
			if (p95_index >= count) {
				p95_index = count - 1;
			}
			printf "%.3f", values[p95_index];
		}
	')

	local sample_json
	sample_json="$(printf '%s\n' "$sorted" | awk 'NF {if (count > 0) {printf ","}; printf "%.3f", $1; count += 1} END {if (count == 0) {printf ""}}')"
	if [[ -z "$sample_json" ]]; then
		sample_json='[]'
	else
		sample_json="[$sample_json]"
	fi

	cat <<EOF_STATS
{"min_ms":$min_ms,"max_ms":$max_ms,"mean_ms":$mean_ms,"median_ms":$median_ms,"p95_ms":$p95_ms,"samples_ms":$sample_json}
EOF_STATS
}

run_command_timed() {
	local expected_exit="$1"
	local project_dir="$2"
	shift 2
	local start_ns
	start_ns=$(now_ns)
	set +e
	"$binary" --no-color --path "$project_dir" "$@" >/dev/null 2>&1
	local exit_code=$?
	set -e
	local end_ns
	end_ns=$(now_ns)

	if [[ "$exit_code" -ne "$expected_exit" ]]; then
		echo "command failed with unexpected exit code: expected=$expected_exit actual=$exit_code command=$*" >&2
		exit 1
	fi

	awk -v start="$start_ns" -v end="$end_ns" 'BEGIN { printf "%.3f", (end - start) / 1000000 }'
}

scenario_json_entries=()

record_scenario() {
	local name="$1"
	local command_text="$2"
	local expected_exit="$3"
	shift 3
	local samples=("$@")

	local stats_json
	stats_json=$(calc_stats_json "${samples[@]}")
	local escaped_name
	escaped_name=$(json_escape "$name")
	local escaped_command
	escaped_command=$(json_escape "$command_text")

	scenario_json_entries+=("{\"name\":\"$escaped_name\",\"command\":\"$escaped_command\",\"expected_exit\":$expected_exit,\"iterations\":${#samples[@]},\"stats\":$stats_json}")
}

run_non_mutating_scenario() {
	local name="$1"
	local source_fixture="$2"
	local cache_mode="$3"
	local expected_exit="$4"
	shift 4
	local command=("$@")
	local case_dir="$suite_dir/case-$name"
	local samples=()

	rm -rf "$case_dir"
	cp -R "$source_fixture" "$case_dir"

	for ((i = 1; i <= warmup; i++)); do
		if [[ "$cache_mode" == "cold" ]]; then
			rm -rf "$case_dir/.mdt"
		fi
		run_command_timed "$expected_exit" "$case_dir" "${command[@]}" >/dev/null
	done

	for ((i = 1; i <= iterations; i++)); do
		if [[ "$cache_mode" == "cold" ]]; then
			rm -rf "$case_dir/.mdt"
		fi
		samples+=("$(run_command_timed "$expected_exit" "$case_dir" "${command[@]}")")
	done

	record_scenario "$name" "${command[*]}" "$expected_exit" "${samples[@]}"
}

run_mutating_scenario() {
	local name="$1"
	local source_fixture="$2"
	local expected_exit="$3"
	shift 3
	local command=("$@")
	local case_dir="$suite_dir/case-$name"
	local samples=()

	for ((i = 1; i <= warmup; i++)); do
		rm -rf "$case_dir"
		cp -R "$source_fixture" "$case_dir"
		run_command_timed "$expected_exit" "$case_dir" "${command[@]}" >/dev/null
	done

	for ((i = 1; i <= iterations; i++)); do
		rm -rf "$case_dir"
		cp -R "$source_fixture" "$case_dir"
		samples+=("$(run_command_timed "$expected_exit" "$case_dir" "${command[@]}")")
	done

	record_scenario "$name" "${command[*]}" "$expected_exit" "${samples[@]}"
}

binary=""
output=""
label="unnamed"
iterations=9
warmup=2
file_count=180
workdir=""

while [[ $# -gt 0 ]]; do
	case "$1" in
		--binary)
			binary="${2:-}"
			shift 2
			;;
		--output)
			output="${2:-}"
			shift 2
			;;
		--label)
			label="${2:-}"
			shift 2
			;;
		--iterations)
			iterations="${2:-}"
			shift 2
			;;
		--warmup)
			warmup="${2:-}"
			shift 2
			;;
		--files)
			file_count="${2:-}"
			shift 2
			;;
		--workdir)
			workdir="${2:-}"
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

if [[ -z "$binary" ]]; then
	echo "--binary is required" >&2
	exit 1
fi

if [[ -z "$output" ]]; then
	echo "--output is required" >&2
	exit 1
fi

if [[ ! -x "$binary" ]]; then
	echo "binary is not executable: $binary" >&2
	exit 1
fi

for numeric_arg in iterations warmup file_count; do
	value="${!numeric_arg}"
	if ! [[ "$value" =~ ^[0-9]+$ ]]; then
		case "$numeric_arg" in
			file_count)
				echo "--files must be a non-negative integer" >&2
				;;
			*)
				echo "--$numeric_arg must be a non-negative integer" >&2
				;;
		esac
		exit 1
	fi
done

if (( iterations < 1 )); then
	echo "--iterations must be >= 1" >&2
	exit 1
fi

if (( file_count < 1 )); then
	echo "--files must be >= 1" >&2
	exit 1
fi

if [[ -z "$workdir" ]]; then
	workdir=$(mktemp -d -t mdt-benchmark-suite-XXXXXX)
	cleanup_workdir=true
else
	mkdir -p "$workdir"
	cleanup_workdir=false
fi

suite_dir="$workdir/suite"
rm -rf "$suite_dir"
mkdir -p "$suite_dir"

stale_fixture="$suite_dir/workload-stale"
clean_fixture="$suite_dir/workload-clean"

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
"$script_dir/generate_workload.sh" --output "$stale_fixture" --files "$file_count" >/dev/null

cp -R "$stale_fixture" "$clean_fixture"
"$binary" --no-color --path "$clean_fixture" update >/dev/null

run_non_mutating_scenario "check_cold_clean" "$clean_fixture" "cold" 0 check
run_non_mutating_scenario "check_warm_clean" "$clean_fixture" "warm" 0 check
run_non_mutating_scenario "check_cold_stale" "$stale_fixture" "cold" 1 check
run_non_mutating_scenario "check_diff_stale" "$stale_fixture" "cold" 1 check --diff
run_mutating_scenario "update_stale" "$stale_fixture" 0 update
run_mutating_scenario "update_noop_clean" "$clean_fixture" 0 update
run_non_mutating_scenario "list_clean" "$clean_fixture" "warm" 0 list
run_non_mutating_scenario "info_clean" "$clean_fixture" "warm" 0 info

mkdir -p "$(dirname "$output")"

escaped_label=$(json_escape "$label")
escaped_binary=$(json_escape "$binary")
generated_at_utc=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
scenario_joined=$(join_by , "${scenario_json_entries[@]}")

cat > "$output" <<EOF_JSON
{
	"schema_version": 1,
	"generated_at_utc": "${generated_at_utc}",
	"label": "${escaped_label}",
	"binary": "${escaped_binary}",
	"iterations": ${iterations},
	"warmup": ${warmup},
	"file_count": ${file_count},
	"scenarios": [${scenario_joined}]
}
EOF_JSON

echo "benchmark suite complete: $output"

if [[ "$cleanup_workdir" == true ]]; then
	rm -rf "$workdir"
fi

#!/usr/bin/env bash
set -euo pipefail

output=""
inputs=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --output)
      output="${2:?--output requires a path}"
      shift 2
      ;;
    --input)
      inputs+=("${2:?--input requires a path}")
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

if [[ -z "$output" ]]; then
  echo "--output is required" >&2
  exit 1
fi

if [[ ${#inputs[@]} -eq 0 ]]; then
  echo "at least one --input is required" >&2
  exit 1
fi

rm -rf "$output"
mkdir -p "$output"

for input in "${inputs[@]}"; do
  runtimes="$input/runtimes"
  if [[ ! -d "$runtimes" ]]; then
    echo "native stage input '$input' is missing a runtimes directory" >&2
    exit 1
  fi
  shopt -s nullglob dotglob
  for rid_dir in "$runtimes"/*; do
    if [[ -d "$rid_dir" ]]; then
      name="$(basename "$rid_dir")"
      mkdir -p "$output/runtimes"
      cp -R "$rid_dir" "$output/runtimes/$name"
    fi
  done
done

echo "Merged native stages into $output"

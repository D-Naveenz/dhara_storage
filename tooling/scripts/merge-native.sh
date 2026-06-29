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

copy_tree() {
  local source="$1"
  local destination="$2"
  if [[ ! -d "$source" ]]; then
    return 0
  fi
  mkdir -p "$destination"
  shopt -s nullglob dotglob
  for entry in "$source"/*; do
    local name
    name="$(basename "$entry")"
    if [[ -d "$entry" ]]; then
      copy_tree "$entry" "$destination/$name"
    elif [[ -f "$entry" ]]; then
      cp "$entry" "$destination/$name"
    fi
  done
}

rm -rf "$output"
mkdir -p "$output"

for input in "${inputs[@]}"; do
  runtimes="$input/runtimes"
  if [[ ! -d "$runtimes" ]]; then
    echo "native stage input '$input' is missing a runtimes directory" >&2
    exit 1
  fi
  copy_tree "$runtimes" "$output/runtimes"
done

echo "Merged native stages into $output"

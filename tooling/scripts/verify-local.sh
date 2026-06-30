#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo_root"

args=(quality run)
for arg in "$@"; do
  case "$arg" in
    --skip-docs) args+=(--skip-docs) ;;
    --skip-dotnet) args+=(--skip-dotnet) ;;
    *) echo "unknown argument: $arg" >&2; exit 2 ;;
  esac
done

exec cargo run -p dhara_tool -- "${args[@]}"

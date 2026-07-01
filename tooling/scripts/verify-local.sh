#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo_root"

"$(dirname "$0")/ensure-dhara-tool-dist.sh"

bin="$repo_root/target/dist/dhara_tool"
args=(-r "$repo_root" --yes quality run)
for arg in "$@"; do
  case "$arg" in
    --skip-docs) args+=(--skip-docs) ;;
    --skip-dotnet) args+=(--skip-dotnet) ;;
    *) echo "unknown argument: $arg" >&2; exit 2 ;;
  esac
done

exec "$bin" "${args[@]}"

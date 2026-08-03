#!/usr/bin/env bash
# Ensures target/dist/drot (+ drot_tui) match tooling/drot, then launches the tool.
#
# Default: interactive TUI (drot_tui) — for developers.
# Agents / scripts: pass --cli to run the direct CLI (drot).
#
# Examples:
#   ./tooling/scripts/run-drot.sh
#   ./tooling/scripts/run-drot.sh --cli --yes quality run
#   ./tooling/scripts/run-drot.sh --cli --yes build run --skip-verify
set -euo pipefail

force_build=false
use_cli=false
repository=""
drot_args=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --force-build|-f)
      force_build=true
      shift
      ;;
    --cli)
      use_cli=true
      shift
      ;;
    -r|--repository)
      repository="$2"
      shift 2
      ;;
    --)
      shift
      drot_args=("$@")
      break
      ;;
    *)
      drot_args+=("$1")
      shift
      ;;
  esac
done

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo_root"

ensure_args=()
if [[ "$force_build" == true ]]; then
  ensure_args+=(--force)
fi
"$(dirname "$0")/ensure-drot-dist.sh" "${ensure_args[@]}"

# Remaining command tokens imply CLI even without --cli (TUI does not take subcommands).
if [[ "$use_cli" != true && ${#drot_args[@]} -gt 0 ]]; then
  use_cli=true
fi

if [[ "$use_cli" == true ]]; then
  bin="$repo_root/target/dist/drot"
else
  bin="$repo_root/target/dist/drot_tui"
fi

if [[ ! -f "$bin" ]]; then
  echo "drot binary missing at $bin after ensure-drot-dist" >&2
  exit 1
fi

args=()
if [[ -n "$repository" ]]; then
  args+=(-r "$repository")
elif [[ " ${drot_args[*]:-} " != *" -r "* && " ${drot_args[*]:-} " != *" --repository "* ]]; then
  args+=(-r "$repo_root")
fi
args+=("${drot_args[@]:-}")

exec "$bin" "${args[@]}"

#!/usr/bin/env bash
# Ensures target/dist/drot matches tooling/drot/Cargo.toml, then runs drot with -r <repo>.
set -euo pipefail

force_build=false
repository=""
drot_args=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --force-build|-f)
      force_build=true
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

bin="$repo_root/target/dist/drot"
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

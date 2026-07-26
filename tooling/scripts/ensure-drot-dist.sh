#!/usr/bin/env bash
# Ensures target/dist/drot matches tooling/drot/Cargo.toml workspace.package.version.
# Tool version lives only in the DROT submodule (https://github.com/D-Naveenz/dhara_repo_orchestration).
set -euo pipefail

force=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --force|-f) force=true; shift ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo_root"

manifest="$repo_root/tooling/drot/Cargo.toml"
if [[ ! -f "$manifest" ]]; then
  echo "DROT submodule missing at tooling/drot — run: git submodule update --init --recursive" >&2
  exit 1
fi
expected_version="$(grep -A1 '^\[workspace\.package\]' "$manifest" | grep -E '^\s*version\s*=' | head -1 | sed -E 's/.*"([^"]+)".*/\1/')"
if [[ -z "$expected_version" ]]; then
  echo "missing workspace.package.version in $manifest" >&2
  exit 1
fi

bin="$repo_root/target/dist/drot"
need_build=false

if [[ "$force" == true ]]; then
  echo "build: --force requested"
  need_build=true
elif [[ ! -f "$bin" ]]; then
  echo "build: dist missing (manifest v$expected_version)"
  need_build=true
else
  built_version="$("$bin" --version | tr -d '\r\n')"
  if [[ "$built_version" != "$expected_version" ]]; then
    echo "build: dist v$built_version != manifest v$expected_version"
    need_build=true
  else
    echo "skip: dist v$expected_version current"
  fi
fi

if [[ "$need_build" == true ]]; then
  export CARGO_TARGET_DIR="$repo_root/target"
  cargo build --manifest-path tooling/drot/Cargo.toml -p drot --profile dist
  built_version="$("$bin" --version | tr -d '\r\n')"
  if [[ "$built_version" != "$expected_version" ]]; then
    echo "smoke failed: dist reports v$built_version, expected v$expected_version" >&2
    exit 1
  fi
  echo "built: dist v$expected_version"
fi

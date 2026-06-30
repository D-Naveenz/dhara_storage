#!/usr/bin/env bash
# Ensures target/dist/dhara_tool matches tooling/dhara_tool/Cargo.toml package.version.
# Bump tool version (and config sync) when shipping tool changes — same policy as CI cache.
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

manifest="$repo_root/tooling/dhara_tool/Cargo.toml"
expected_version="$(grep -E '^version\s*=' "$manifest" | head -1 | sed -E 's/.*"([^"]+)".*/\1/')"
if [[ -z "$expected_version" ]]; then
  echo "missing package.version in $manifest" >&2
  exit 1
fi

bin="$repo_root/target/dist/dhara_tool"
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
  cargo build -p dhara_tool --profile dist
  built_version="$("$bin" --version | tr -d '\r\n')"
  if [[ "$built_version" != "$expected_version" ]]; then
    echo "smoke failed: dist reports v$built_version, expected v$expected_version" >&2
    exit 1
  fi
  echo "built: dist v$expected_version"
fi

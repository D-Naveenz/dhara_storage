#!/usr/bin/env bash
set -euo pipefail

skip_docs=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-docs)
      skip_docs=true
      shift
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo_root"

run() {
  echo "==> $*"
  "$@"
}

run cargo fmt -p dhara_storage_dal -p dhara_storage -p dharastorage -p dhara_tool --check
run cargo clippy -p dhara_storage --all-targets --all-features -- -D warnings
run cargo clippy -p dhara_storage_dal -p dharastorage -p dhara_tool --all-targets -- -D warnings

if [[ "$skip_docs" != true ]]; then
  run cargo doc -p dhara_storage --no-deps --all-features
  run cargo doc -p dhara_storage_dal -p dharastorage -p dhara_tool --no-deps
fi

run cargo test -p dhara_storage --all-features
run cargo test -p dhara_storage_dal
run cargo test -p dharastorage

tests_project="src/bindings/Dhara.Storage.Tests/Dhara.Storage.Tests.csproj"
if command -v dotnet >/dev/null 2>&1; then
  run dotnet test "$tests_project"
else
  echo "warning: dotnet not found; skipping .NET tests" >&2
fi

echo "Local CI checks passed."

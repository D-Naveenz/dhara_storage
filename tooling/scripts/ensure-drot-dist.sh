#!/usr/bin/env bash
# Ensures target/dist/drot (+ drot_tui) match the tooling/drot git checkout.
# Stamp: target/dist/.drot-git-rev (full HEAD SHA when the tree was clean at build time).
# Tool version still lives in tooling/drot/Cargo.toml; rebuild gating is by git, not semver.
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

drot_root="$repo_root/tooling/drot"
manifest="$drot_root/Cargo.toml"
if [[ ! -f "$manifest" ]]; then
  echo "DROT submodule missing at tooling/drot — run: git submodule update --init --recursive" >&2
  exit 1
fi

cli_bin="$repo_root/target/dist/drot"
tui_bin="$repo_root/target/dist/drot_tui"
stamp_path="$repo_root/target/dist/.drot-git-rev"
need_build=false

drot_git_head() {
  git -C "$drot_root" rev-parse HEAD
}

drot_git_dirty() {
  [[ -n "$(git -C "$drot_root" status --porcelain)" ]]
}

if [[ "$force" == true ]]; then
  echo "build: --force requested"
  need_build=true
fi

head="$(drot_git_head)"

if [[ "$need_build" != true ]]; then
  if [[ ! -f "$cli_bin" || ! -f "$tui_bin" ]]; then
    echo "build: dist missing CLI and/or TUI"
    need_build=true
  elif drot_git_dirty; then
    # Stamp is only trustworthy on a clean tree; local edits must rebuild.
    echo "build: tooling/drot has uncommitted changes"
    need_build=true
  elif [[ ! -f "$stamp_path" ]]; then
    echo "build: missing dist git stamp (.drot-git-rev)"
    need_build=true
  else
    stamped="$(tr -d '\r\n' <"$stamp_path")"
    if [[ "$stamped" != "$head" ]]; then
      echo "build: dist stamp ${stamped:0:12} != HEAD ${head:0:12}"
      need_build=true
    else
      echo "skip: dist matches tooling/drot @${head:0:12}"
    fi
  fi
fi

if [[ "$need_build" == true ]]; then
  export CARGO_TARGET_DIR="$repo_root/target"
  cargo build --manifest-path tooling/drot/Cargo.toml -p drot -p drot_tui --profile dist
  if [[ ! -f "$cli_bin" || ! -f "$tui_bin" ]]; then
    echo "smoke failed: drot and/or drot_tui missing under target/dist after dist build" >&2
    exit 1
  fi

  mkdir -p "$(dirname "$stamp_path")"
  head_after="$(drot_git_head)"
  if drot_git_dirty; then
    printf 'dirty %s' "$head_after" >"$stamp_path"
    echo "built: dist from dirty tooling/drot @${head_after:0:12} (will rebuild until clean)"
  else
    printf '%s' "$head_after" >"$stamp_path"
    echo "built: dist from tooling/drot @${head_after:0:12}"
  fi
fi

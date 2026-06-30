#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo_root"

tool_path="target/ci/dhara_tool"
if [[ ! -x "$tool_path" ]]; then
  cargo build -p dhara_tool --profile ci
fi

exec "$tool_path" package stage-native

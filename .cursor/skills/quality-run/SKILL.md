---
name: quality-run
description: >-
  Runs DROT local quality checks (`quality run` / verify-local), fixes failures,
  and commits the result so PRs are green before push. Use when finishing feature
  work, before creating a pull request, when the user asks to verify / quality /
  CI parity, or to avoid first-push quality.yml failures.
---

# Quality run (pre-PR gate)

Goal: leave the branch with **passing local `quality run`** and those fixes **committed**, so GitHub `quality.yml` does not fail on first push.

This is the **pre-push** counterpart to **autopilot** (post-PR CI / review triage). Prefer this skill so autopilot is not required just for fmt/clippy/doc.

## When to run

Apply this skill (do not wait to be asked again) when:

- Creating or updating a PR for this repo
- User asks to verify, run quality, CI parity, or “make sure CI passes”
- Feature/fix work is otherwise done and about to be pushed

Skip only when the user explicitly says to skip quality, or the task is docs-only with no Rust/C# compile impact **and** they did not ask for verify.

## Command

From the storage repo root (Windows workstation):

```powershell
./tooling/scripts/verify-local.ps1
```

Equivalent: `./tooling/scripts/run-drot.ps1 --yes quality run`

Optional skips (only if the user asks): `-SkipDocs`, `-SkipDotnet`.

Do **not** use `build run` unless the user asked for a full package build. Do **not** weaken `.github/workflows` or clippy flags to make checks pass.

## Fix loop

1. Run `verify-local.ps1` (long-running; background + await).
2. On failure, read the DROT / cargo / dotnet log: fix the **reported** failure first (fmt → clippy → doc → tests → dotnet, in the order the tool stops).
3. Typical fixes:
   - **fmt**: `cargo fmt -p dhara_storage_core -p dhara_storage -p dharastorage-ffi -p dhara-sd` (same package set as CI)
   - **clippy / compile**: smallest correct code change; prefer grouping args or removing dead params over `#[allow]` unless allow is already the local pattern
   - **doc / tests / .NET**: fix product code or tests; do not delete coverage to silence failures
4. Re-run `verify-local.ps1` until exit code **0** and the log shows local CI checks passed.
5. If blocked (need product decision, secrets, or unrelated infra), stop and report — do not invent workarounds.

Cap: after **three** full quality attempts without progress, stop and summarize remaining failures.

## Commit (authorized by this skill)

When this skill is active, **commit** after quality is green. Do not wait for a separate “please commit.”

Follow the user’s git commit protocol (status / diff / log → stage → commit → status). Message focuses on **why** (e.g. satisfy quality gate / rustfmt / clippy).

- Include the quality fixes and any uncommitted work that belongs to the current task.
- Do not commit secrets (`.env`, credentials).
- Do **not** push unless the user asked to push or create/update a PR that requires push.
- Do not amend unless the user’s amend rules are fully met.

If there is nothing to commit (already clean and green), say so — do not create an empty commit.

## Report

End with: quality outcome (pass/fail), what was fixed, commit hash or “no commit needed,” and whether push/PR is still outstanding.

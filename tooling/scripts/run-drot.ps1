# Ensures target/dist/drot matches tooling/drot, then launches the tool.
#
# Default (no subcommand): interactive TUI.
# Agents / scripts: pass a subcommand or --help for the Direct CLI.
#
# Examples:
#   ./tooling/scripts/run-drot.ps1
#   ./tooling/scripts/run-drot.ps1 --yes quality run
#   ./tooling/scripts/run-drot.ps1 --help
#   ./tooling/scripts/run-drot.ps1 --yes build run --skip-verify
[CmdletBinding(PositionalBinding = $false)]
param(
    # Named switch (not ForceBuild+Alias): PositionalBinding=$false + Alias("Force") was unreliable.
    [switch] $Force,
    # Named-only: otherwise `--yes` / subcommands bind positionally to Repository.
    [string] $Repository,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $DrotArgs
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $repoRoot

$ensureArgs = @()
if ($Force) {
    $ensureArgs += "-Force"
}
& (Join-Path $PSScriptRoot "ensure-drot-dist.ps1") @ensureArgs
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$bin = Join-Path $repoRoot "target\dist\drot.exe"

if (-not (Test-Path $bin)) {
    throw "drot binary missing at $bin after ensure-drot-dist"
}

$launchArgs = @()
if ($Repository) {
    $launchArgs += "-r", $Repository
} elseif ($DrotArgs -notcontains "-r" -and $DrotArgs -notcontains "--repository") {
    $launchArgs += "-r", $repoRoot
}
$launchArgs += $DrotArgs

& $bin @launchArgs
exit $LASTEXITCODE

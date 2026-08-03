# Ensures target/dist/drot (+ drot_tui) match tooling/drot, then launches the tool.
#
# Default: interactive TUI (drot_tui) — for developers.
# Agents / scripts: pass -Cli / --cli to run the direct CLI (drot).
#
# Examples:
#   ./tooling/scripts/run-drot.ps1
#   ./tooling/scripts/run-drot.ps1 -Cli --yes quality run
#   ./tooling/scripts/run-drot.ps1 -Cli --yes build run --skip-verify
[CmdletBinding(PositionalBinding = $false)]
param(
    # Named switch (not ForceBuild+Alias): PositionalBinding=$false + Alias("Force") was unreliable.
    [switch] $Force,
    [switch] $Cli,
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

# Remaining command tokens imply CLI even without -Cli (TUI does not take subcommands).
$useCli = [bool]$Cli -or ($null -ne $DrotArgs -and $DrotArgs.Count -gt 0)

if ($useCli) {
    $bin = Join-Path $repoRoot "target\dist\drot.exe"
} else {
    $bin = Join-Path $repoRoot "target\dist\drot_tui.exe"
}

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

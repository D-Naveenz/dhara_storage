# Ensures target/dist/drot (+ drot_tui) match tooling/drot, then launches the tool.
#
# Default: interactive TUI (drot_tui) — for developers.
# Agents / scripts: pass -Cli / --cli to run the direct CLI (drot).
#
# Examples:
#   ./tooling/scripts/run-drot.ps1
#   ./tooling/scripts/run-drot.ps1 -Cli --yes quality run
#   ./tooling/scripts/run-drot.ps1 -Cli --yes build run --skip-verify
param(
    [Alias("Force")]
    [switch] $ForceBuild,
    [switch] $Cli,
    [string] $Repository,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $DrotArgs
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $repoRoot

$ensureArgs = @()
if ($ForceBuild) {
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

$args = @()
if ($Repository) {
    $args += "-r", $Repository
} elseif ($DrotArgs -notcontains "-r" -and $DrotArgs -notcontains "--repository") {
    $args += "-r", $repoRoot
}
$args += $DrotArgs

& $bin @args
exit $LASTEXITCODE

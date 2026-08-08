param(
    [switch] $SkipDocs,
    [switch] $SkipDotnet
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $repoRoot

$launchArgs = @("--yes", "quality", "run")
if ($SkipDocs) { $launchArgs += "--skip-docs" }
if ($SkipDotnet) { $launchArgs += "--skip-dotnet" }

& (Join-Path $PSScriptRoot "run-drot.ps1") @launchArgs
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

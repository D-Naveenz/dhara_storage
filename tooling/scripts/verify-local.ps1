param(
    [switch] $SkipDocs,
    [switch] $SkipDotnet
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $repoRoot

$args = @("--yes", "quality", "run")
if ($SkipDocs) { $args += "--skip-docs" }
if ($SkipDotnet) { $args += "--skip-dotnet" }

& (Join-Path $PSScriptRoot "run-drot.ps1") @args
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

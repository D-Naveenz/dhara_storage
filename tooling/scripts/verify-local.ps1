param(
    [switch] $SkipDocs,
    [switch] $SkipDotnet
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $repoRoot

$args = @("quality", "run")
if ($SkipDocs) { $args += "--skip-docs" }
if ($SkipDotnet) { $args += "--skip-dotnet" }

cargo run -p dhara_tool -- @args
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

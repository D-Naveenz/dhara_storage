param(
    [switch] $SkipDocs,
    [switch] $SkipDotnet
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $repoRoot

& (Join-Path $PSScriptRoot "ensure-dhara-tool-dist.ps1")
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$bin = Join-Path $repoRoot "target\dist\dhara_tool.exe"
$args = @("--yes", "quality", "run")
if ($SkipDocs) { $args += "--skip-docs" }
if ($SkipDotnet) { $args += "--skip-dotnet" }

& $bin @args
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

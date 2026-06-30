param(
    [string] $Configuration = "Release",
    [string] $NativeStage = "tooling/artifacts/native-stage",
    [string] $ToolPath = "target\ci\dhara_tool.exe"
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $repoRoot

cargo build -p dhara_tool --profile ci

& $ToolPath verify package --configuration $Configuration --native-stage $NativeStage
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

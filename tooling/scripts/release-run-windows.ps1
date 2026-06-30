param(
    [string] $NativeStage = "tooling/artifacts/native-stage",
    [string] $PrepackedNuget = "",
    [switch] $DryRun,
    [switch] $SkipCargo,
    [switch] $SkipNuget,
    [switch] $VerifyPackage,
    [string] $ToolPath = "target\ci\dhara_tool.exe"
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $repoRoot

if (-not (Test-Path -LiteralPath $ToolPath)) {
    cargo build -p dhara_tool --profile ci
}

$toolArgs = @(
    "release", "run",
    "--native-stage", $NativeStage
)

if ($PrepackedNuget) {
    $toolArgs += @("--prepacked-nuget", $PrepackedNuget)
}
if ($DryRun) { $toolArgs += "--dry-run" }
if ($SkipCargo) { $toolArgs += "--skip-cargo" }
if ($SkipNuget) { $toolArgs += "--skip-nuget" }
if ($VerifyPackage) { $toolArgs += "--verify-package" }

$vsInstall = & "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" `
    -latest -products * `
    -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 Microsoft.VisualStudio.Component.VC.Tools.ARM64 `
    -property installationPath

if ([string]::IsNullOrWhiteSpace($vsInstall)) {
    throw "Visual Studio with x64 and ARM64 MSVC build tools was not found."
}

$vcvars = Join-Path $vsInstall "VC\Auxiliary\Build\vcvarsall.bat"
$command = "call `"$vcvars`" x64_arm64 && `"$ToolPath`" " + ($toolArgs -join " ")
& cmd.exe /d /c $command
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

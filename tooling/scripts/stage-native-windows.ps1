param(
    [string] $ToolPath = "target\ci\dhara_tool.exe"
)

$ErrorActionPreference = "Stop"

$vsInstall = & "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" `
    -latest -products * `
    -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 Microsoft.VisualStudio.Component.VC.Tools.ARM64 `
    -property installationPath

if ([string]::IsNullOrWhiteSpace($vsInstall)) {
    throw "Visual Studio with x64 and ARM64 MSVC build tools was not found."
}

$vcvars = Join-Path $vsInstall "VC\Auxiliary\Build\vcvarsall.bat"
$command = "call `"$vcvars`" x64_arm64 && `"$ToolPath`" package stage-native"
& cmd.exe /d /c $command
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

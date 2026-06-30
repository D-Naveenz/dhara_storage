# Ensures target/dist/dhara_tool matches tooling/dhara_tool/Cargo.toml package.version.
# Bump tool version (and config sync) when shipping tool changes — same policy as CI cache.
param(
    [switch] $Force
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $repoRoot

$manifestPath = Join-Path $repoRoot "tooling\dhara_tool\Cargo.toml"
$manifestContent = Get-Content $manifestPath -Raw
if ($manifestContent -notmatch '(?m)^version\s*=\s*"([^"]+)"') {
    throw "missing package.version in $manifestPath"
}
$expectedVersion = $Matches[1]

$bin = Join-Path $repoRoot "target\dist\dhara_tool.exe"
$needBuild = [bool]$Force

if (-not $needBuild) {
    if (-not (Test-Path $bin)) {
        Write-Host "build: dist missing (manifest v$expectedVersion)"
        $needBuild = $true
    }
    else {
        $builtVersion = (& $bin --version).Trim()
        if ($builtVersion -ne $expectedVersion) {
            Write-Host "build: dist v$builtVersion != manifest v$expectedVersion"
            $needBuild = $true
        }
        else {
            Write-Host "skip: dist v$expectedVersion current"
        }
    }
}
else {
    Write-Host "build: -Force requested"
}

if ($needBuild) {
    cargo build -p dhara_tool --profile dist
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }

    $builtVersion = (& $bin --version).Trim()
    if ($builtVersion -ne $expectedVersion) {
        Write-Error "smoke failed: dist reports v$builtVersion, expected v$expectedVersion"
        exit 1
    }

    Write-Host "built: dist v$expectedVersion"
}

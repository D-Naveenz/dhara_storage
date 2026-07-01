# Ensures target/dist/dhara_tool matches tooling/dhara_tool/Cargo.toml workspace.package.version.
# Bump [tool].version in dhara.config.toml and workspace.package.version in tooling/dhara_tool/Cargo.toml together when shipping tool changes.
param(
    [switch] $Force
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $repoRoot

$manifestPath = Join-Path $repoRoot "tooling\dhara_tool\Cargo.toml"
$manifestContent = Get-Content $manifestPath -Raw
if ($manifestContent -notmatch '(?ms)\[workspace\.package\][^\[]*?version\s*=\s*"([^"]+)"') {
    throw "missing workspace.package.version in $manifestPath"
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

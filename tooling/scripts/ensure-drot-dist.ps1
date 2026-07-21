# Ensures target/dist/drot matches tooling/drot/Cargo.toml workspace.package.version.
# Tool version lives only in the DROT submodule (https://github.com/D-Naveenz/dhara_repo_orchestration).
param(
    [switch] $Force
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $repoRoot

$manifestPath = Join-Path $repoRoot "tooling\drot\Cargo.toml"
if (-not (Test-Path $manifestPath)) {
    throw "DROT submodule missing at tooling\drot - run: git submodule update --init --recursive"
}
$manifestContent = Get-Content $manifestPath -Raw
if ($manifestContent -notmatch '(?ms)\[workspace\.package\][^\[]*?version\s*=\s*"([^"]+)"') {
    throw "missing workspace.package.version in $manifestPath"
}
$expectedVersion = $Matches[1]

$bin = Join-Path $repoRoot "target\dist\drot.exe"
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
    $env:CARGO_TARGET_DIR = Join-Path $repoRoot "target"
    cargo build --manifest-path tooling/drot/Cargo.toml -p drot --profile dist
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

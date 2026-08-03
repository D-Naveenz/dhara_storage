# Ensures target/dist/drot (+ drot_tui) match tooling/drot/Cargo.toml workspace.package.version.
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

$cliBin = Join-Path $repoRoot "target\dist\drot.exe"
$tuiBin = Join-Path $repoRoot "target\dist\drot_tui.exe"
$needBuild = [bool]$Force

if (-not $needBuild) {
    if (-not (Test-Path $cliBin) -or -not (Test-Path $tuiBin)) {
        Write-Host "build: dist missing CLI and/or TUI (manifest v$expectedVersion)"
        $needBuild = $true
    }
    else {
        $builtVersion = (& $cliBin --version).Trim()
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
    cargo build --manifest-path tooling/drot/Cargo.toml -p drot -p drot_tui --profile dist
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }

    if (-not (Test-Path $tuiBin)) {
        Write-Error "smoke failed: drot_tui missing at $tuiBin after dist build"
        exit 1
    }

    $builtVersion = (& $cliBin --version).Trim()
    if ($builtVersion -ne $expectedVersion) {
        Write-Error "smoke failed: dist reports v$builtVersion, expected v$expectedVersion"
        exit 1
    }

    Write-Host "built: dist v$expectedVersion (drot + drot_tui)"
}

# Ensures target/dist/drot (+ drot_tui) match the tooling/drot git checkout.
# Stamp: target/dist/.drot-git-rev (full HEAD SHA when the tree was clean at build time).
# Tool version still lives in tooling/drot/Cargo.toml; rebuild gating is by git, not semver.
param(
    [switch] $Force
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $repoRoot

$drotRoot = Join-Path $repoRoot "tooling\drot"
$manifestPath = Join-Path $drotRoot "Cargo.toml"
if (-not (Test-Path $manifestPath)) {
    throw "DROT submodule missing at tooling\drot - run: git submodule update --init --recursive"
}

$cliBin = Join-Path $repoRoot "target\dist\drot.exe"
$tuiBin = Join-Path $repoRoot "target\dist\drot_tui.exe"
$stampPath = Join-Path $repoRoot "target\dist\.drot-git-rev"
$needBuild = [bool]$Force

function Get-DrotGitHead {
    $raw = & git -C $drotRoot rev-parse HEAD
    if ($LASTEXITCODE -ne 0) {
        throw "failed to read git HEAD in tooling\drot (exit $LASTEXITCODE)"
    }
    $head = ("$raw").Trim()
    if ([string]::IsNullOrWhiteSpace($head)) {
        throw "failed to read git HEAD in tooling\drot (empty)"
    }
    return $head
}

function Test-DrotGitDirty {
    $porcelain = & git -C $drotRoot status --porcelain
    if ($LASTEXITCODE -ne 0) {
        throw "failed to read git status in tooling\drot (exit $LASTEXITCODE)"
    }
    foreach ($line in @($porcelain)) {
        if (-not [string]::IsNullOrWhiteSpace("$line")) {
            return $true
        }
    }
    return $false
}

if ($Force) {
    Write-Host "build: -Force requested"
}

$head = Get-DrotGitHead
$dirty = Test-DrotGitDirty

if (-not $needBuild) {
    if (-not (Test-Path $cliBin) -or -not (Test-Path $tuiBin)) {
        Write-Host "build: dist missing CLI and/or TUI"
        $needBuild = $true
    }
    elseif ($dirty) {
        # Stamp is only trustworthy on a clean tree; local edits must rebuild.
        Write-Host "build: tooling/drot has uncommitted changes"
        $needBuild = $true
    }
    elseif (-not (Test-Path $stampPath)) {
        Write-Host "build: missing dist git stamp (.drot-git-rev)"
        $needBuild = $true
    }
    else {
        $stamped = (Get-Content -Path $stampPath -Raw).Trim()
        if ($stamped -ne $head) {
            Write-Host "build: dist stamp $($stamped.Substring(0, [Math]::Min(12, $stamped.Length))) != HEAD $($head.Substring(0, 12))"
            $needBuild = $true
        }
        else {
            Write-Host "skip: dist matches tooling/drot @$($head.Substring(0, 12))"
        }
    }
}

if ($needBuild) {
    $env:CARGO_TARGET_DIR = Join-Path $repoRoot "target"
    # cargo prints progress on stderr; do not let that trip ErrorAction Stop.
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    cargo build --manifest-path tooling/drot/Cargo.toml -p drot -p drot_tui --profile dist
    $cargoExit = $LASTEXITCODE
    $ErrorActionPreference = $prevEap
    if ($cargoExit -ne 0) {
        exit $cargoExit
    }

    if (-not (Test-Path $cliBin) -or -not (Test-Path $tuiBin)) {
        Write-Error "smoke failed: drot and/or drot_tui missing under target\dist after dist build"
        exit 1
    }

    $distDir = Split-Path $stampPath -Parent
    if (-not (Test-Path $distDir)) {
        New-Item -ItemType Directory -Path $distDir | Out-Null
    }

    # Re-read after build: only stamp a clean HEAD (dirty builds stay marked so the next ensure rebuilds).
    $headAfter = Get-DrotGitHead
    $dirtyAfter = Test-DrotGitDirty
    if ($dirtyAfter) {
        Set-Content -Path $stampPath -Value "dirty $headAfter" -NoNewline
        Write-Host "built: dist from dirty tooling/drot @$($headAfter.Substring(0, 12)) (will rebuild until clean)"
    }
    else {
        Set-Content -Path $stampPath -Value $headAfter -NoNewline
        Write-Host "built: dist from tooling/drot @$($headAfter.Substring(0, 12))"
    }
}

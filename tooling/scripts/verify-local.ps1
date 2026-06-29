param(
    [switch] $SkipDocs
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $repoRoot

function Invoke-Step {
    param(
        [string] $Label,
        [scriptblock] $Command
    )
    Write-Host "==> $Label"
    & $Command
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

Invoke-Step "cargo fmt --check" {
    cargo fmt -p dhara_storage_dal -p dhara_storage -p dharastorage -p dhara_tool --check
}

Invoke-Step "cargo clippy (dhara_storage)" {
    cargo clippy -p dhara_storage --all-targets --all-features -- -D warnings
}

Invoke-Step "cargo clippy (other crates)" {
    cargo clippy -p dhara_storage_dal -p dharastorage -p dhara_tool --all-targets -- -D warnings
}

if (-not $SkipDocs) {
    Invoke-Step "cargo doc" {
        cargo doc -p dhara_storage --no-deps --all-features
        cargo doc -p dhara_storage_dal -p dharastorage -p dhara_tool --no-deps
    }
}

Invoke-Step "cargo test (dhara_storage)" {
    cargo test -p dhara_storage --all-features
}

Invoke-Step "cargo test (dhara_storage_dal)" {
    cargo test -p dhara_storage_dal
}

Invoke-Step "cargo test (dharastorage)" {
    cargo test -p dharastorage
}

$testsProject = "src/bindings/Dhara.Storage.Tests/Dhara.Storage.Tests.csproj"
if (Get-Command dotnet -ErrorAction SilentlyContinue) {
    Invoke-Step "dotnet test" {
        dotnet test $testsProject
    }
} else {
    Write-Warning "dotnet not found; skipping .NET tests"
}

Write-Host "Local CI checks passed."

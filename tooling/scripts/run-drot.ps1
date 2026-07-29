# Ensures target/dist/drot matches tooling/drot/Cargo.toml, then runs drot with -r <repo>.
param(
    [Alias("Force")]
    [switch] $ForceBuild,
    [string] $Repository,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $DrotArgs
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $repoRoot

$ensureArgs = @()
if ($ForceBuild) {
    $ensureArgs += "-Force"
}
& (Join-Path $PSScriptRoot "ensure-drot-dist.ps1") @ensureArgs
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$bin = Join-Path $repoRoot "target\dist\drot.exe"
if (-not (Test-Path $bin)) {
    throw "drot binary missing at $bin after ensure-drot-dist"
}

$args = @()
if ($Repository) {
    $args += "-r", $Repository
} elseif ($DrotArgs -notcontains "-r" -and $DrotArgs -notcontains "--repository") {
    $args += "-r", $repoRoot
}
$args += $DrotArgs

& $bin @args
exit $LASTEXITCODE

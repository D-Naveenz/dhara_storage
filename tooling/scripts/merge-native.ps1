param(
    [Parameter(Mandatory = $true)]
    [string] $Output,
    [Parameter(Mandatory = $true)]
    [string[]] $StagePaths
)

$ErrorActionPreference = "Stop"

if (Test-Path -LiteralPath $Output) {
    Remove-Item -LiteralPath $Output -Recurse -Force
}
New-Item -ItemType Directory -Path $Output -Force | Out-Null

foreach ($stage in $StagePaths) {
    $runtimes = Join-Path $stage "runtimes"
    if (-not (Test-Path -LiteralPath $runtimes)) {
        throw "native stage input '$stage' is missing a runtimes directory"
    }

    Get-ChildItem -LiteralPath $runtimes -Directory | ForEach-Object {
        $destination = Join-Path $Output "runtimes" $_.Name
        Copy-Item -LiteralPath $_.FullName -Destination $destination -Recurse -Force
    }
}

Write-Host "Merged native stages into $Output"

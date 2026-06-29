param(
    [Parameter(Mandatory = $true)]
    [string] $Output,
    [Parameter(Mandatory = $true)]
    [string[]] $Input
)

$ErrorActionPreference = "Stop"

function Copy-RuntimesTree {
    param(
        [string] $Source,
        [string] $Destination
    )

    if (-not (Test-Path -LiteralPath $Source)) {
        return
    }

    Get-ChildItem -LiteralPath $Source -Force | ForEach-Object {
        $target = Join-Path $Destination $_.Name
        if ($_.PSIsContainer) {
            New-Item -ItemType Directory -Path $target -Force | Out-Null
            Copy-RuntimesTree -Source $_.FullName -Destination $target
        }
        elseif ($_.Mode -notlike "d*") {
            $parent = Split-Path -Parent $target
            if ($parent) {
                New-Item -ItemType Directory -Path $parent -Force | Out-Null
            }
            Copy-Item -LiteralPath $_.FullName -Destination $target -Force
        }
    }
}

if (Test-Path -LiteralPath $Output) {
    Remove-Item -LiteralPath $Output -Recurse -Force
}
New-Item -ItemType Directory -Path $Output -Force | Out-Null

foreach ($stage in $Input) {
    $runtimes = Join-Path $stage "runtimes"
    if (-not (Test-Path -LiteralPath $runtimes)) {
        throw "native stage input '$stage' is missing a runtimes directory"
    }
    Copy-RuntimesTree -Source $runtimes -Destination (Join-Path $Output "runtimes")
}

Write-Host "Merged native stages into $Output"

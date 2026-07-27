# Diagnose and repair GitHub SSH + Git LFS on Windows.
# Default: analyze and suggest. Use -Repair or -Recreate to act immediately.
param(
    [switch] $Repair,
    [switch] $Recreate,
    [switch] $Yes,
    [string] $Email,
    [string] $KeyTitle,
    [string] $KeyPath = (Join-Path $env:USERPROFILE ".ssh\id_ed25519")
)

$ErrorActionPreference = "Stop"

$script:OpenSshExe = Join-Path $env:WINDIR "System32\OpenSSH\ssh.exe"
$script:ExpectedSshCommand = "C:/Windows/System32/OpenSSH/ssh.exe"
$script:Findings = [System.Collections.Generic.List[object]]::new()

function Write-Check {
    param(
        [ValidateSet("OK", "WARN", "FAIL")]
        [string] $Level,
        [string] $Message
    )

    $prefix = "[$Level]"
    switch ($Level) {
        "OK" { Write-Host "$prefix $Message" -ForegroundColor Green }
        "WARN" { Write-Host "$prefix $Message" -ForegroundColor Yellow }
        "FAIL" { Write-Host "$prefix $Message" -ForegroundColor Red }
    }

    $script:Findings.Add([pscustomobject]@{ Level = $Level; Message = $Message })
}

function Test-CommandExists {
    param([string] $Name)

    if ($Name -match '[\\/]') {
        return Test-Path -LiteralPath $Name
    }

    return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

function Test-IsAdministrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Get-KeyFingerprint {
    param([string] $PublicKeyPath)

    if (-not (Test-Path -LiteralPath $PublicKeyPath)) {
        return $null
    }

    $output = & ssh-keygen -lf $PublicKeyPath 2>$null
    if ($LASTEXITCODE -ne 0 -or -not $output) {
        return $null
    }

    return ($output -split '\s+')[1]
}

function Get-SshAgentState {
    $service = Get-Service -Name ssh-agent -ErrorAction SilentlyContinue
    if (-not $service) {
        return [pscustomobject]@{
            Exists = $false
            Running = $false
            StartupType = $null
        }
    }

    return [pscustomobject]@{
        Exists = $true
        Running = $service.Status -eq "Running"
        StartupType = $service.StartType
    }
}

function Get-LoadedSshKeyFingerprints {
    $output = & ssh-add -l 2>$null
    if ($LASTEXITCODE -ne 0 -or -not $output) {
        return @()
    }

    $fingerprints = [System.Collections.Generic.List[string]]::new()
    foreach ($line in @($output)) {
        if ($line -match 'The agent has no identities') {
            continue
        }
        $parts = $line -split '\s+'
        if ($parts.Count -ge 2) {
            $fingerprints.Add($parts[1])
        }
    }

    return $fingerprints
}

function Get-GitSshCommandState {
    $value = git config --global --get core.sshCommand 2>$null
    if (-not $value) {
        return [pscustomobject]@{
            Configured = $false
            Value = $null
            UsesWindowsOpenSsh = $false
        }
    }

    $normalized = $value.Trim().Trim('"').Replace('\', '/')
    $expected = $script:ExpectedSshCommand
    $usesWindowsOpenSsh = $normalized -ieq $expected

    return [pscustomobject]@{
        Configured = $true
        Value = $value
        UsesWindowsOpenSsh = $usesWindowsOpenSsh
    }
}

function Test-GhAuthenticated {
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $lines = @(& gh auth status 2>&1 | ForEach-Object { "$_" })
        $output = ($lines -join "`n").Trim()
    }
    finally {
        $ErrorActionPreference = $prevEap
    }

    if ($LASTEXITCODE -ne 0) {
        return [pscustomobject]@{
            Authenticated = $false
            Detail = $output
        }
    }

    $login = $null
    if ($output -match 'Logged in to github\.com account (\S+)') {
        $login = $Matches[1]
    }

    return [pscustomobject]@{
        Authenticated = $true
        Login = $login
        Detail = $output
    }
}

function Test-GitLfsHooksInstalled {
    foreach ($scope in @("--global", "--system")) {
        $clean = git config $scope --get filter.lfs.clean 2>$null
        if ($clean) {
            return $true
        }
    }

    return $false
}

function Test-GitHubSsh {
    param([switch] $BatchMode)

    $sshArgs = @(
        "-T",
        "-o", "StrictHostKeyChecking=accept-new",
        "git@github.com"
    )
    if ($BatchMode) {
        $sshArgs = @("-T", "-o", "BatchMode=yes", "-o", "StrictHostKeyChecking=accept-new", "git@github.com")
    }

    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $lines = @(& $script:OpenSshExe @sshArgs 2>&1 | ForEach-Object { "$_" })
        $output = ($lines -join "`n").Trim()
    }
    finally {
        $ErrorActionPreference = $prevEap
    }

    $success = $output -match 'successfully authenticated'

    return [pscustomobject]@{
        Success = $success
        Output = $output
    }
}

function Get-GitRepoRoot {
    $root = git rev-parse --show-toplevel 2>$null
    if ($LASTEXITCODE -ne 0 -or -not $root) {
        return $null
    }

    return (Resolve-Path $root).Path
}

function Test-RepoLfsHealth {
    param([string] $RepoRoot)

    if (-not $RepoRoot) {
        return [pscustomobject]@{
            InRepo = $false
            HasLfsFiles = $false
            NeedsRepair = $false
            Detail = $null
        }
    }

    Push-Location $RepoRoot
    try {
        $lfsFiles = & git lfs ls-files 2>$null
        if ($LASTEXITCODE -ne 0) {
            return [pscustomobject]@{
                InRepo = $true
                HasLfsFiles = $false
                NeedsRepair = $false
                Detail = "git lfs ls-files failed"
            }
        }

        $hasLfs = [bool](@($lfsFiles) | Where-Object { $_ })
        if (-not $hasLfs) {
            return [pscustomobject]@{
                InRepo = $true
                HasLfsFiles = $false
                NeedsRepair = $false
                Detail = "no LFS-tracked files in this repo"
            }
        }

        $status = git status --porcelain 2>$null
        $dirtyLfs = $false
        foreach ($line in @($status)) {
            if ($line -match '^(..)\s+(.+)$') {
                $path = $Matches[2]
                $check = & git check-attr filter -- $path 2>$null
                if ($check -match 'filter:\s+lfs') {
                    $dirtyLfs = $true
                    break
                }
            }
        }

        $missingPointer = $false
        $samplePath = "bindings/csharp/Dhara.Storage/assets/dhara-logo-colored_sm.png"
        if (Test-Path -LiteralPath $samplePath) {
            $head = Get-Content -LiteralPath $samplePath -TotalCount 1 -ErrorAction SilentlyContinue
            if ($head -match '^version https://git-lfs.github.com/spec/v1') {
                $missingPointer = $true
            }
        }

        return [pscustomobject]@{
            InRepo = $true
            HasLfsFiles = $true
            NeedsRepair = $dirtyLfs -or $missingPointer
            Detail = if ($missingPointer) {
                "LFS pointer file present where smudged content is expected"
            }
            elseif ($dirtyLfs) {
                "working tree has unresolved LFS files"
            }
            else {
                "LFS files appear checked out"
            }
        }
    }
    finally {
        Pop-Location
    }
}

function Initialize-SshAgentService {
    $state = Get-SshAgentState
    if (-not $state.Exists) {
        throw "ssh-agent service is not installed. Install the Windows OpenSSH Client optional feature."
    }

    $needsConfig = $state.StartupType -ne "Automatic" -or -not $state.Running
    if (-not $needsConfig) {
        Write-Host "skip: ssh-agent already automatic and running"
        return
    }

    if (-not (Test-IsAdministrator)) {
        Write-Host ""
        Write-Host "ssh-agent must be enabled by an administrator. Run this in an elevated PowerShell window:"
        Write-Host ""
        Write-Host '  Set-Service -Name ssh-agent -StartupType Automatic; Start-Service ssh-agent'
        Write-Host ""
        exit 2
    }

    if ($state.StartupType -ne "Automatic") {
        Set-Service -Name ssh-agent -StartupType Automatic
        Write-Host "set: ssh-agent startup type Automatic"
    }

    if (-not $state.Running) {
        Start-Service ssh-agent
        Write-Host "start: ssh-agent"
    }
}

function Set-GitSshCommand {
    $current = git config --global --get core.sshCommand 2>$null
    if ($current -eq $script:ExpectedSshCommand) {
        Write-Host "skip: core.sshCommand already set"
        return
    }

    git config --global core.sshCommand $script:ExpectedSshCommand
    if ($LASTEXITCODE -ne 0) {
        throw "failed to set git config --global core.sshCommand"
    }

    Write-Host "set: git config --global core.sshCommand $script:ExpectedSshCommand"
}

function Install-GitLfsHooks {
    & git lfs install 2>&1 | ForEach-Object { Write-Host $_ }
    if ($LASTEXITCODE -ne 0) {
        throw "git lfs install failed"
    }
}

function Add-SshKeyToAgent {
    param([string] $PrivateKeyPath)

    if (-not (Test-Path -LiteralPath $PrivateKeyPath)) {
        throw "private key not found: $PrivateKeyPath"
    }

    $fingerprint = Get-KeyFingerprint -PublicKeyPath "$PrivateKeyPath.pub"
    $loaded = Get-LoadedSshKeyFingerprints
    if ($fingerprint -and ($loaded -contains $fingerprint)) {
        Write-Host "skip: key already loaded in ssh-agent ($fingerprint)"
        return
    }

    & ssh-add $PrivateKeyPath
    if ($LASTEXITCODE -ne 0) {
        throw "ssh-add failed for $PrivateKeyPath"
    }

    Write-Host "added: $PrivateKeyPath to ssh-agent"
}

function Repair-RepoLfsCheckout {
    param([string] $RepoRoot)

    if (-not $RepoRoot) {
        Write-Host "skip: not inside a git repository"
        return
    }

    Push-Location $RepoRoot
    try {
        Write-Host "repair: git lfs pull"
        & git lfs pull
        if ($LASTEXITCODE -ne 0) {
            throw "git lfs pull failed"
        }

        Write-Host "repair: restore working tree from HEAD"
        & git restore --source=HEAD :/ 2>$null
        if ($LASTEXITCODE -ne 0) {
            & git checkout -f HEAD
            if ($LASTEXITCODE -ne 0) {
                throw "failed to restore working tree after git lfs pull"
            }
        }
    }
    finally {
        Pop-Location
    }
}

function Invoke-SshSetupAnalysis {
    param([string] $PrivateKeyPath)

    $publicKeyPath = "$PrivateKeyPath.pub"
    Write-Host ""
    Write-Host "=== GitHub SSH + LFS analysis ==="
    Write-Host ""

    $requiredCommands = @("git", "git-lfs", "gh", "ssh", "ssh-keygen", "ssh-add")
    $missing = @($requiredCommands | Where-Object { -not (Test-CommandExists $_) })
    if ($missing.Count -gt 0) {
        Write-Check -Level FAIL -Message ("missing commands: {0}" -f ($missing -join ", "))
    }
    else {
        Write-Check -Level OK -Message "required commands available (git, git-lfs, gh, ssh, ssh-keygen, ssh-add)"
    }

    if (Test-CommandExists $script:OpenSshExe) {
        Write-Check -Level OK -Message "Windows OpenSSH client found at $script:OpenSshExe"
    }
    else {
        Write-Check -Level FAIL -Message "Windows OpenSSH client missing at $script:OpenSshExe"
    }

    $ghAuth = Test-GhAuthenticated
    if ($ghAuth.Authenticated) {
        $who = if ($ghAuth.Login) { $ghAuth.Login } else { "github.com" }
        Write-Check -Level OK -Message "gh authenticated ($who)"
    }
    else {
        Write-Check -Level WARN -Message "gh not authenticated; run: gh auth login"
    }

    if ((Test-Path -LiteralPath $PrivateKeyPath) -and (Test-Path -LiteralPath $publicKeyPath)) {
        Write-Check -Level OK -Message "local key pair exists at $PrivateKeyPath"
    }
    else {
        Write-Check -Level FAIL -Message "local key pair missing at $PrivateKeyPath"
    }

    $agent = Get-SshAgentState
    if (-not $agent.Exists) {
        Write-Check -Level FAIL -Message "ssh-agent service not installed"
    }
    elseif ($agent.StartupType -ne "Automatic") {
        Write-Check -Level WARN -Message "ssh-agent startup type is $($agent.StartupType) (expected Automatic)"
    }
    elseif (-not $agent.Running) {
        Write-Check -Level WARN -Message "ssh-agent service is stopped"
    }
    else {
        Write-Check -Level OK -Message "ssh-agent service is Automatic and running"
    }

    $fingerprint = Get-KeyFingerprint -PublicKeyPath $publicKeyPath
    $loaded = Get-LoadedSshKeyFingerprints
    if ($loaded.Count -eq 0) {
        Write-Check -Level FAIL -Message "no SSH keys loaded in ssh-agent (Git LFS background requests will fail)"
    }
    elseif ($fingerprint -and ($loaded -contains $fingerprint)) {
        Write-Check -Level OK -Message "expected key loaded in ssh-agent ($fingerprint)"
    }
    elseif ($loaded.Count -gt 0) {
        Write-Check -Level WARN -Message "ssh-agent has keys loaded, but not the expected key at $PrivateKeyPath"
    }

    $sshCmd = Get-GitSshCommandState
    if (-not $sshCmd.Configured) {
        Write-Check -Level WARN -Message "git core.sshCommand is unset (Git may not use Windows OpenSSH agent)"
    }
    elseif ($sshCmd.UsesWindowsOpenSsh) {
        Write-Check -Level OK -Message "git core.sshCommand uses Windows OpenSSH"
    }
    else {
        Write-Check -Level WARN -Message "git core.sshCommand is '$($sshCmd.Value)' (expected $script:ExpectedSshCommand)"
    }

    if (Test-CommandExists "git-lfs") {
        $version = (& git lfs version 2>$null).Trim()
        if (Test-GitLfsHooksInstalled) {
            Write-Check -Level OK -Message "Git LFS hooks installed ($version)"
        }
        else {
            Write-Check -Level WARN -Message "Git LFS hooks not detected ($version); run: git lfs install"
        }
    }

    $github = Test-GitHubSsh -BatchMode
    if ($github.Success) {
        Write-Check -Level OK -Message "GitHub SSH authentication succeeded (batch mode)"
    }
    else {
        Write-Check -Level FAIL -Message "GitHub SSH authentication failed in batch mode"
        if ($github.Output) {
            Write-Host "      $($github.Output)" -ForegroundColor DarkGray
        }
    }

    $repoRoot = Get-GitRepoRoot
    if ($repoRoot) {
        $lfsHealth = Test-RepoLfsHealth -RepoRoot $repoRoot
        if ($lfsHealth.NeedsRepair) {
            Write-Check -Level WARN -Message "repo LFS checkout needs repair ($($lfsHealth.Detail))"
        }
        elseif ($lfsHealth.HasLfsFiles) {
            Write-Check -Level OK -Message "repo LFS checkout looks healthy ($($lfsHealth.Detail))"
        }
        else {
            Write-Check -Level OK -Message "in git repo at $repoRoot ($($lfsHealth.Detail))"
        }
    }
    else {
        Write-Check -Level OK -Message "not inside a git repository (repo LFS repair skipped)"
    }

    Write-Host ""
    Write-Host "=== Recommendations ==="
    Write-Host ""

    $failures = @($script:Findings | Where-Object { $_.Level -eq "FAIL" })
    $warnings = @($script:Findings | Where-Object { $_.Level -eq "WARN" })

    if ($failures.Count -eq 0 -and $warnings.Count -eq 0) {
        Write-Host "All checks passed. No action required."
        return
    }

    $suggestRepair = $false
    $suggestRecreate = $false

    foreach ($item in $failures + $warnings) {
        if ($item.Message -match 'local key pair missing') {
            $suggestRecreate = $true
        }
        if ($item.Message -match 'ssh-agent|core\.sshCommand|no SSH keys loaded|Git LFS hooks|LFS checkout needs repair|GitHub SSH authentication failed') {
            $suggestRepair = $true
        }
    }

    if ($suggestRepair) {
        Write-Host "  .\tooling\scripts\setup-github-ssh.ps1 -Repair"
    }
    if ($suggestRecreate) {
        Write-Host "  .\tooling\scripts\setup-github-ssh.ps1 -Recreate"
    }
    if (-not $suggestRepair -and -not $suggestRecreate) {
        Write-Host "  Review warnings above and re-run analysis after fixing prerequisites."
    }
}

function Invoke-SshSetupRepair {
    param(
        [string] $PrivateKeyPath,
        [switch] $SkipRepoRepair
    )

    Write-Host ""
    Write-Host "=== Repair GitHub SSH + LFS ==="
    Write-Host ""

    Initialize-SshAgentService
    Set-GitSshCommand
    Install-GitLfsHooks
    Add-SshKeyToAgent -PrivateKeyPath $PrivateKeyPath

    Write-Host "verify: ssh -T git@github.com"
    $github = Test-GitHubSsh
    if (-not $github.Success) {
        Write-Host $github.Output
        throw "GitHub SSH verification failed"
    }
    Write-Host $github.Output

    if (-not $SkipRepoRepair) {
        Repair-RepoLfsCheckout -RepoRoot (Get-GitRepoRoot)
    }

    Write-Host ""
    Write-Host "Repair complete."
}

function Get-GhSshKeyEntries {
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $lines = @(& gh ssh-key list 2>&1 | ForEach-Object { "$_" })
    }
    finally {
        $ErrorActionPreference = $prevEap
    }

    $entries = [System.Collections.Generic.List[object]]::new()
    foreach ($line in $lines) {
        if ($line -match '^(warning:|error:)' -or $line -match 'admin:ssh_signing_key') {
            continue
        }

        $parts = $line -split "`t"
        if ($parts.Count -ge 4 -and $parts[3] -match '^\d+$') {
            $keyParts = $parts[1].Trim() -split '\s+'
            $keyBlob = if ($keyParts.Count -ge 2) { $keyParts[1] } else { $null }
            $entries.Add([pscustomobject]@{
                Id = [int]$parts[3]
                Title = $parts[0].Trim()
                KeyBlob = $keyBlob
            })
        }
    }

    if ($entries.Count -eq 0 -and $LASTEXITCODE -ne 0) {
        $detail = ($lines -join "`n").Trim()
        throw "gh ssh-key list failed: $detail"
    }

    return $entries
}

function Remove-GitHubSshKeysForLocalKey {
    param(
        [string] $PublicKeyPath,
        [string] $PrivateKeyPath
    )

    $localKeyBlob = $null
    if (Test-Path -LiteralPath $PublicKeyPath) {
        $localParts = (Get-Content -LiteralPath $PublicKeyPath -Raw).Trim() -split '\s+'
        if ($localParts.Count -ge 2) {
            $localKeyBlob = $localParts[1]
        }
    }

    $keyBaseName = [System.IO.Path]::GetFileNameWithoutExtension($PrivateKeyPath)
    $entries = Get-GhSshKeyEntries

    if ($entries.Count -eq 0) {
        Write-Host "skip: no GitHub SSH keys registered"
        return
    }

    $toDelete = @($entries | Where-Object {
        ($localKeyBlob -and $_.KeyBlob -eq $localKeyBlob) -or
        ($_.Title -match [regex]::Escape($keyBaseName))
    })

    if ($toDelete.Count -eq 0) {
        Write-Host "skip: no matching GitHub SSH keys for $PublicKeyPath"
        return
    }

    foreach ($entry in $toDelete) {
        Write-Host "delete: GitHub SSH key id $($entry.Id) ($($entry.Title))"
        & gh ssh-key delete $entry.Id --yes
        if ($LASTEXITCODE -ne 0) {
            throw "gh ssh-key delete $($entry.Id) failed"
        }
    }
}

function Invoke-SshKeyRecreate {
    param(
        [string] $PrivateKeyPath,
        [string] $EmailAddress,
        [string] $Title,
        [switch] $ConfirmYes
    )

    Write-Host ""
    Write-Host "=== Recreate GitHub SSH key ==="
    Write-Host ""

    $ghAuth = Test-GhAuthenticated
    if (-not $ghAuth.Authenticated) {
        throw "gh is not authenticated. Run: gh auth login"
    }

    if (-not $ConfirmYes) {
        Write-Host "This will delete local keys at:"
        Write-Host "  $PrivateKeyPath"
        Write-Host "  $PrivateKeyPath.pub"
        Write-Host "and remove matching GitHub SSH keys, then generate a new Ed25519 key pair."
        $answer = Read-Host "Continue? [y/N]"
        if ($answer -notmatch '^[Yy]') {
            Write-Host "Aborted."
            exit 0
        }
    }

    $publicKeyPath = "$PrivateKeyPath.pub"
    Remove-GitHubSshKeysForLocalKey -PublicKeyPath $publicKeyPath -PrivateKeyPath $PrivateKeyPath

    foreach ($path in @($PrivateKeyPath, $publicKeyPath)) {
        if (Test-Path -LiteralPath $path) {
            Remove-Item -LiteralPath $path -Force
            Write-Host "removed: $path"
        }
    }

    if (-not $EmailAddress) {
        $EmailAddress = Read-Host "Email for ssh-keygen -C"
        if (-not $EmailAddress) {
            throw "email is required for ssh-keygen -C (pass -Email or enter at prompt)"
        }
    }

    $sshDir = Split-Path -Parent $PrivateKeyPath
    if (-not (Test-Path -LiteralPath $sshDir)) {
        New-Item -ItemType Directory -Path $sshDir -Force | Out-Null
    }

    Write-Host "generate: ssh-keygen -t ed25519 -C $EmailAddress -f $PrivateKeyPath"
    & ssh-keygen -t ed25519 -C $EmailAddress -f $PrivateKeyPath
    if ($LASTEXITCODE -ne 0) {
        throw "ssh-keygen failed"
    }

    if (-not $Title) {
        $Title = "{0} {1}" -f $env:COMPUTERNAME, (Get-Date -Format "yyyy-MM-dd")
    }

    Write-Host "upload: gh ssh-key add $publicKeyPath --title $Title"
    & gh ssh-key add $publicKeyPath --title $Title
    if ($LASTEXITCODE -ne 0) {
        throw "gh ssh-key add failed"
    }

    Invoke-SshSetupRepair -PrivateKeyPath $PrivateKeyPath
}

# --- main ---

if ($Repair -and $Recreate) {
    Write-Error "Use only one of -Repair or -Recreate."
    exit 2
}

$KeyPath = (Resolve-Path -LiteralPath $KeyPath -ErrorAction SilentlyContinue).Path
if (-not $KeyPath) {
    $KeyPath = (Join-Path $env:USERPROFILE ".ssh\id_ed25519")
}

if ($Recreate) {
    Invoke-SshKeyRecreate -PrivateKeyPath $KeyPath -EmailAddress $Email -Title $KeyTitle -ConfirmYes:$Yes
    exit 0
}

if ($Repair) {
    Invoke-SshSetupRepair -PrivateKeyPath $KeyPath
    exit 0
}

Invoke-SshSetupAnalysis -PrivateKeyPath $KeyPath

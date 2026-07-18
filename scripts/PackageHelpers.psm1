Set-StrictMode -Version Latest

function Get-NodeMajorVersion {
    param([Parameter(Mandatory)][string]$Version)

    if ($Version -notmatch '^v?(\d+)(?:\.|$)') {
        throw "Could not parse Node.js version '$Version'."
    }
    return [int]$Matches[1]
}

function Assert-SupportedNodeVersion {
    param([Parameter(Mandatory)][string]$Version)

    $major = Get-NodeMajorVersion -Version $Version
    if ($major -lt 20) {
        throw "Node.js 20 is required; found $Version."
    }
    if ($major -gt 20) {
        Write-Warning "Node.js 20 is supported; continuing with unverified $Version."
    }
}

function Assert-GameNotRunning {
    param([object[]]$Processes = @())

    if ($Processes.Count -gt 0) {
        $ids = ($Processes | ForEach-Object { $_.Id }) -join ', '
        throw "granblue_fantasy_relink.exe is running (PID: $ids). Exit the game before packaging."
    }
}

function Select-ProductMsi {
    param(
        [Parameter(Mandatory)][object[]]$Artifacts,
        [Parameter(Mandatory)][string]$ProductName,
        [Parameter(Mandatory)][string]$Version,
        [Parameter(Mandatory)][datetime]$BuildStartedAt
    )

    $expectedName = '^' + [regex]::Escape("${ProductName}_${Version}_x64_") + '[^\\]+\.msi$'
    $matches = @($Artifacts | Where-Object { $_.Name -match $expectedName })
    if ($matches.Count -ne 1) {
        throw "Expected exactly one ${ProductName} ${Version} x64 MSI; found $($matches.Count)."
    }
    if ($matches[0].LastWriteTimeUtc.ToUniversalTime() -lt $BuildStartedAt.ToUniversalTime()) {
        throw "The ${ProductName} MSI was not produced by the current build."
    }
    return $matches[0]
}

function Set-ArtifactHashesInText {
    param(
        [Parameter(Mandatory)][string]$Text,
        [Parameter(Mandatory)][string]$MsiHash,
        [Parameter(Mandatory)][string]$HookHash
    )

    foreach ($hash in @($MsiHash, $HookHash)) {
        if ($hash -notmatch '^[A-Fa-f0-9]{64}$') {
            throw "Invalid SHA-256 value '$hash'."
        }
    }

    $msiPattern = '(?m)(^- MSI(?: SHA-256)?: `)[A-Fa-f0-9]{64}(`\s*$)'
    $hookPattern = '(?m)(^- `hook\.dll`(?: SHA-256)?: `)[A-Fa-f0-9]{64}(`\s*$)'
    $msiMatches = [regex]::Matches($Text, $msiPattern)
    $hookMatches = [regex]::Matches($Text, $hookPattern)
    if ($msiMatches.Count -ne 1 -or $hookMatches.Count -ne 1) {
        throw "Expected exactly one MSI hash and one hook.dll hash; found $($msiMatches.Count) and $($hookMatches.Count)."
    }

    $normalizedMsi = $MsiHash.ToUpperInvariant()
    $normalizedHook = $HookHash.ToUpperInvariant()
    $updated = [regex]::Replace($Text, $msiPattern, { param($match) $match.Groups[1].Value + $normalizedMsi + $match.Groups[2].Value })
    return [regex]::Replace($updated, $hookPattern, { param($match) $match.Groups[1].Value + $normalizedHook + $match.Groups[2].Value })
}

function Invoke-NativeCommand {
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [string[]]$Arguments = @()
    )

    $output = & $FilePath @Arguments
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0) {
        throw "Command failed with exit code ${exitCode}: $FilePath $($Arguments -join ' ')"
    }
    return $output
}

Export-ModuleMember -Function Get-NodeMajorVersion, Assert-SupportedNodeVersion, Assert-GameNotRunning, Select-ProductMsi, Set-ArtifactHashesInText, Invoke-NativeCommand

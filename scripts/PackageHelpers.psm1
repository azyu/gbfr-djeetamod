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
    if ($major -ne 24) {
        throw "Node.js 24 is required; found $Version."
    }
}

function Assert-GameNotRunning {
    param([object[]]$Processes = @())

    if ($Processes.Count -gt 0) {
        $ids = ($Processes | ForEach-Object { $_.Id }) -join ', '
        throw "granblue_fantasy_relink.exe is running (PID: $ids). Exit the game before packaging."
    }
}

function Select-ProductNsisInstaller {
    param(
        [Parameter(Mandatory)][object[]]$Artifacts,
        [Parameter(Mandatory)][string]$ProductName,
        [Parameter(Mandatory)][string]$Version,
        [Parameter(Mandatory)][datetime]$BuildStartedAt
    )

    $expectedName = '^' + [regex]::Escape("${ProductName}_${Version}_x64-setup.exe") + '$'
    $matches = @($Artifacts | Where-Object { $_.Name -match $expectedName })
    if ($matches.Count -ne 1) {
        throw "Expected exactly one ${ProductName} ${Version} x64 NSIS installer; found $($matches.Count)."
    }
    if ($matches[0].LastWriteTimeUtc.ToUniversalTime() -lt $BuildStartedAt.ToUniversalTime()) {
        throw "The ${ProductName} NSIS installer was not produced by the current build."
    }
    return $matches[0]
}

function Assert-ReleaseVersionAgreement {
    param(
        [Parameter(Mandatory)][string]$RequestedVersion,
        [Parameter(Mandatory)][string]$PackageVersion,
        [Parameter(Mandatory)][string]$CargoVersion,
        [Parameter(Mandatory)][string]$TauriVersion
    )

    if ($RequestedVersion -notmatch '^\d+\.\d+\.\d+$') {
        throw "Release version must use stable X.Y.Z format; found '$RequestedVersion'."
    }

    $versions = [ordered]@{
        package = $PackageVersion
        cargo = $CargoVersion
        tauri = $TauriVersion
    }
    foreach ($entry in $versions.GetEnumerator()) {
        if ($entry.Value -ne $RequestedVersion) {
            throw "Release version '$RequestedVersion' does not match $($entry.Key) version '$($entry.Value)'."
        }
    }

    return $RequestedVersion
}

function Assert-UpdaterSigningEnvironment {
    param([Parameter(Mandatory)][System.Collections.IDictionary]$Values)

    foreach ($name in @('TAURI_PRIVATE_KEY', 'TAURI_KEY_PASSWORD')) {
        if (-not $Values.Contains($name) -or [string]::IsNullOrWhiteSpace([string]$Values[$name])) {
            throw "$name must be set for signed updater packaging."
        }
    }
}

function Select-ProductNsisUpdaterArtifacts {
    param(
        [Parameter(Mandatory)][object[]]$Artifacts,
        [Parameter(Mandatory)][string]$ProductName,
        [Parameter(Mandatory)][string]$Version,
        [Parameter(Mandatory)][datetime]$BuildStartedAt
    )

    $archiveName = "${ProductName}_${Version}_x64-setup.nsis.zip"
    $signatureName = "${archiveName}.sig"
    $archives = @($Artifacts | Where-Object { $_.Name -ceq $archiveName })
    $signatures = @($Artifacts | Where-Object { $_.Name -ceq $signatureName })
    if ($archives.Count -ne 1 -or $signatures.Count -ne 1) {
        throw "Expected exactly one ${ProductName} ${Version} updater archive/signature pair; found $($archives.Count) and $($signatures.Count)."
    }

    foreach ($artifact in @($archives[0], $signatures[0])) {
        if ($artifact.LastWriteTimeUtc.ToUniversalTime() -lt $BuildStartedAt.ToUniversalTime()) {
            throw "The ${ProductName} updater artifacts were not produced by the current build."
        }
    }

    return [pscustomobject]@{
        Archive = $archives[0]
        Signature = $signatures[0]
    }
}

function ConvertTo-GitHubReleaseAssetName {
    param([Parameter(Mandatory)][string]$Name)

    return $Name.Replace(' ', '.')
}

function New-TauriUpdaterManifest {
    param(
        [Parameter(Mandatory)][string]$Version,
        [Parameter(Mandatory)][AllowEmptyString()][string]$Notes,
        [Parameter(Mandatory)][datetime]$PublishedAt,
        [Parameter(Mandatory)][string]$ArchiveUrl,
        [Parameter(Mandatory)][string]$Signature
    )

    if ($Version -notmatch '^\d+\.\d+\.\d+$') {
        throw "Updater version must use stable X.Y.Z format; found '$Version'."
    }
    if ([string]::IsNullOrWhiteSpace($Signature)) {
        throw 'Updater signature must not be empty.'
    }

    $uri = $null
    if (-not [uri]::TryCreate($ArchiveUrl, [System.UriKind]::Absolute, [ref]$uri) -or $uri.Scheme -ne 'https') {
        throw 'Updater archive URL must be an absolute HTTPS URL.'
    }
    $expectedTagPath = "/releases/download/v${Version}/"
    if (-not $uri.AbsolutePath.Contains($expectedTagPath)) {
        throw "Updater archive URL must use release tag v${Version}."
    }

    $manifest = [ordered]@{
        version = $Version
        notes = $Notes
        pub_date = $PublishedAt.ToUniversalTime().ToString('o')
        platforms = [ordered]@{
            'windows-x86_64' = [ordered]@{
                signature = $Signature.Trim()
                url = $ArchiveUrl
            }
        }
    }
    return $manifest | ConvertTo-Json -Depth 5
}

function Set-ArtifactHashesInText {
    param(
        [Parameter(Mandatory)][string]$Text,
        [Parameter(Mandatory)][string]$InstallerHash,
        [Parameter(Mandatory)][string]$HookHash
    )

    foreach ($hash in @($InstallerHash, $HookHash)) {
        if ($hash -notmatch '^[A-Fa-f0-9]{64}$') {
            throw "Invalid SHA-256 value '$hash'."
        }
    }

    $installerPattern = '(?m)(^- NSIS installer(?: SHA-256)?: `)[A-Fa-f0-9]{64}(`\s*$)'
    $hookPattern = '(?m)(^- `hook\.dll`(?: SHA-256)?: `)[A-Fa-f0-9]{64}(`\s*$)'
    $installerMatches = [regex]::Matches($Text, $installerPattern)
    $hookMatches = [regex]::Matches($Text, $hookPattern)
    if ($installerMatches.Count -ne 1 -or $hookMatches.Count -ne 1) {
        throw "Expected exactly one NSIS installer hash and one hook.dll hash; found $($installerMatches.Count) and $($hookMatches.Count)."
    }

    $normalizedInstaller = $InstallerHash.ToUpperInvariant()
    $normalizedHook = $HookHash.ToUpperInvariant()
    $updated = [regex]::Replace($Text, $installerPattern, { param($match) $match.Groups[1].Value + $normalizedInstaller + $match.Groups[2].Value })
    return [regex]::Replace($updated, $hookPattern, { param($match) $match.Groups[1].Value + $normalizedHook + $match.Groups[2].Value })
}

function New-NsisUpdaterArchive {
    param(
        [Parameter(Mandatory)][System.IO.FileInfo]$Installer,
        [Parameter(Mandatory)][string]$DestinationPath
    )

    Add-Type -AssemblyName System.IO.Compression
    $destination = [IO.Path]::GetFullPath($DestinationPath)
    $parent = Split-Path -Parent $destination
    if (-not (Test-Path -LiteralPath $parent -PathType Container)) {
        throw "Updater archive directory is missing: $parent"
    }

    $fileStream = [IO.File]::Open($destination, [IO.FileMode]::Create, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try {
        $archive = [IO.Compression.ZipArchive]::new(
            $fileStream,
            [IO.Compression.ZipArchiveMode]::Create,
            $false
        )
        try {
            $entry = $archive.CreateEntry($Installer.Name, [IO.Compression.CompressionLevel]::NoCompression)
            $input = $Installer.OpenRead()
            $output = $entry.Open()
            try {
                $input.CopyTo($output)
            }
            finally {
                $output.Dispose()
                $input.Dispose()
            }
        }
        finally {
            $archive.Dispose()
        }
    }
    finally {
        $fileStream.Dispose()
    }

    return Get-Item -LiteralPath $destination
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

Export-ModuleMember -Function Get-NodeMajorVersion, Assert-SupportedNodeVersion, Assert-GameNotRunning, Select-ProductNsisInstaller, Assert-ReleaseVersionAgreement, Assert-UpdaterSigningEnvironment, Select-ProductNsisUpdaterArtifacts, ConvertTo-GitHubReleaseAssetName, New-TauriUpdaterManifest, Set-ArtifactHashesInText, New-NsisUpdaterArchive, Invoke-NativeCommand

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

Import-Module (Join-Path $PSScriptRoot '..\PackageHelpers.psm1') -Force

function Assert-Equal {
    param($Actual, $Expected, [string]$Message)
    if ($Actual -ne $Expected) {
        throw "$Message Expected '$Expected', got '$Actual'."
    }
}

function Assert-Throws {
    param([scriptblock]$Action, [string]$Message)
    try {
        & $Action
    }
    catch {
        return
    }
    throw "$Message Expected an exception."
}

Assert-Equal (Get-NodeMajorVersion -Version 'v20.11.1') 20 'Node major parsing failed.'
Assert-Throws { Assert-SupportedNodeVersion -Version 'v19.9.0' } 'Node 19 must fail.'
Assert-SupportedNodeVersion -Version 'v20.11.1'
$nodeWarnings = @()
Assert-SupportedNodeVersion -Version 'v24.16.0' -WarningVariable nodeWarnings
Assert-Equal $nodeWarnings.Count 1 'Node 24 must emit one warning.'

Assert-GameNotRunning -Processes @()
Assert-Throws { Assert-GameNotRunning -Processes @([pscustomobject]@{ Id = 1234 }) } 'A running game must fail.'

$buildStartedAt = [datetime]'2026-07-19T01:00:00Z'
$productInstaller = [pscustomobject]@{
    Name = 'Djeeta MOD_0.1.0_x64-setup.exe'
    LastWriteTimeUtc = [datetime]'2026-07-19T01:00:01Z'
}
$otherInstaller = [pscustomobject]@{
    Name = 'build_trait_caps_0.1.0_x64-setup.exe'
    LastWriteTimeUtc = [datetime]'2026-07-19T01:00:02Z'
}
$applicationExe = [pscustomobject]@{
    Name = 'Djeeta MOD.exe'
    LastWriteTimeUtc = [datetime]'2026-07-19T01:00:03Z'
}
$selectedInstaller = Select-ProductNsisInstaller -Artifacts @($productInstaller, $otherInstaller, $applicationExe) -ProductName 'Djeeta MOD' -Version '0.1.0' -BuildStartedAt $buildStartedAt
Assert-Equal $selectedInstaller.Name $productInstaller.Name 'Product installer selection failed.'
Assert-Throws {
    Select-ProductNsisInstaller -Artifacts @($otherInstaller, $applicationExe) -ProductName 'Djeeta MOD' -Version '0.1.0' -BuildStartedAt $buildStartedAt
} 'A non-product result must fail.'
Assert-Throws {
    Select-ProductNsisInstaller -Artifacts @($productInstaller, $productInstaller) -ProductName 'Djeeta MOD' -Version '0.1.0' -BuildStartedAt $buildStartedAt
} 'Multiple product installers must fail.'
Assert-Throws {
    Select-ProductNsisInstaller -Artifacts @(
        [pscustomobject]@{
            Name = 'Djeeta MOD_0.1.0_x64-setup.exe'
            LastWriteTimeUtc = [datetime]'2026-07-19T00:59:59Z'
        }
    ) -ProductName 'Djeeta MOD' -Version '0.1.0' -BuildStartedAt $buildStartedAt
} 'A stale installer must fail.'

Assert-Equal (Assert-ReleaseVersionAgreement -RequestedVersion '0.1.2' -PackageVersion '0.1.2' -CargoVersion '0.1.2' -TauriVersion '0.1.2') '0.1.2' 'Matching release versions failed.'
Assert-Throws { Assert-ReleaseVersionAgreement -RequestedVersion '0.1.2' -PackageVersion '0.1.2' -CargoVersion '0.1.1' -TauriVersion '0.1.2' } 'Cargo mismatch must fail.'
Assert-Throws { Assert-ReleaseVersionAgreement -RequestedVersion 'v0.1.2' -PackageVersion '0.1.2' -CargoVersion '0.1.2' -TauriVersion '0.1.2' } 'Prefixed version must fail.'
Assert-Throws { Assert-ReleaseVersionAgreement -RequestedVersion '0.1.2-beta.1' -PackageVersion '0.1.2-beta.1' -CargoVersion '0.1.2-beta.1' -TauriVersion '0.1.2-beta.1' } 'Prerelease must fail.'

Assert-UpdaterSigningEnvironment -Values @{
    TAURI_PRIVATE_KEY = 'private-key'
    TAURI_KEY_PASSWORD = 'password'
}
Assert-Throws {
    Assert-UpdaterSigningEnvironment -Values @{
        TAURI_PRIVATE_KEY = ' '
        TAURI_KEY_PASSWORD = 'password'
    }
} 'An empty private key must fail.'
Assert-Throws {
    Assert-UpdaterSigningEnvironment -Values @{
        TAURI_PRIVATE_KEY = 'private-key'
        TAURI_KEY_PASSWORD = ''
    }
} 'An empty key password must fail.'

$updaterArchive = [pscustomobject]@{
    Name = 'Djeeta MOD_0.1.2_x64-setup.nsis.zip'
    LastWriteTimeUtc = [datetime]'2026-07-19T01:00:01Z'
}
$updaterSignature = [pscustomobject]@{
    Name = 'Djeeta MOD_0.1.2_x64-setup.nsis.zip.sig'
    LastWriteTimeUtc = [datetime]'2026-07-19T01:00:02Z'
}
$selectedUpdater = Select-ProductNsisUpdaterArtifacts -Artifacts @($updaterArchive, $updaterSignature) -ProductName 'Djeeta MOD' -Version '0.1.2' -BuildStartedAt $buildStartedAt
Assert-Equal $selectedUpdater.Archive.Name $updaterArchive.Name 'Updater archive selection failed.'
Assert-Equal $selectedUpdater.Signature.Name $updaterSignature.Name 'Updater signature selection failed.'

Assert-Throws {
    Select-ProductNsisUpdaterArtifacts -Artifacts @($updaterArchive) -ProductName 'Djeeta MOD' -Version '0.1.2' -BuildStartedAt $buildStartedAt
} 'A missing updater signature must fail.'
Assert-Throws {
    Select-ProductNsisUpdaterArtifacts -Artifacts @($updaterArchive, $updaterSignature, $updaterSignature) -ProductName 'Djeeta MOD' -Version '0.1.2' -BuildStartedAt $buildStartedAt
} 'Duplicate updater signatures must fail.'
Assert-Throws {
    Select-ProductNsisUpdaterArtifacts -Artifacts @(
        $updaterArchive,
        [pscustomobject]@{
            Name = 'Djeeta MOD_0.1.2_x64-setup.nsis.zip.sig'
            LastWriteTimeUtc = [datetime]'2026-07-19T00:59:59Z'
        }
    ) -ProductName 'Djeeta MOD' -Version '0.1.2' -BuildStartedAt $buildStartedAt
} 'A stale updater signature must fail.'
Assert-Throws {
    Select-ProductNsisUpdaterArtifacts -Artifacts @(
        [pscustomobject]@{ Name = 'Other_0.1.2_x64-setup.nsis.zip'; LastWriteTimeUtc = [datetime]'2026-07-19T01:00:01Z' },
        [pscustomobject]@{ Name = 'Other_0.1.2_x64-setup.nsis.zip.sig'; LastWriteTimeUtc = [datetime]'2026-07-19T01:00:02Z' }
    ) -ProductName 'Djeeta MOD' -Version '0.1.2' -BuildStartedAt $buildStartedAt
} 'Wrong-product updater artifacts must fail.'
Assert-Throws {
    Select-ProductNsisUpdaterArtifacts -Artifacts @($updaterArchive, $updaterSignature) -ProductName 'Djeeta MOD' -Version '0.1.1' -BuildStartedAt $buildStartedAt
} 'Wrong-version updater artifacts must fail.'

$archiveUrl = 'https://github.com/azyu/gbfr-djeetamod/releases/download/v0.1.2/Djeeta%20MOD_0.1.2_x64-setup.nsis.zip'
$manifest = New-TauriUpdaterManifest -Version '0.1.2' -Notes 'Release notes' -PublishedAt ([datetime]'2026-07-22T00:00:00Z') -ArchiveUrl $archiveUrl -Signature 'signed-content'
$parsed = $manifest | ConvertFrom-Json
Assert-Equal $parsed.version '0.1.2' 'Manifest version failed.'
Assert-Equal $parsed.platforms.'windows-x86_64'.signature 'signed-content' 'Manifest signature failed.'
Assert-Equal $parsed.platforms.'windows-x86_64'.url $archiveUrl 'Manifest URL failed.'
Assert-Throws {
    New-TauriUpdaterManifest -Version '0.1.2' -Notes '' -PublishedAt ([datetime]'2026-07-22T00:00:00Z') -ArchiveUrl $archiveUrl -Signature ' '
} 'An empty updater signature must fail.'
Assert-Throws {
    New-TauriUpdaterManifest -Version '0.1.2' -Notes '' -PublishedAt ([datetime]'2026-07-22T00:00:00Z') -ArchiveUrl 'http://example.com/releases/download/v0.1.2/update.zip' -Signature 'signed-content'
} 'A non-HTTPS updater URL must fail.'
Assert-Throws {
    New-TauriUpdaterManifest -Version '0.1.2' -Notes '' -PublishedAt ([datetime]'2026-07-22T00:00:00Z') -ArchiveUrl 'https://github.com/azyu/gbfr-djeetamod/releases/download/v0.1.1/update.zip' -Signature 'signed-content'
} 'A mismatched updater tag must fail.'

$oldInstaller = 'A' * 64
$oldHook = 'B' * 64
$newInstaller = 'C' * 64
$newHook = 'D' * 64
$readme = @'
- NSIS installer: `AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA`
- `hook.dll`: `BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB`
'@
$smoke = @'
- NSIS installer SHA-256: `AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA`
- `hook.dll` SHA-256: `BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB`
'@

$updatedReadme = Set-ArtifactHashesInText -Text $readme -InstallerHash $newInstaller -HookHash $newHook
$updatedSmoke = Set-ArtifactHashesInText -Text $smoke -InstallerHash $newInstaller -HookHash $newHook
Assert-Equal ([regex]::Matches($updatedReadme, $newInstaller).Count) 1 'README installer replacement failed.'
Assert-Equal ([regex]::Matches($updatedReadme, $newHook).Count) 1 'README hook replacement failed.'
Assert-Equal ([regex]::Matches($updatedSmoke, $newInstaller).Count) 1 'Smoke installer replacement failed.'
Assert-Equal ([regex]::Matches($updatedSmoke, $newHook).Count) 1 'Smoke hook replacement failed.'

Assert-Throws {
    Set-ArtifactHashesInText -Text ($readme + "`r`n" + $readme) -InstallerHash $newInstaller -HookHash $newHook
} 'Duplicate labels must fail.'
Assert-Throws {
    Set-ArtifactHashesInText -Text "- NSIS installer: ``$oldInstaller``" -InstallerHash $newInstaller -HookHash $newHook
} 'Missing hook label must fail.'

Write-Output 'Package helper tests passed.'

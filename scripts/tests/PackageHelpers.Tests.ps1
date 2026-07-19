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

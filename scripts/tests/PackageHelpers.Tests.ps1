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
$productMsi = [pscustomobject]@{
    Name = 'Djeeta MOD_0.1.0_x64_en-US.msi'
    LastWriteTimeUtc = [datetime]'2026-07-19T01:00:01Z'
}
$helperMsi = [pscustomobject]@{
    Name = 'build_trait_caps_0.1.0_x64_en-US.msi'
    LastWriteTimeUtc = [datetime]'2026-07-19T01:00:02Z'
}
$selectedMsi = Select-ProductMsi -Artifacts @($productMsi, $helperMsi) -ProductName 'Djeeta MOD' -Version '0.1.0' -BuildStartedAt $buildStartedAt
Assert-Equal $selectedMsi.Name $productMsi.Name 'Product MSI selection failed.'
Assert-Throws {
    Select-ProductMsi -Artifacts @($helperMsi) -ProductName 'Djeeta MOD' -Version '0.1.0' -BuildStartedAt $buildStartedAt
} 'A helper-only MSI result must fail.'
Assert-Throws {
    Select-ProductMsi -Artifacts @(
        $productMsi,
        [pscustomobject]@{
            Name = 'Djeeta MOD_0.1.0_x64_ko-KR.msi'
            LastWriteTimeUtc = [datetime]'2026-07-19T01:00:03Z'
        }
    ) -ProductName 'Djeeta MOD' -Version '0.1.0' -BuildStartedAt $buildStartedAt
} 'Multiple product MSIs must fail.'
Assert-Throws {
    Select-ProductMsi -Artifacts @(
        [pscustomobject]@{
            Name = 'Djeeta MOD_0.1.0_x64_en-US.msi'
            LastWriteTimeUtc = [datetime]'2026-07-19T00:59:59Z'
        }
    ) -ProductName 'Djeeta MOD' -Version '0.1.0' -BuildStartedAt $buildStartedAt
} 'A stale product MSI must fail.'

$oldMsi = 'A' * 64
$oldHook = 'B' * 64
$newMsi = 'C' * 64
$newHook = 'D' * 64
$readme = @'
- MSI: `AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA`
- `hook.dll`: `BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB`
'@
$smoke = @'
- MSI SHA-256: `AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA`
- `hook.dll` SHA-256: `BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB`
'@

$updatedReadme = Set-ArtifactHashesInText -Text $readme -MsiHash $newMsi -HookHash $newHook
$updatedSmoke = Set-ArtifactHashesInText -Text $smoke -MsiHash $newMsi -HookHash $newHook
Assert-Equal ([regex]::Matches($updatedReadme, $newMsi).Count) 1 'README MSI replacement failed.'
Assert-Equal ([regex]::Matches($updatedReadme, $newHook).Count) 1 'README hook replacement failed.'
Assert-Equal ([regex]::Matches($updatedSmoke, $newMsi).Count) 1 'Smoke MSI replacement failed.'
Assert-Equal ([regex]::Matches($updatedSmoke, $newHook).Count) 1 'Smoke hook replacement failed.'

Assert-Throws {
    Set-ArtifactHashesInText -Text ($readme + "`r`n" + $readme) -MsiHash $newMsi -HookHash $newHook
} 'Duplicate labels must fail.'
Assert-Throws {
    Set-ArtifactHashesInText -Text "- MSI: ``$oldMsi``" -MsiHash $newMsi -HookHash $newHook
} 'Missing hook label must fail.'

Write-Output 'Package helper tests passed.'

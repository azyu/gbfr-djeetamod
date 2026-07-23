param(
    [string]$RequestedVersion
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

Import-Module (Join-Path $PSScriptRoot 'PackageHelpers.psm1') -Force

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$utf8WithoutBom = New-Object System.Text.UTF8Encoding($false)

Push-Location $repositoryRoot
try {
    if ($env:OS -ne 'Windows_NT') {
        throw 'NSIS signing is supported only on Windows.'
    }

    Assert-UpdaterSigningEnvironment -Values @{
        TAURI_PRIVATE_KEY = $env:TAURI_PRIVATE_KEY
        TAURI_KEY_PASSWORD = $env:TAURI_KEY_PASSWORD
    }

    $preparationSummaryPath = Join-Path $repositoryRoot 'target\release\package-preparation.json'
    if (-not (Test-Path -LiteralPath $preparationSummaryPath -PathType Leaf)) {
        throw "Package preparation summary is missing: $preparationSummaryPath"
    }
    $preparation = Get-Content -Raw -LiteralPath $preparationSummaryPath | ConvertFrom-Json

    $effectiveRequestedVersion = $RequestedVersion
    if ([string]::IsNullOrWhiteSpace($effectiveRequestedVersion)) {
        $effectiveRequestedVersion = [string]$preparation.Version
    }
    $productVersion = Assert-ReleaseVersionAgreement `
        -RequestedVersion $effectiveRequestedVersion `
        -PackageVersion ([string]$preparation.Version) `
        -CargoVersion ([string]$preparation.Version) `
        -TauriVersion ([string]$preparation.Version)
    $productName = [string]$preparation.ProductName
    $buildStartedAt = [datetime]::Parse([string]$preparation.BuildStartedAt).ToUniversalTime()

    $requiredArtifacts = [ordered]@{
        Installer = [string]$preparation.InstallerPath
        Hook = [string]$preparation.HookPath
        BundledHook = [string]$preparation.BundledHookPath
        UpdaterArchive = [string]$preparation.UpdaterArchivePath
    }
    foreach ($artifact in $requiredArtifacts.GetEnumerator()) {
        if (-not (Test-Path -LiteralPath $artifact.Value -PathType Leaf)) {
            throw "$($artifact.Key) artifact is missing: $($artifact.Value)"
        }
    }

    $installer = Get-Item -LiteralPath $requiredArtifacts.Installer
    $expectedInstallerName = "${productName}_${productVersion}_x64-setup.exe"
    if ($installer.Name -cne $expectedInstallerName) {
        throw "Prepared installer name does not match ${productName} ${productVersion}: $($installer.Name)"
    }
    $updaterArchive = Get-Item -LiteralPath $requiredArtifacts.UpdaterArchive
    $expectedArchiveName = "${productName}_${productVersion}_x64-setup.nsis.zip"
    if ($updaterArchive.Name -cne $expectedArchiveName) {
        throw "Prepared updater archive name does not match ${productName} ${productVersion}: $($updaterArchive.Name)"
    }

    $expectedHashes = [ordered]@{
        Installer = [string]$preparation.InstallerSHA256
        Hook = [string]$preparation.HookSHA256
        BundledHook = [string]$preparation.HookSHA256
        UpdaterArchive = [string]$preparation.UpdaterArchiveSHA256
    }
    foreach ($artifact in $requiredArtifacts.GetEnumerator()) {
        $actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $artifact.Value).Hash
        if ($actualHash -cne $expectedHashes[$artifact.Key]) {
            throw "$($artifact.Key) hash differs from the package preparation summary."
        }
    }

    $npmPath = (Get-Command npm.cmd -ErrorAction Stop).Source
    try {
        Invoke-NativeCommand -FilePath $npmPath -Arguments @(
            'run',
            'tauri',
            '--',
            'signer',
            'sign',
            $updaterArchive.FullName
        )
    }
    finally {
        Remove-Item Env:TAURI_PRIVATE_KEY -ErrorAction SilentlyContinue
        Remove-Item Env:TAURI_KEY_PASSWORD -ErrorAction SilentlyContinue
    }

    $updaterArtifacts = @(Get-ChildItem -LiteralPath $updaterArchive.DirectoryName -Filter '*.nsis.zip*')
    $updater = Select-ProductNsisUpdaterArtifacts `
        -Artifacts $updaterArtifacts `
        -ProductName $productName `
        -Version $productVersion `
        -BuildStartedAt $buildStartedAt

    $updaterSignature = [IO.File]::ReadAllText($updater.Signature.FullName)
    $releaseArchiveName = ConvertTo-GitHubReleaseAssetName -Name $updater.Archive.Name
    $encodedArchiveName = [uri]::EscapeDataString($releaseArchiveName)
    $archiveUrl = "https://github.com/azyu/gbfr-djeetamod/releases/download/v${productVersion}/${encodedArchiveName}"
    $latestJson = New-TauriUpdaterManifest `
        -Version $productVersion `
        -Notes ([string]$preparation.ReleaseNotes) `
        -PublishedAt ([datetime]::UtcNow) `
        -ArchiveUrl $archiveUrl `
        -Signature $updaterSignature
    $latestJsonPath = Join-Path $repositoryRoot 'target\release\latest.json'
    [IO.File]::WriteAllText($latestJsonPath, $latestJson, $utf8WithoutBom)

    $installerHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $installer.FullName).Hash
    $releaseHookHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $requiredArtifacts.Hook).Hash
    $updaterArchiveHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $updater.Archive.FullName).Hash

    $updatedDocuments = @{}
    foreach ($documentPath in @('README.md', 'docs\testing\game-2.0.2-smoke-test.md')) {
        $absolutePath = Join-Path $repositoryRoot $documentPath
        $currentText = [IO.File]::ReadAllText($absolutePath)
        $updatedText = Set-ArtifactHashesInText `
            -Text $currentText `
            -InstallerHash $installerHash `
            -HookHash $releaseHookHash
        if ($updatedText -ne $currentText) {
            $updatedDocuments[$absolutePath] = $updatedText
        }
    }
    foreach ($document in $updatedDocuments.GetEnumerator()) {
        [IO.File]::WriteAllText($document.Key, $document.Value, $utf8WithoutBom)
    }

    $packageSummaryPath = Join-Path $repositoryRoot 'target\release\package-summary.json'
    $packageSummary = [ordered]@{
        Version = $productVersion
        InstallerPath = $installer.FullName
        InstallerSHA256 = $installerHash
        HookPath = $requiredArtifacts.Hook
        HookSHA256 = $releaseHookHash
        UpdaterArchivePath = $updater.Archive.FullName
        UpdaterArchiveSHA256 = $updaterArchiveHash
        UpdaterSignaturePath = $updater.Signature.FullName
        LatestJsonPath = $latestJsonPath
    } | ConvertTo-Json -Depth 3
    [IO.File]::WriteAllText($packageSummaryPath, $packageSummary, $utf8WithoutBom)

    $gitPath = (Get-Command git.exe -ErrorAction Stop).Source
    Invoke-NativeCommand -FilePath $gitPath -Arguments @('diff', '--check')

    [pscustomobject]@{
        InstallerPath = $installer.FullName
        InstallerSHA256 = $installerHash
        HookSHA256 = $releaseHookHash
        UpdaterArchivePath = $updater.Archive.FullName
        UpdaterArchiveSHA256 = $updaterArchiveHash
        UpdaterSignaturePath = $updater.Signature.FullName
        LatestJsonPath = $latestJsonPath
        PackageSummaryPath = $packageSummaryPath
        HookHashesEqual = $true
        UpdatedDocuments = @($updatedDocuments.Keys)
    } | Format-List
}
finally {
    Pop-Location
}

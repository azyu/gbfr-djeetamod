param(
    [string]$RequestedVersion,
    [string]$ReleaseNotesPath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

Import-Module (Join-Path $PSScriptRoot 'PackageHelpers.psm1') -Force

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$utf8WithoutBom = New-Object System.Text.UTF8Encoding($false)

Push-Location $repositoryRoot
try {
    if ($env:OS -ne 'Windows_NT') {
        throw 'NSIS packaging is supported only on Windows.'
    }

    $requiredPaths = @(
        'package.json',
        'src-tauri\Cargo.toml',
        'src-tauri\tauri.conf.json',
        'README.md',
        'docs\testing\game-2.0.2-smoke-test.md'
    )
    foreach ($path in $requiredPaths) {
        if (-not (Test-Path -LiteralPath $path)) {
            throw "Required packaging input is missing: $path"
        }
    }

    $packageJson = Get-Content -Raw -LiteralPath 'package.json' | ConvertFrom-Json
    $tauriConfig = Get-Content -Raw -LiteralPath 'src-tauri\tauri.conf.json' | ConvertFrom-Json
    $cargoManifest = Get-Content -Raw -LiteralPath 'src-tauri\Cargo.toml'
    $cargoPackageSection = [regex]::Match($cargoManifest, '(?ms)^\[package\]\s*(.*?)(?=^\[|\z)')
    $cargoVersionMatch = [regex]::Match($cargoPackageSection.Groups[1].Value, '(?m)^version\s*=\s*"([^"]+)"\s*$')
    if (-not $cargoPackageSection.Success -or -not $cargoVersionMatch.Success) {
        throw 'Could not read the package version from src-tauri\Cargo.toml.'
    }

    $effectiveRequestedVersion = $RequestedVersion
    if ([string]::IsNullOrWhiteSpace($effectiveRequestedVersion)) {
        $effectiveRequestedVersion = [string]$packageJson.version
    }
    $productVersion = Assert-ReleaseVersionAgreement `
        -RequestedVersion $effectiveRequestedVersion `
        -PackageVersion ([string]$packageJson.version) `
        -CargoVersion $cargoVersionMatch.Groups[1].Value `
        -TauriVersion ([string]$tauriConfig.package.version)
    Assert-UpdaterSigningEnvironment -Values @{
        TAURI_PRIVATE_KEY = $env:TAURI_PRIVATE_KEY
        TAURI_KEY_PASSWORD = $env:TAURI_KEY_PASSWORD
    }

    $releaseNotes = ''
    if (-not [string]::IsNullOrWhiteSpace($ReleaseNotesPath)) {
        if (-not (Test-Path -LiteralPath $ReleaseNotesPath -PathType Leaf)) {
            throw "Release notes file is missing: $ReleaseNotesPath"
        }
        $releaseNotes = [System.IO.File]::ReadAllText((Resolve-Path -LiteralPath $ReleaseNotesPath).Path)
    }

    $gameProcesses = @(Get-Process -Name 'granblue_fantasy_relink' -ErrorAction SilentlyContinue)
    Assert-GameNotRunning -Processes $gameProcesses

    $nodePath = (Get-Command node.exe -ErrorAction Stop).Source
    $npmPath = (Get-Command npm.cmd -ErrorAction Stop).Source
    $gitPath = (Get-Command git.exe -ErrorAction Stop).Source
    $powershellPath = (Get-Command powershell.exe -ErrorAction Stop).Source
    $cargoCommand = Get-Command cargo.exe -ErrorAction SilentlyContinue
    if ($null -ne $cargoCommand) {
        $cargoPath = $cargoCommand.Source
    }
    else {
        $cargoPath = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
        if (-not (Test-Path -LiteralPath $cargoPath)) {
            throw "Cargo was not found on PATH or at $cargoPath."
        }
    }
    $env:Path = "$(Split-Path -Parent $cargoPath);$env:Path"

    $nodeVersion = (@(Invoke-NativeCommand -FilePath $nodePath -Arguments @('--version')) | Select-Object -Last 1).Trim()
    Assert-SupportedNodeVersion -Version $nodeVersion

    Invoke-NativeCommand -FilePath $powershellPath -Arguments @(
        '-NoProfile',
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        (Join-Path $repositoryRoot 'scripts\tests\PackageHelpers.Tests.ps1')
    )
    Invoke-NativeCommand -FilePath $npmPath -Arguments @('ci')
    Invoke-NativeCommand -FilePath $npmPath -Arguments @('run', 'format-check')
    Invoke-NativeCommand -FilePath $npmPath -Arguments @('run', 'lint')
    Invoke-NativeCommand -FilePath $npmPath -Arguments @('run', 'tsc')
    Invoke-NativeCommand -FilePath $npmPath -Arguments @('test', '--', '--run')
    Invoke-NativeCommand -FilePath $npmPath -Arguments @('run', 'build')
    Invoke-NativeCommand -FilePath $cargoPath -Arguments @('build', '--release', '--locked', '--package', 'hook')
    Invoke-NativeCommand -FilePath $cargoPath -Arguments @('test', '--workspace', '--all-targets', '--locked')

    $releaseHookPath = Join-Path $repositoryRoot 'target\release\hook.dll'
    $bundledHookPath = Join-Path $repositoryRoot 'src-tauri\hook.dll'
    if (-not (Test-Path -LiteralPath $releaseHookPath)) {
        throw "Release hook was not produced: $releaseHookPath"
    }
    Copy-Item -LiteralPath $releaseHookPath -Destination $bundledHookPath -Force

    $productName = $tauriConfig.package.productName
    $buildStartedAt = [datetime]::UtcNow
    Invoke-NativeCommand -FilePath $npmPath -Arguments @(
        'run',
        'tauri',
        '--',
        'build',
        '--bundles',
        'nsis',
        'updater',
        '--',
        '--bin',
        'gbfr-logs'
    )

    $installerArtifacts = @(Get-ChildItem -LiteralPath 'target\release\bundle\nsis' -Filter '*.exe')
    $installer = Select-ProductNsisInstaller -Artifacts $installerArtifacts -ProductName $productName -Version $productVersion -BuildStartedAt $buildStartedAt
    $updaterArtifacts = @(Get-ChildItem -LiteralPath 'target\release\bundle\nsis' -Filter '*.nsis.zip*')
    $updater = Select-ProductNsisUpdaterArtifacts -Artifacts $updaterArtifacts -ProductName $productName -Version $productVersion -BuildStartedAt $buildStartedAt

    $updaterSignature = [System.IO.File]::ReadAllText($updater.Signature.FullName)
    $encodedArchiveName = [uri]::EscapeDataString($updater.Archive.Name)
    $archiveUrl = "https://github.com/azyu/gbfr-djeetamod/releases/download/v${productVersion}/${encodedArchiveName}"
    $latestJson = New-TauriUpdaterManifest -Version $productVersion -Notes $releaseNotes -PublishedAt ([datetime]::UtcNow) -ArchiveUrl $archiveUrl -Signature $updaterSignature
    $latestJsonPath = Join-Path $repositoryRoot 'target\release\latest.json'
    [System.IO.File]::WriteAllText($latestJsonPath, $latestJson, $utf8WithoutBom)

    $releaseHookHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $releaseHookPath).Hash
    $bundledHookHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $bundledHookPath).Hash
    if ($releaseHookHash -ne $bundledHookHash) {
        throw 'Release and bundled hook.dll hashes differ.'
    }
    $installerHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $installer.FullName).Hash
    $updaterArchiveHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $updater.Archive.FullName).Hash

    $packageSummaryPath = Join-Path $repositoryRoot 'target\release\package-summary.json'
    $packageSummary = [ordered]@{
        Version = $productVersion
        InstallerPath = $installer.FullName
        InstallerSHA256 = $installerHash
        HookPath = $releaseHookPath
        HookSHA256 = $releaseHookHash
        UpdaterArchivePath = $updater.Archive.FullName
        UpdaterArchiveSHA256 = $updaterArchiveHash
        UpdaterSignaturePath = $updater.Signature.FullName
        LatestJsonPath = $latestJsonPath
    } | ConvertTo-Json -Depth 3
    [System.IO.File]::WriteAllText($packageSummaryPath, $packageSummary, $utf8WithoutBom)

    $updatedDocuments = @{}
    foreach ($documentPath in @('README.md', 'docs\testing\game-2.0.2-smoke-test.md')) {
        $absolutePath = Join-Path $repositoryRoot $documentPath
        $currentText = [System.IO.File]::ReadAllText($absolutePath)
        $updatedText = Set-ArtifactHashesInText -Text $currentText -InstallerHash $installerHash -HookHash $releaseHookHash
        if ($updatedText -ne $currentText) {
            $updatedDocuments[$absolutePath] = $updatedText
        }
    }
    foreach ($document in $updatedDocuments.GetEnumerator()) {
        [System.IO.File]::WriteAllText($document.Key, $document.Value, $utf8WithoutBom)
    }

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

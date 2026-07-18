$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

Import-Module (Join-Path $PSScriptRoot 'PackageHelpers.psm1') -Force

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$utf8WithoutBom = New-Object System.Text.UTF8Encoding($false)

Push-Location $repositoryRoot
try {
    if ($env:OS -ne 'Windows_NT') {
        throw 'MSI packaging is supported only on Windows.'
    }

    $requiredPaths = @(
        'package.json',
        'src-tauri\tauri.conf.json',
        'README.md',
        'docs\testing\game-2.0.2-smoke-test.md'
    )
    foreach ($path in $requiredPaths) {
        if (-not (Test-Path -LiteralPath $path)) {
            throw "Required packaging input is missing: $path"
        }
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

    Invoke-NativeCommand -FilePath $npmPath -Arguments @('run', 'tauri', 'build', '--', '--bundles', 'msi')

    $msi = Get-ChildItem -LiteralPath 'target\release\bundle\msi' -Filter '*.msi' |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if ($null -eq $msi) {
        throw 'Tauri did not produce an MSI.'
    }

    $releaseHookHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $releaseHookPath).Hash
    $bundledHookHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $bundledHookPath).Hash
    if ($releaseHookHash -ne $bundledHookHash) {
        throw 'Release and bundled hook.dll hashes differ.'
    }
    $msiHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $msi.FullName).Hash

    foreach ($documentPath in @('README.md', 'docs\testing\game-2.0.2-smoke-test.md')) {
        $absolutePath = Join-Path $repositoryRoot $documentPath
        $currentText = [System.IO.File]::ReadAllText($absolutePath)
        $updatedText = Set-ArtifactHashesInText -Text $currentText -MsiHash $msiHash -HookHash $releaseHookHash
        if ($updatedText -ne $currentText) {
            [System.IO.File]::WriteAllText($absolutePath, $updatedText, $utf8WithoutBom)
        }
    }

    Invoke-NativeCommand -FilePath $gitPath -Arguments @('diff', '--check')

    [pscustomobject]@{
        MsiPath = $msi.FullName
        MsiSHA256 = $msiHash
        HookSHA256 = $releaseHookHash
        HookHashesEqual = $true
        UpdatedDocuments = @('README.md', 'docs/testing/game-2.0.2-smoke-test.md')
    } | Format-List
}
finally {
    Pop-Location
}

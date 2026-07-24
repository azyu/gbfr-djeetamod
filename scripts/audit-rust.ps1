$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$auditVersion = '0.22.2'
$archiveName = 'cargo-audit-x86_64-pc-windows-msvc-v0.22.2.zip'
$archiveUrl = "https://github.com/rustsec/rustsec/releases/download/cargo-audit/v${auditVersion}/${archiveName}"
$expectedArchiveHash = '0a7316540862c13d954f648917ceacca593747baed6eec180fafa590be2710ab'
$temporaryBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$temporaryRoot = Join-Path $temporaryBase ('djeeta-rust-audit-' + [guid]::NewGuid().ToString('N'))
$auditExitCode = 0

New-Item -ItemType Directory -Path $temporaryRoot | Out-Null
try {
    $archivePath = Join-Path $temporaryRoot $archiveName
    Invoke-WebRequest -UseBasicParsing -Uri $archiveUrl -OutFile $archivePath

    $actualArchiveHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $archivePath).Hash.ToLowerInvariant()
    if ($actualArchiveHash -cne $expectedArchiveHash) {
        throw "cargo-audit archive hash mismatch. Expected $expectedArchiveHash, found $actualArchiveHash."
    }

    $extractPath = Join-Path $temporaryRoot 'cargo-audit'
    Expand-Archive -LiteralPath $archivePath -DestinationPath $extractPath
    $auditExecutables = @(Get-ChildItem -LiteralPath $extractPath -Filter 'cargo-audit.exe' -Recurse)
    if ($auditExecutables.Count -ne 1) {
        throw "Expected exactly one cargo-audit.exe, found $($auditExecutables.Count)."
    }

    Push-Location $repositoryRoot
    try {
        & $auditExecutables[0].FullName audit --file 'Cargo.lock'
        if ($LASTEXITCODE -ne 0) {
            $auditExitCode = $LASTEXITCODE
        }

        & $auditExecutables[0].FullName audit --no-fetch --file 'protocol\Cargo.lock'
        if ($LASTEXITCODE -ne 0 -and $auditExitCode -eq 0) {
            $auditExitCode = $LASTEXITCODE
        }
    }
    finally {
        Pop-Location
    }
}
finally {
    $resolvedTemporaryRoot = [IO.Path]::GetFullPath($temporaryRoot)
    if (
        -not $resolvedTemporaryRoot.StartsWith($temporaryBase, [StringComparison]::OrdinalIgnoreCase) -or
        -not ([IO.Path]::GetFileName($resolvedTemporaryRoot)).StartsWith('djeeta-rust-audit-', [StringComparison]::Ordinal)
    ) {
        throw "Refusing to remove unexpected audit directory: $resolvedTemporaryRoot"
    }
    Remove-Item -LiteralPath $resolvedTemporaryRoot -Recurse -Force -ErrorAction SilentlyContinue
}

if ($auditExitCode -ne 0) {
    exit $auditExitCode
}

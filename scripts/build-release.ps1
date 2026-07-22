$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$keyPath = Join-Path ([Environment]::GetFolderPath('UserProfile')) '.djeeta-mod\updater.key'
if (-not (Test-Path -LiteralPath $keyPath -PathType Leaf)) {
    throw "Updater private key is missing: $keyPath"
}

$npmPath = (Get-Command npm.cmd -ErrorAction Stop).Source
$securePassword = Read-Host 'Updater key password' -AsSecureString
$passwordPointer = [IntPtr]::Zero
$packageExitCode = 1

Push-Location $repositoryRoot
try {
    $env:TAURI_PRIVATE_KEY = [IO.File]::ReadAllText($keyPath)
    $passwordPointer = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($securePassword)
    $env:TAURI_KEY_PASSWORD = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($passwordPointer)

    & $npmPath run package:nsis
    $packageExitCode = $LASTEXITCODE
}
finally {
    if ($passwordPointer -ne [IntPtr]::Zero) {
        [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($passwordPointer)
    }
    Remove-Item Env:TAURI_PRIVATE_KEY -ErrorAction SilentlyContinue
    Remove-Item Env:TAURI_KEY_PASSWORD -ErrorAction SilentlyContinue
    Pop-Location
}

exit $packageExitCode

Set-StrictMode -Version Latest

if ($null -eq ('DjeetaMod.StoredZipWriter' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.IO;
using System.Text;

namespace DjeetaMod
{
    public static class StoredZipWriter
    {
        private static readonly uint[] CrcTable = CreateCrcTable();

        public static void Create(string sourcePath, string destinationPath)
        {
            var source = new FileInfo(sourcePath);
            if (source.Length > UInt32.MaxValue)
            {
                throw new InvalidOperationException("Updater installer is too large for a non-Zip64 archive.");
            }

            byte[] name = Encoding.UTF8.GetBytes(source.Name);
            if (name.Length > UInt16.MaxValue)
            {
                throw new InvalidOperationException("Updater installer filename is too long.");
            }

            uint size = (uint)source.Length;
            uint crc = ComputeCrc32(source.FullName);
            DateTime timestamp = source.LastWriteTime;
            ushort dosTime = GetDosTime(timestamp);
            ushort dosDate = GetDosDate(timestamp);

            using (var output = new FileStream(destinationPath, FileMode.Create, FileAccess.Write, FileShare.None))
            using (var writer = new BinaryWriter(output, Encoding.UTF8, true))
            {
                writer.Write(0x04034B50u);
                writer.Write((ushort)20);
                writer.Write((ushort)0x0800);
                writer.Write((ushort)0);
                writer.Write(dosTime);
                writer.Write(dosDate);
                writer.Write(crc);
                writer.Write(size);
                writer.Write(size);
                writer.Write((ushort)name.Length);
                writer.Write((ushort)0);
                writer.Write(name);

                using (var input = source.OpenRead())
                {
                    input.CopyTo(output);
                }

                uint centralOffset = checked((uint)output.Position);
                writer.Write(0x02014B50u);
                writer.Write((ushort)20);
                writer.Write((ushort)20);
                writer.Write((ushort)0x0800);
                writer.Write((ushort)0);
                writer.Write(dosTime);
                writer.Write(dosDate);
                writer.Write(crc);
                writer.Write(size);
                writer.Write(size);
                writer.Write((ushort)name.Length);
                writer.Write((ushort)0);
                writer.Write((ushort)0);
                writer.Write((ushort)0);
                writer.Write((ushort)0);
                writer.Write(0u);
                writer.Write(0u);
                writer.Write(name);

                uint centralSize = checked((uint)output.Position - centralOffset);
                writer.Write(0x06054B50u);
                writer.Write((ushort)0);
                writer.Write((ushort)0);
                writer.Write((ushort)1);
                writer.Write((ushort)1);
                writer.Write(centralSize);
                writer.Write(centralOffset);
                writer.Write((ushort)0);
            }
        }

        private static uint ComputeCrc32(string path)
        {
            uint crc = UInt32.MaxValue;
            byte[] buffer = new byte[64 * 1024];
            using (var input = File.OpenRead(path))
            {
                int read;
                while ((read = input.Read(buffer, 0, buffer.Length)) > 0)
                {
                    for (int index = 0; index < read; index++)
                    {
                        crc = CrcTable[(crc ^ buffer[index]) & 0xFF] ^ (crc >> 8);
                    }
                }
            }
            return crc ^ UInt32.MaxValue;
        }

        private static uint[] CreateCrcTable()
        {
            var table = new uint[256];
            for (uint value = 0; value < table.Length; value++)
            {
                uint crc = value;
                for (int bit = 0; bit < 8; bit++)
                {
                    crc = (crc & 1) == 1 ? 0xEDB88320u ^ (crc >> 1) : crc >> 1;
                }
                table[value] = crc;
            }
            return table;
        }

        private static DateTime ClampZipTimestamp(DateTime value)
        {
            if (value.Year < 1980)
            {
                return new DateTime(1980, 1, 1, 0, 0, 0);
            }
            if (value.Year > 2107)
            {
                return new DateTime(2107, 12, 31, 23, 59, 58);
            }
            return value;
        }

        private static ushort GetDosTime(DateTime value)
        {
            value = ClampZipTimestamp(value);
            return (ushort)((value.Hour << 11) | (value.Minute << 5) | (value.Second / 2));
        }

        private static ushort GetDosDate(DateTime value)
        {
            value = ClampZipTimestamp(value);
            return (ushort)(((value.Year - 1980) << 9) | (value.Month << 5) | value.Day);
        }
    }
}
'@
}

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

    $destination = [IO.Path]::GetFullPath($DestinationPath)
    $parent = Split-Path -Parent $destination
    if (-not (Test-Path -LiteralPath $parent -PathType Container)) {
        throw "Updater archive directory is missing: $parent"
    }

    [DjeetaMod.StoredZipWriter]::Create($Installer.FullName, $destination)
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

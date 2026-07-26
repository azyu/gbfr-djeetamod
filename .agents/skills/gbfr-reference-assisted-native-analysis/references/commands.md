# Command and Evidence Reference

## Inputs

Use explicit paths supplied by the task or resolved from the running target.
Never hard-code a user profile or download directory in committed artifacts.

```powershell
Get-FileHash -Algorithm SHA256 -LiteralPath $gameExecutable
Get-FileHash -Algorithm SHA256 -LiteralPath $referenceArchive
```

## Temporary Extraction

```powershell
$analysisDirectory = Join-Path ([System.IO.Path]::GetTempPath()) `
  ('djeeta-native-reference-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $analysisDirectory | Out-Null
Expand-Archive -LiteralPath $referenceArchive -DestinationPath $analysisDirectory
```

Prefer an already installed `ildasm.exe`. Keep its output under
`$analysisDirectory`.

## Signature Recovery

```powershell
python scripts/extract_ildasm_signatures.py `
  --il $ildasmOutput `
  --class-name NativeAutomation `
  --output (Join-Path $analysisDirectory 'patterns.json')

python scripts/scan_pe_signatures.py `
  --exe $gameExecutable `
  --patterns (Join-Path $analysisDirectory 'patterns.json') `
  --require-unique `
  --output (Join-Path $analysisDirectory 'matches.json')
```

Read only the entries relevant to the failed boundary. Do not paste generated
IL or full tool output into the repository.

## Evidence Record

```text
Reference method or symbol:
Behavioral boundary:
Candidate game function:
Candidate field, callback, or event:
Independent signature count and RVA:
ABI and owner corroboration:
Positive observation:
Hidden/stale or unrelated negative:
Successor:
Promotion result:
Remaining uncertainty:
```

## Safe Cleanup

Resolve and display the two paths first:

```powershell
$resolvedAnalysis = (Resolve-Path -LiteralPath $analysisDirectory).Path
$resolvedTemp = (Resolve-Path -LiteralPath ([System.IO.Path]::GetTempPath())).Path.TrimEnd('\')
$isChild = $resolvedAnalysis.StartsWith(
  $resolvedTemp + '\',
  [System.StringComparison]::OrdinalIgnoreCase
)
if (-not $isChild -or $resolvedAnalysis -eq $resolvedTemp) {
  throw 'Reference directory escaped the temporary root'
}
```

After that verification succeeds, delete the exact resolved directory in the
same PowerShell environment and confirm it no longer exists.

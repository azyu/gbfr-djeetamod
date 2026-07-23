# Release Workflow Security Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove GitHub Actions script injection paths and ensure updater signing credentials are visible only while signing a previously prepared updater archive.

**Architecture:** Keep `package:nsis` as a credential-free preparation command that verifies and builds the installer and updater ZIP. Add `package:sign` to validate and sign those prepared bytes, generate the updater manifest, and finalize hashes. Split the release workflow into preflight, prepare, sign, and publish steps with separate environment scopes.

**Tech Stack:** GitHub Actions YAML, Windows PowerShell 5, Tauri 1 CLI, npm scripts, Vitest static configuration tests.

## Global Constraints

- Preserve `npm.cmd run package:nsis` as the canonical NSIS preparation entry point.
- Preserve current-user NSIS packaging, signed updater verification, draft-first publication, four release assets, and hash documentation.
- Never expose `TAURI_PRIVATE_KEY` or `TAURI_KEY_PASSWORD` to `npm ci`, frontend tests/builds, Cargo, or publication commands.
- Never interpolate `inputs.version` or `inputs.publish` inside inline PowerShell source.
- Keep checkout credentials disabled until the trusted publication phase.
- Do not modify or stage the user's `AGENTS.md` change.
- Use one implementation commit after all tasks and required verification pass.

---

### Task 1: Create updater archives without signing credentials

**Files:**
- Modify: `scripts/PackageHelpers.psm1`
- Modify: `scripts/tests/PackageHelpers.Tests.ps1`

**Interfaces:**
- Consumes: a current-build NSIS installer represented by `System.IO.FileInfo`.
- Produces: `New-NsisUpdaterArchive -Installer <FileInfo> -DestinationPath <string>` returning the created `System.IO.FileInfo`.

- [ ] **Step 1: Write the failing archive-format test**

Append a PowerShell test that creates a temporary installer, calls `New-NsisUpdaterArchive`, opens the result with `System.IO.Compression.ZipFile`, and asserts:

```powershell
$archiveTestRoot = Join-Path ([IO.Path]::GetTempPath()) ('djeeta-updater-archive-test-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $archiveTestRoot | Out-Null
try {
    $installerPath = Join-Path $archiveTestRoot 'Djeeta MOD_0.1.2_x64-setup.exe'
    [IO.File]::WriteAllBytes($installerPath, [byte[]](1, 2, 3, 4))
    $archivePath = Join-Path $archiveTestRoot 'Djeeta MOD_0.1.2_x64-setup.nsis.zip'
    $created = New-NsisUpdaterArchive -Installer (Get-Item -LiteralPath $installerPath) -DestinationPath $archivePath

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($created.FullName)
    try {
        Assert-Equal $zip.Entries.Count 1 'Updater archive must contain exactly one file.'
        Assert-Equal $zip.Entries[0].FullName 'Djeeta MOD_0.1.2_x64-setup.exe' 'Updater archive entry name failed.'
        Assert-Equal $zip.Entries[0].Length 4 'Updater archive entry length failed.'
    }
    finally {
        $zip.Dispose()
    }
}
finally {
    Remove-Item -LiteralPath $archiveTestRoot -Recurse -Force
}
```

- [ ] **Step 2: Run the focused PowerShell test and verify RED**

Run:

```powershell
npm.cmd run test:package-helpers
```

Expected: FAIL because `New-NsisUpdaterArchive` is not exported.

- [ ] **Step 3: Implement the archive helper**

Add and export this focused helper:

```powershell
function New-NsisUpdaterArchive {
    param(
        [Parameter(Mandatory)][System.IO.FileInfo]$Installer,
        [Parameter(Mandatory)][string]$DestinationPath
    )

    Add-Type -AssemblyName System.IO.Compression
    $destination = [IO.Path]::GetFullPath($DestinationPath)
    $parent = Split-Path -Parent $destination
    if (-not (Test-Path -LiteralPath $parent -PathType Container)) {
        throw "Updater archive directory is missing: $parent"
    }

    $fileStream = [IO.File]::Open($destination, [IO.FileMode]::Create, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try {
        $archive = New-Object IO.Compression.ZipArchive(
            $fileStream,
            [IO.Compression.ZipArchiveMode]::Create,
            $false
        )
        try {
            $entry = $archive.CreateEntry($Installer.Name, [IO.Compression.CompressionLevel]::NoCompression)
            $input = $Installer.OpenRead()
            $output = $entry.Open()
            try {
                $input.CopyTo($output)
            }
            finally {
                $output.Dispose()
                $input.Dispose()
            }
        }
        finally {
            $archive.Dispose()
        }
    }
    finally {
        $fileStream.Dispose()
    }

    return Get-Item -LiteralPath $destination
}
```

- [ ] **Step 4: Run the focused PowerShell test and verify GREEN**

Run:

```powershell
npm.cmd run test:package-helpers
```

Expected: PASS with `Package helper tests passed.`

---

### Task 2: Split credential-free preparation from signing and finalization

**Files:**
- Modify: `package.json`
- Modify: `scripts/package.ps1`
- Create: `scripts/sign-package.ps1`
- Modify: `scripts/build-release.ps1`
- Modify: `src/securityConfiguration.test.ts`
- Modify: `src/releaseBuildCommand.test.ts`

**Interfaces:**
- Consumes: `New-NsisUpdaterArchive` from Task 1.
- Produces: `target/release/package-preparation.json`, `target/release/package-summary.json`, and npm script `package:sign`.

- [ ] **Step 1: Write failing phase-separation tests**

Update the static tests to require all of these conditions:

```ts
expect(packageJson.scripts["package:sign"]).toBe(
  "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/sign-package.ps1"
);
expect(packagingScript).not.toContain("Assert-UpdaterSigningEnvironment");
expect(packagingScript).not.toContain("TAURI_PRIVATE_KEY");
expect(packagingScript).not.toMatch(/'build',\s*'--bundles',\s*'nsis',\s*'updater'/);
expect(packagingScript).toContain("New-NsisUpdaterArchive");
expect(packagingScript).toContain("package-preparation.json");
expect(signingScript).toContain("Assert-UpdaterSigningEnvironment");
expect(signingScript).toContain("'signer', 'sign'");
expect(signingScript).not.toContain("npm.cmd ci");
expect(signingScript).not.toContain("cargo build");
expect(signingScript).toContain("package-summary.json");
```

Update the local wrapper test to require removal of `TAURI_KEY_PASSWORD` before `package:nsis`, restoration of both signing variables only before `package:sign`, and cleanup in `finally`.

- [ ] **Step 2: Run focused Vitest tests and verify RED**

Run:

```powershell
npm.cmd test -- --run src/securityConfiguration.test.ts src/releaseBuildCommand.test.ts
```

Expected: FAIL because `package:sign`, `scripts/sign-package.ps1`, and phase separation do not exist.

- [ ] **Step 3: Make `package:nsis` credential-free**

In `scripts/package.ps1`:

- remove `Assert-UpdaterSigningEnvironment`;
- retain version agreement, game-process guard, helper tests, `npm ci`, frontend verification, hook build, workspace Rust tests, and hook copy;
- invoke Tauri with `--bundles nsis` only;
- select the installer produced after `$buildStartedAt`;
- create the exact `.nsis.zip` name through `New-NsisUpdaterArchive`;
- verify installer, archive, release hook, and bundled hook hashes;
- write `target/release/package-preparation.json` with:

```powershell
[ordered]@{
    Version = $productVersion
    ProductName = $productName
    BuildStartedAt = $buildStartedAt.ToUniversalTime().ToString('o')
    InstallerPath = $installer.FullName
    InstallerSHA256 = $installerHash
    HookPath = $releaseHookPath
    HookSHA256 = $releaseHookHash
    BundledHookPath = $bundledHookPath
    UpdaterArchivePath = $updaterArchive.FullName
    UpdaterArchiveSHA256 = $updaterArchiveHash
    ReleaseNotes = $releaseNotes
}
```

Do not create a signature, `latest.json`, final summary, or documentation changes in this phase.

- [ ] **Step 4: Add the signing/finalization script**

Create `scripts/sign-package.ps1` with `RequestedVersion` input. It must:

1. import `PackageHelpers.psm1`;
2. validate both signing environment variables;
3. read `target/release/package-preparation.json`;
4. require stable version equality and exact product artifact names;
5. recompute and compare installer, hook, bundled-hook, and archive SHA-256 values;
6. invoke:

```powershell
Invoke-NativeCommand -FilePath $npmPath -Arguments @(
    'run', 'tauri', '--',
    'signer', 'sign',
    $preparation.UpdaterArchivePath
)
```

7. select the archive/signature pair using `Select-ProductNsisUpdaterArtifacts`;
8. create `target/release/latest.json`;
9. update `README.md` and `docs/testing/game-2.0.2-smoke-test.md`;
10. write the existing final `package-summary.json` shape.

- [ ] **Step 5: Update npm and local release entry points**

Add:

```json
"package:sign": "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/sign-package.ps1"
```

Change `scripts/build-release.ps1` so it captures and removes `TAURI_KEY_PASSWORD`, runs `package:nsis` without signing variables, then sets both variables only around `package:sign` and clears them in `finally`.

- [ ] **Step 6: Run focused phase-separation tests and verify GREEN**

Run:

```powershell
npm.cmd run test:package-helpers
npm.cmd test -- --run src/securityConfiguration.test.ts src/releaseBuildCommand.test.ts
```

Expected: both commands PASS.

---

### Task 3: Harden the GitHub release workflow

**Files:**
- Modify: `.github/workflows/release.yaml`
- Modify: `src/releaseWorkflow.test.ts`

**Interfaces:**
- Consumes: `package:nsis`, `package:sign`, preparation summary, and final summary from Task 2.
- Produces: preflight, prepare, sign, and publish workflow steps with distinct credential scopes.

- [ ] **Step 1: Write the failing workflow-boundary test**

Require:

```ts
expect(workflow).toContain("persist-credentials: false");
expect(workflow).not.toMatch(/^\s{4}env:\s*\r?\n(?:\s{6}.+\r?\n)*\s{6}TAURI_PRIVATE_KEY:/m);
expect(workflow).not.toContain("$version = '${{ inputs.version }}'");
expect(workflow).not.toContain("'${{ inputs.publish }}'");
expect(workflow).toContain("RELEASE_VERSION: ${{ inputs.version }}");
expect(workflow).toContain("PUBLISH_RELEASE: ${{ inputs.publish }}");
expect(workflow).toContain("$version = $env:RELEASE_VERSION");
expect(workflow).toContain("npm.cmd run package:nsis");
expect(workflow).toContain("npm.cmd run package:sign");
expect(workflow).toContain("gh auth setup-git");
```

Extract the signing step text and assert it contains both Tauri secrets but none of `npm ci`, `cargo`, `package:nsis`, `git push`, or `gh release`.

- [ ] **Step 2: Run the focused workflow test and verify RED**

Run:

```powershell
npm.cmd test -- --run src/releaseWorkflow.test.ts
```

Expected: FAIL on persisted credentials, job-level signing secrets, and direct input interpolation.

- [ ] **Step 3: Split the workflow into trust phases**

Implement these steps:

1. Checkout exact SHA with `persist-credentials: false`.
2. Setup Node exact SHA.
3. **Preflight release** with step-scoped `RELEASE_VERSION` and `GH_TOKEN`; validate the version, check tag/release absence, and write release notes to `$env:RUNNER_TEMP\djeeta-release-notes.md`.
4. **Build unsigned artifacts** with only `RELEASE_VERSION`; call `package:nsis`.
5. **Sign updater archive** with only `RELEASE_VERSION`, `TAURI_PRIVATE_KEY`, and `TAURI_KEY_PASSWORD`; call `package:sign`.
6. **Publish verified release** with `RELEASE_VERSION`, `PUBLISH_RELEASE`, and `GH_TOKEN`; run `gh auth setup-git`, then preserve existing document-diff, commit, push, tag, draft, upload, digest, manifest, signature, and optional publication checks.

Every inline script reads dispatch values only through `$env:RELEASE_VERSION` and `$env:PUBLISH_RELEASE`.

- [ ] **Step 4: Run the focused workflow test and verify GREEN**

Run:

```powershell
npm.cmd test -- --run src/releaseWorkflow.test.ts
```

Expected: PASS.

---

### Task 4: Verify and commit the security stage

**Files:**
- Verify all files changed by Tasks 1-3.
- Preserve: `AGENTS.md`

**Interfaces:**
- Consumes: completed preparation/signing/workflow implementation.
- Produces: one verified implementation commit.

- [ ] **Step 1: Run focused regression tests**

```powershell
npm.cmd run test:package-helpers
npm.cmd test -- --run src/securityConfiguration.test.ts src/releaseBuildCommand.test.ts src/releaseWorkflow.test.ts
```

Expected: PASS.

- [ ] **Step 2: Run required frontend verification**

```powershell
npm.cmd run format-check
npm.cmd run lint
npm.cmd run tsc
npm.cmd test -- --run
npm.cmd run build
```

Expected: every command exits 0.

- [ ] **Step 3: Inspect the final diff and credential boundaries**

```powershell
git diff --check
git diff -- .github/workflows/release.yaml package.json scripts/package.ps1 scripts/sign-package.ps1 scripts/build-release.ps1 scripts/PackageHelpers.psm1 scripts/tests/PackageHelpers.Tests.ps1 src/releaseWorkflow.test.ts src/releaseBuildCommand.test.ts src/securityConfiguration.test.ts
git status --short
```

Expected: no whitespace errors; only intended release-security files plus the already-existing user change to `AGENTS.md`.

- [ ] **Step 4: Commit only the implementation files**

```powershell
git add -- .github/workflows/release.yaml package.json scripts/package.ps1 scripts/sign-package.ps1 scripts/build-release.ps1 scripts/PackageHelpers.psm1 scripts/tests/PackageHelpers.Tests.ps1 src/releaseWorkflow.test.ts src/releaseBuildCommand.test.ts src/securityConfiguration.test.ts docs/superpowers/plans/2026-07-23-release-workflow-security.md
git commit -m "fix: isolate release signing credentials"
```

Expected: commit succeeds without staging `AGENTS.md`.

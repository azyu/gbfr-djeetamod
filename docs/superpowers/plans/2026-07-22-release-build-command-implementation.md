# Release Build Command Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a double-clickable Windows command that securely prepares updater signing credentials and delegates to the existing canonical release packager.

**Architecture:** A root `build-release.cmd` is the human-facing entry point and delegates all credential handling to one focused PowerShell wrapper. The wrapper reads the per-user private key, prompts for the password as a `SecureString`, sets process-scoped Tauri variables, calls the unchanged npm packaging entry point, and always clears credentials.

**Tech Stack:** Windows CMD, Windows PowerShell 5.1, npm scripts, Vitest static contract tests, Tauri 1 signed updater packaging.

## Global Constraints

- `npm.cmd run package:nsis` remains the sole implementation of the release build.
- The private key path resolves to `%USERPROFILE%\.djeeta-mod\updater.key` and is never committed or printed.
- The password is never accepted as a command-line argument or stored in a file.
- `TAURI_PRIVATE_KEY` and `TAURI_KEY_PASSWORD` are removed on success and failure.
- The command file preserves the PowerShell exit code and pauses so double-click output remains visible.
- Existing uncommitted signed-package hash updates in `README.md` and `docs/testing/game-2.0.2-smoke-test.md` must be preserved and committed together.
- Do not read, modify, stage, or commit `logs.db`.

---

### Task 1: Finalize the verified signed packager changes

**Files:**
- Modify: `scripts/package.ps1`
- Modify: `src/securityConfiguration.test.ts`
- Modify generated: `README.md`
- Modify generated: `docs/testing/game-2.0.2-smoke-test.md`

**Interfaces:**
- Consumes: `Select-ProductNsisUpdaterArtifacts`, `New-TauriUpdaterManifest`, signing environment variables, and the successful local `0.1.1` signed build.
- Produces: the committed canonical packager contract used by the wrapper in Task 2.

- [ ] **Step 1: Re-run focused packaging regressions**

Run:

```powershell
npm.cmd run test:package-helpers
npm.cmd test -- --run src/securityConfiguration.test.ts src/releaseWorkflow.test.ts
git diff --check
```

Expected: package helper tests pass, all 10 Vitest cases pass, and `git diff --check` exits 0.

- [ ] **Step 2: Revalidate the generated artifact summary without exposing signatures**

Run:

```powershell
$summary = Get-Content -Raw target/release/package-summary.json | ConvertFrom-Json
foreach ($path in @($summary.InstallerPath, $summary.HookPath, $summary.UpdaterArchivePath, $summary.UpdaterSignaturePath, $summary.LatestJsonPath)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing artifact: $path" }
}
if ((Get-FileHash -Algorithm SHA256 -LiteralPath $summary.InstallerPath).Hash -ne $summary.InstallerSHA256) { throw 'Installer hash mismatch.' }
if ((Get-FileHash -Algorithm SHA256 -LiteralPath $summary.UpdaterArchivePath).Hash -ne $summary.UpdaterArchiveSHA256) { throw 'Updater hash mismatch.' }
if ((Get-FileHash -Algorithm SHA256 -LiteralPath $summary.HookPath).Hash -ne $summary.HookSHA256) { throw 'Hook hash mismatch.' }
$manifest = Get-Content -Raw -LiteralPath $summary.LatestJsonPath | ConvertFrom-Json
$signature = [IO.File]::ReadAllText($summary.UpdaterSignaturePath).Trim()
if ($manifest.platforms.'windows-x86_64'.signature -ne $signature) { throw 'Manifest signature mismatch.' }
```

Expected: exit 0 with no secret or signature content printed.

- [ ] **Step 3: Commit the canonical packager and generated hashes together**

```powershell
git add scripts/package.ps1 src/securityConfiguration.test.ts README.md docs/testing/game-2.0.2-smoke-test.md
git commit -m "feat: package signed update releases"
```

Expected: exactly those four files are committed; files under `target/` remain ignored.

---

### Task 2: Add the secure double-click release entry point

**Files:**
- Create: `src/releaseBuildCommand.test.ts`
- Create: `build-release.cmd`
- Create: `scripts/build-release.ps1`

**Interfaces:**
- Consumes: `%USERPROFILE%\.djeeta-mod\updater.key`, a masked interactive password, and the `package:nsis` npm script.
- Produces: `build-release.cmd` with no arguments and a process exit code equal to the canonical package command result.

- [ ] **Step 1: Write the failing command security contract**

Create `src/releaseBuildCommand.test.ts`:

```ts
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, it } from "vitest";

const readRepositoryFile = (path: string) => readFileSync(resolve(process.cwd(), path), "utf8");

it("delegates double-click release builds to the secure PowerShell wrapper", () => {
  const command = readRepositoryFile("build-release.cmd");

  expect(command).toContain('powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\\build-release.ps1"');
  expect(command).toContain('set "BUILD_EXIT_CODE=%ERRORLEVEL%"');
  expect(command).toContain("pause");
  expect(command).toContain("exit /b %BUILD_EXIT_CODE%");
});

it("keeps updater credentials process-scoped and always clears them", () => {
  const wrapper = readRepositoryFile("scripts/build-release.ps1");

  expect(wrapper).toContain("[Environment]::GetFolderPath('UserProfile')");
  expect(wrapper).toContain("'.djeeta-mod\\updater.key'");
  expect(wrapper).toContain("Read-Host 'Updater key password' -AsSecureString");
  expect(wrapper).toContain("SecureStringToBSTR");
  expect(wrapper).toContain("PtrToStringBSTR");
  expect(wrapper).toContain("Get-Command npm.cmd");
  expect(wrapper).toContain("& $npmPath run package:nsis");
  expect(wrapper).toContain("finally");
  expect(wrapper).toContain("ZeroFreeBSTR");
  expect(wrapper).toContain("Remove-Item Env:TAURI_PRIVATE_KEY");
  expect(wrapper).toContain("Remove-Item Env:TAURI_KEY_PASSWORD");
  expect(wrapper).not.toMatch(/--password|-p\s+['\"]/);
});
```

- [ ] **Step 2: Run the contract to verify RED**

Run:

```powershell
npm.cmd test -- --run src/releaseBuildCommand.test.ts
```

Expected: FAIL because `build-release.cmd` and `scripts/build-release.ps1` do not exist.

- [ ] **Step 3: Implement the minimal command entry point**

Create `build-release.cmd`:

```bat
@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\build-release.ps1"
set "BUILD_EXIT_CODE=%ERRORLEVEL%"
echo.
pause
exit /b %BUILD_EXIT_CODE%
```

- [ ] **Step 4: Implement the secure PowerShell wrapper**

Create `scripts/build-release.ps1`:

```powershell
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
```

- [ ] **Step 5: Run focused GREEN verification**

Run:

```powershell
npm.cmd test -- --run src/releaseBuildCommand.test.ts
$tokens = $null
$errors = $null
[Management.Automation.Language.Parser]::ParseFile((Resolve-Path 'scripts/build-release.ps1'), [ref]$tokens, [ref]$errors) | Out-Null
if ($errors.Count -ne 0) { throw ($errors.Message -join "`n") }
git diff --check
```

Expected: 2 tests pass, the parser reports no errors, and the diff check exits 0.

- [ ] **Step 6: Commit the release entry point**

```powershell
git add build-release.cmd scripts/build-release.ps1 src/releaseBuildCommand.test.ts
git commit -m "feat: add secure release build command"
```

---

### Task 3: Run the complete automated regression suite

**Files:**
- Verify only: all implementation files from Tasks 1 and 2

**Interfaces:**
- Consumes: the committed canonical packager and release command.
- Produces: final automated evidence before GitHub integration continues.

- [ ] **Step 1: Run frontend and packaging verification**

Run:

```powershell
npm.cmd run test:package-helpers
npm.cmd run format-check
npm.cmd run lint
npm.cmd run tsc
npm.cmd test -- --run
npm.cmd run build
```

Expected: every command exits 0. Existing Tauri browser-environment and React Router warnings may appear; there are no failing tests.

- [ ] **Step 2: Run Rust verification with the locked updater dependency graph**

Run:

```powershell
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
```

Expected: both commands exit 0; existing dead-code warnings are allowed.

- [ ] **Step 3: Confirm repository hygiene**

Run:

```powershell
git diff --check
git status --short
git check-ignore target/release/latest.json target/release/package-summary.json
```

Expected: no uncommitted implementation changes, generated target files are ignored, and neither private key files nor `logs.db` appear.

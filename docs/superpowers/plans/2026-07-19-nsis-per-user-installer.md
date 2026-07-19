# NSIS Per-User Installer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the per-machine MSI with one Tauri NSIS setup executable that installs Djeeta MOD for the current Windows user.

**Architecture:** Vitest protects the real Tauri security and packaging configuration. PowerShell helpers select one fresh NSIS setup executable and update its hash, while the existing orchestrator continues to run every frontend/Rust gate, synchronize `hook.dll`, package, and update active release records.

**Tech Stack:** Windows x64, Tauri 1.5, NSIS, PowerShell 5.1+, TypeScript, Vitest, Rust nightly-2024-05-04, SHA-256

## Global Constraints

- Produce NSIS only with `bundle.windows.nsis.installMode` exactly `currentUser`.
- Keep `src-tauri/manifest.xml` at `asInvoker` and keep SmartScreen protection enabled.
- Preserve automatic `hook.dll` injection and all gameplay behavior.
- Existing MSI installs are removed manually; never delete `C:\Program Files\Djeeta MOD` automatically.
- Do not add Defender exclusions or claim that per-user installation eliminates behavior-based detection.
- Do not rewrite historical specs/plans or touch the unrelated untracked `logs.db`.
- Do not claim game 2.0.2 compatibility until the manual smoke checklist passes.

---

## File map

- `src/securityConfiguration.test.ts`: real manifest, WebView, Tauri target/scope, and npm packaging-command contracts.
- `src-tauri/tauri.conf.json`: single NSIS target and current-user scope.
- `scripts/PackageHelpers.psm1`: fresh NSIS selection and release-hash rewriting.
- `scripts/tests/PackageHelpers.Tests.ps1`: helper red/green coverage.
- `scripts/package.ps1`: complete verified NSIS workflow.
- `package.json`: public `package:nsis` entry point.
- `README.md`, `docs/testing/game-2.0.2-smoke-test.md`, `AGENTS.md`: active user, test, and maintainer contracts.

### Task 1: Lock Tauri to current-user NSIS

**Files:**
- Modify: `src/securityConfiguration.test.ts`
- Modify: `src-tauri/tauri.conf.json`

**Interfaces:**
- Consumes: Tauri `bundle.targets` and `bundle.windows.nsis.installMode`.
- Produces: only `nsis`, with install mode `currentUser`.

- [ ] **Step 1: Add the failing configuration test**

Add to `src/securityConfiguration.test.ts`:

```ts
it("packages only a current-user NSIS installer", () => {
  const config = JSON.parse(readRepositoryFile("src-tauri/tauri.conf.json")) as {
    tauri: {
      bundle: {
        targets: string[];
        windows: { nsis?: { installMode?: string } };
      };
    };
  };

  expect(config.tauri.bundle.targets).toEqual(["nsis"]);
  expect(config.tauri.bundle.targets).not.toContain("msi");
  expect(config.tauri.bundle.windows.nsis?.installMode).toBe("currentUser");
});
```

- [ ] **Step 2: Prove RED**

Run: `npm test -- --run src/securityConfiguration.test.ts`

Expected: the new test fails on the current `['msi']` target and missing `nsis` object; the existing two tests pass.

- [ ] **Step 3: Make the minimal configuration change**

In `src-tauri/tauri.conf.json`, retain `webviewInstallMode` and make the Windows bundle block:

```json
"targets": ["nsis"],
"windows": {
  "webviewInstallMode": {
    "type": "embedBootstrapper"
  },
  "nsis": {
    "installMode": "currentUser"
  }
}
```

Do not change the disabled updater's `basicUi` property; it is not installer scope.

- [ ] **Step 4: Prove GREEN and commit**

Run: `npm test -- --run src/securityConfiguration.test.ts`

Expected: all 3 tests pass.

```powershell
git add src/securityConfiguration.test.ts src-tauri/tauri.conf.json
git commit -m "build: target current-user NSIS installer"
```

### Task 2: Convert package helpers from MSI to NSIS

**Files:**
- Modify: `scripts/tests/PackageHelpers.Tests.ps1`
- Modify: `scripts/PackageHelpers.psm1`

**Interfaces:**
- Produces: `Select-ProductNsisInstaller -Artifacts <object[]> -ProductName <string> -Version <string> -BuildStartedAt <datetime> -> object`.
- Produces: `Set-ArtifactHashesInText -Text <string> -InstallerHash <64 hex> -HookHash <64 hex> -> string`.

- [ ] **Step 1: Replace artifact fixtures with failing NSIS cases**

Replace the MSI selection cases in `scripts/tests/PackageHelpers.Tests.ps1` with:

```powershell
$buildStartedAt = [datetime]'2026-07-19T01:00:00Z'
$productInstaller = [pscustomobject]@{
    Name = 'Djeeta MOD_0.1.0_x64-setup.exe'
    LastWriteTimeUtc = [datetime]'2026-07-19T01:00:01Z'
}
$otherInstaller = [pscustomobject]@{
    Name = 'build_trait_caps_0.1.0_x64-setup.exe'
    LastWriteTimeUtc = [datetime]'2026-07-19T01:00:02Z'
}
$applicationExe = [pscustomobject]@{
    Name = 'Djeeta MOD.exe'
    LastWriteTimeUtc = [datetime]'2026-07-19T01:00:03Z'
}
$selected = Select-ProductNsisInstaller -Artifacts @($productInstaller, $otherInstaller, $applicationExe) -ProductName 'Djeeta MOD' -Version '0.1.0' -BuildStartedAt $buildStartedAt
Assert-Equal $selected.Name $productInstaller.Name 'Product installer selection failed.'
Assert-Throws {
    Select-ProductNsisInstaller -Artifacts @($otherInstaller, $applicationExe) -ProductName 'Djeeta MOD' -Version '0.1.0' -BuildStartedAt $buildStartedAt
} 'A non-product result must fail.'
Assert-Throws {
    Select-ProductNsisInstaller -Artifacts @($productInstaller, $productInstaller) -ProductName 'Djeeta MOD' -Version '0.1.0' -BuildStartedAt $buildStartedAt
} 'Multiple product installers must fail.'
Assert-Throws {
    Select-ProductNsisInstaller -Artifacts @([pscustomobject]@{
        Name = 'Djeeta MOD_0.1.0_x64-setup.exe'
        LastWriteTimeUtc = [datetime]'2026-07-19T00:59:59Z'
    }) -ProductName 'Djeeta MOD' -Version '0.1.0' -BuildStartedAt $buildStartedAt
} 'A stale installer must fail.'
```

Replace the hash fixtures/calls with:

```powershell
$oldInstaller = 'A' * 64
$oldHook = 'B' * 64
$newInstaller = 'C' * 64
$newHook = 'D' * 64
$readme = @'
- NSIS installer: `AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA`
- `hook.dll`: `BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB`
'@
$smoke = @'
- NSIS installer SHA-256: `AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA`
- `hook.dll` SHA-256: `BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB`
'@
$updatedReadme = Set-ArtifactHashesInText -Text $readme -InstallerHash $newInstaller -HookHash $newHook
$updatedSmoke = Set-ArtifactHashesInText -Text $smoke -InstallerHash $newInstaller -HookHash $newHook
Assert-Equal ([regex]::Matches($updatedReadme, $newInstaller).Count) 1 'README installer replacement failed.'
Assert-Equal ([regex]::Matches($updatedSmoke, $newInstaller).Count) 1 'Smoke installer replacement failed.'
Assert-Throws {
    Set-ArtifactHashesInText -Text ($readme + "`r`n" + $readme) -InstallerHash $newInstaller -HookHash $newHook
} 'Duplicate labels must fail.'
Assert-Throws {
    Set-ArtifactHashesInText -Text "- NSIS installer: ```$oldInstaller```" -InstallerHash $newInstaller -HookHash $newHook
} 'Missing hook label must fail.'
```

- [ ] **Step 2: Prove RED**

Run: `npm run test:package-helpers`

Expected: failure because `Select-ProductNsisInstaller` and `InstallerHash` do not exist.

- [ ] **Step 3: Implement strict NSIS selection**

Replace `Select-ProductMsi` in `scripts/PackageHelpers.psm1` with:

```powershell
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
```

- [ ] **Step 4: Generalize hash rewriting**

Change the hash function parameter to `InstallerHash`, validate it alongside `HookHash`, and use:

```powershell
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
```

Export `Select-ProductNsisInstaller` instead of `Select-ProductMsi`.

- [ ] **Step 5: Prove GREEN and commit**

Run: `npm run test:package-helpers`

Expected: `Package helper tests passed.`

```powershell
git add scripts/PackageHelpers.psm1 scripts/tests/PackageHelpers.Tests.ps1
git commit -m "build: validate NSIS package artifacts"
```

### Task 3: Convert the packaging entry point

**Files:**
- Modify: `src/securityConfiguration.test.ts`
- Modify: `package.json`
- Modify: `scripts/package.ps1`

**Interfaces:**
- Consumes: the Task 2 helpers.
- Produces: `npm run package:nsis` and summary fields `InstallerPath`, `InstallerSHA256`, `HookSHA256`, `HookHashesEqual`.

- [ ] **Step 1: Add a failing public-command contract**

Add to `src/securityConfiguration.test.ts`:

```ts
it("exposes only the verified NSIS packaging command", () => {
  const packageJson = JSON.parse(readRepositoryFile("package.json")) as {
    scripts: Record<string, string>;
  };
  const packagingScript = readRepositoryFile("scripts/package.ps1");

  expect(packageJson.scripts["package:nsis"]).toBe(
    "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package.ps1",
  );
  expect(packageJson.scripts).not.toHaveProperty("package:msi");
  expect(packagingScript).toContain("'target\\release\\bundle\\nsis'");
  expect(packagingScript).toMatch(/'build',\s*'--bundles',\s*'nsis'/);
  expect(packagingScript).not.toMatch(/'build',\s*'--bundles',\s*'msi'/);
});
```

- [ ] **Step 2: Prove RED**

Run: `npm test -- --run src/securityConfiguration.test.ts`

Expected: failure because only `package:msi` and MSI paths exist.

- [ ] **Step 3: Change the npm command and orchestrator**

In `package.json`, replace `package:msi` with:

```json
"package:nsis": "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package.ps1"
```

In `scripts/package.ps1`:

- change the Windows error to `NSIS packaging is supported only on Windows.`;
- pass `'build', '--bundles', 'nsis'` to Tauri;
- read `target\release\bundle\nsis\*.exe`;
- call `Select-ProductNsisInstaller`;
- calculate `$installerHash` and pass `-InstallerHash $installerHash`;
- output `InstallerPath` and `InstallerSHA256` instead of MSI fields.

Use this artifact block exactly:

```powershell
$installerArtifacts = @(Get-ChildItem -LiteralPath 'target\release\bundle\nsis' -Filter '*.exe')
$installer = Select-ProductNsisInstaller -Artifacts $installerArtifacts -ProductName $productName -Version $productVersion -BuildStartedAt $buildStartedAt
$installerHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $installer.FullName).Hash
```

Use this summary:

```powershell
[pscustomobject]@{
    InstallerPath = $installer.FullName
    InstallerSHA256 = $installerHash
    HookSHA256 = $releaseHookHash
    HookHashesEqual = $true
    UpdatedDocuments = @($updatedDocuments.Keys)
} | Format-List
```

- [ ] **Step 4: Prove GREEN and commit**

Run:

```powershell
npm test -- --run src/securityConfiguration.test.ts
npm run test:package-helpers
```

Expected: 4 security tests pass and helper tests print their success message.

```powershell
git add src/securityConfiguration.test.ts package.json scripts/package.ps1
git commit -m "build: package verified NSIS installer"
```

### Task 4: Update active documentation and package the release

**Files:**
- Modify: `README.md`
- Modify: `docs/testing/game-2.0.2-smoke-test.md`
- Modify: `AGENTS.md`
- Generate: `target/release/bundle/nsis/Djeeta MOD_0.1.0_x64-setup.exe`

**Interfaces:**
- Consumes: `package:nsis` and the NSIS hash labels required by Task 2.
- Produces: bilingual migration guidance, final installer/hook hashes, and a verified setup executable.

- [ ] **Step 1: Update current Korean and English guidance**

Make surgical replacements in `README.md`; do not reformat unrelated text. Add these notes before the respective installation steps:

```markdown
> 이전 MSI 버전이 `C:\Program Files\Djeeta MOD`에 설치되어 있다면 Windows **설치된 앱**에서 먼저 제거한 뒤 새 NSIS 설치 프로그램을 실행하십시오. 새 버전은 현재 Windows 사용자 전용으로 설치되며 관리자 권한을 요구하지 않습니다.
```

```markdown
> If an earlier MSI build is installed under `C:\Program Files\Djeeta MOD`, remove it from Windows **Installed apps** before running the new NSIS setup executable. The new build installs for the current Windows user and does not require administrator privileges.
```

Change active install text from MSI to NSIS setup, `package:msi` to `package:nsis`, and `--bundles msi` to `--bundles nsis`. Change the active hash label, retaining the old 64-hex value until packaging, to:

```markdown
- NSIS installer: `2047CA4D31C11288065B43B3BDF487DC6DD0A201618EFB9234412BB8E3D87231`
```

- [ ] **Step 2: Update smoke and maintainer contracts**

Change the active smoke hash label to:

```markdown
- NSIS installer SHA-256: `2047CA4D31C11288065B43B3BDF487DC6DD0A201618EFB9234412BB8E3D87231`
```

Add a smoke row requiring the old MSI to be removed, NSIS setup to run without administrator elevation, and Djeeta MOD to appear for the current user.

In `AGENTS.md`, replace WiX/MSI packaging language with NSIS, replace the bundle command with `npm run tauri build -- --bundles nsis`, and require NSIS installer plus hook hashes after packaging.

- [ ] **Step 3: Verify active documentation and commit it**

Run:

```powershell
rg -n "package:msi|--bundles msi|^- MSI(?: SHA-256)?:" README.md docs/testing/game-2.0.2-smoke-test.md AGENTS.md
npm run test:package-helpers
git diff --check
```

Expected: `rg` has no matches, helpers pass, and diff check passes. Historical `docs/superpowers` files are intentionally excluded.

```powershell
git add README.md docs/testing/game-2.0.2-smoke-test.md AGENTS.md
git commit -m "docs: guide per-user NSIS installation"
```

- [ ] **Step 4: Run the complete verified package workflow**

Confirm the game is closed, then run:

```powershell
npm run package:nsis
```

Expected: PowerShell helpers, `npm ci`, formatting, linting, TypeScript, all frontend tests, frontend build, release hook build, all Rust tests, and NSIS packaging pass. The summary prints one setup path, its SHA-256, the hook SHA-256, and `HookHashesEqual : True`.

If the game is running, ask the user to exit it; do not terminate it.

- [ ] **Step 5: Independently verify final artifacts**

Run:

```powershell
$installer = @(Get-ChildItem -LiteralPath 'target\release\bundle\nsis' -Filter 'Djeeta MOD_0.1.0_x64-setup.exe')
if ($installer.Count -ne 1) { throw 'Expected exactly one Djeeta MOD NSIS installer.' }
$installerHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $installer[0].FullName).Hash
$releaseHookHash = (Get-FileHash -Algorithm SHA256 -LiteralPath 'target\release\hook.dll').Hash
$bundledHookHash = (Get-FileHash -Algorithm SHA256 -LiteralPath 'src-tauri\hook.dll').Hash
if ($releaseHookHash -ne $bundledHookHash) { throw 'Hook hashes differ.' }
$readme = [System.IO.File]::ReadAllText((Resolve-Path 'README.md'))
$smoke = [System.IO.File]::ReadAllText((Resolve-Path 'docs\testing\game-2.0.2-smoke-test.md'))
if (-not $readme.Contains($installerHash) -or -not $smoke.Contains($installerHash) -or -not $readme.Contains($releaseHookHash) -or -not $smoke.Contains($releaseHookHash)) { throw 'Final hashes are not recorded.' }
Get-AuthenticodeSignature -LiteralPath $installer[0].FullName, 'target\release\Djeeta MOD.exe', 'target\release\hook.dll' | Select-Object Path, Status
git diff --check
git status --short
```

Expected: one installer, matching hook hashes, both active documents contain both final hashes, all three artifacts report `NotSigned`, and only expected hash-document changes plus `?? logs.db` are present.

- [ ] **Step 6: Commit hashes and run fresh post-commit tests**

```powershell
git add README.md docs/testing/game-2.0.2-smoke-test.md
git commit -m "docs: record NSIS package hashes"
npm test -- --run
cargo test --workspace --all-targets --locked
git diff --check
git status --short --branch
```

Expected: all frontend and Rust tests pass; tracked files are clean and only `?? logs.db` remains. Defender execution remains a separate user-approved test.

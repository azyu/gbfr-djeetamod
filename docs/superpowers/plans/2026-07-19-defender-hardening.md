# Defender Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove unnecessary elevation and WebView2 SmartScreen suppression from Djeeta MOD while preserving its existing automatic game-hook behavior.

**Architecture:** Protect the security posture with one focused Vitest file that reads the real release manifest and Tauri configuration. Change only the requested execution level and the two identical WebView argument strings, then package a fresh MSI for a separately approved Defender test.

**Tech Stack:** Tauri 1, Rust, WebView2 configuration, TypeScript, Vitest, PowerShell packaging

## Global Constraints

- The release executable must request `asInvoker`, not `requireAdministrator`.
- Neither Tauri window may disable `msSmartScreenProtection`.
- Preserve `msWebOOUI`, `msPdfOOUI`, and `--disable-gpu` for this controlled comparison.
- Preserve automatic game detection and `dll-syringe` injection behavior.
- Do not restore quarantined files, add Defender exclusions, disable behavior monitoring, or submit files externally.
- Keep `logs.db` untracked and untouched.
- Do not claim Defender compatibility from automated tests alone.

---

### Task 1: Enforce least privilege and WebView2 security defaults

**Files:**
- Create: `src/securityConfiguration.test.ts`
- Modify: `src-tauri/manifest.xml:17`
- Modify: `src-tauri/tauri.conf.json:52`
- Modify: `src-tauri/tauri.conf.json:66`

**Interfaces:**
- Consumes: the release manifest embedded by `src-tauri/build.rs` and the two Tauri window configurations.
- Produces: an `asInvoker` executable and WebView2 child processes that no longer receive a SmartScreen-disabling feature flag; no runtime API changes.

- [ ] **Step 1: Write the failing security configuration tests**

Create `src/securityConfiguration.test.ts`:

```ts
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, it } from "vitest";

const readRepositoryFile = (path: string) =>
  readFileSync(resolve(process.cwd(), path), "utf8");

it("runs the release application with the caller's privileges", () => {
  const manifest = readRepositoryFile("src-tauri/manifest.xml");

  expect(manifest).toContain('requestedExecutionLevel level="asInvoker"');
  expect(manifest).not.toContain("requireAdministrator");
});

it("keeps SmartScreen protection enabled in every WebView window", () => {
  const config = JSON.parse(readRepositoryFile("src-tauri/tauri.conf.json")) as {
    tauri: { windows: Array<{ additionalBrowserArgs?: string }> };
  };

  for (const window of config.tauri.windows) {
    expect(window.additionalBrowserArgs).toBe(
      "--disable-features=msWebOOUI,msPdfOOUI --disable-gpu"
    );
    expect(window.additionalBrowserArgs).not.toContain("msSmartScreenProtection");
  }
});
```

- [ ] **Step 2: Run the focused tests and verify RED**

Run: `npm test -- --run src/securityConfiguration.test.ts`

Expected: two failures showing `requireAdministrator` is still present and both WebView argument strings still contain `msSmartScreenProtection`.

- [ ] **Step 3: Apply the minimal security configuration changes**

In `src-tauri/manifest.xml`, set:

```xml
<requestedExecutionLevel level="asInvoker" uiAccess="false" />
```

For both windows in `src-tauri/tauri.conf.json`, set:

```json
"additionalBrowserArgs": "--disable-features=msWebOOUI,msPdfOOUI --disable-gpu"
```

- [ ] **Step 4: Run the focused tests and verify GREEN**

Run: `npm test -- --run src/securityConfiguration.test.ts`

Expected: one test file and two tests pass.

- [ ] **Step 5: Format and commit the tested change**

```powershell
npx prettier --write src/securityConfiguration.test.ts src-tauri/tauri.conf.json
npm test -- --run src/securityConfiguration.test.ts
git add -- src/securityConfiguration.test.ts src-tauri/manifest.xml src-tauri/tauri.conf.json
git diff --cached --check
git commit -m "fix: reduce Defender risk signals"
```

---

### Task 2: Package and characterize the hardened binaries

**Files:**
- Modify if the generated hash changes: `README.md`
- Modify if the generated hash changes: `docs/testing/game-2.0.2-smoke-test.md`
- Verify: `target/release/Djeeta MOD.exe`
- Verify: `target/release/hook.dll`
- Verify: `src-tauri/hook.dll`
- Generate: `target/release/bundle/msi/Djeeta MOD_0.1.0_x64_en-US.msi`

**Interfaces:**
- Consumes: the hardened manifest and Tauri configuration from Task 1.
- Produces: a fully verified MSI and SHA-256 values suitable for a later Defender test or Microsoft sample submission.

- [ ] **Step 1: Confirm the game is stopped and run the authoritative package workflow**

```powershell
$game = Get-Process -Name 'granblue_fantasy_relink' -ErrorAction SilentlyContinue
if ($game) { throw "Game is still running (PID: $($game.Id -join ', '))." }
npm run package:msi
```

Expected: format, lint, TypeScript, frontend tests, Vite build, release hook build, Rust workspace tests, Tauri MSI build, and hook hash equality all succeed.

- [ ] **Step 2: Independently verify hashes and unsigned status**

```powershell
$exe = 'target/release/Djeeta MOD.exe'
$releaseHook = 'target/release/hook.dll'
$bundledHook = 'src-tauri/hook.dll'
$msi = 'target/release/bundle/msi/Djeeta MOD_0.1.0_x64_en-US.msi'
$exeHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $exe).Hash
$releaseHookHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $releaseHook).Hash
$bundledHookHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $bundledHook).Hash
$msiHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $msi).Hash
if ($releaseHookHash -ne $bundledHookHash) { throw 'Hook hashes differ.' }
$signatures = @($exe, $releaseHook, $msi) | ForEach-Object { Get-AuthenticodeSignature -LiteralPath $_ }
if (@($signatures | Where-Object Status -ne 'NotSigned').Count -ne 0) { throw 'Unexpected signature state.' }
Select-String -Path 'README.md','docs/testing/game-2.0.2-smoke-test.md' -Pattern $releaseHookHash,$msiHash
git diff --check
git status --short
```

Expected: hook hashes match, the MSI and hook hashes occur in both documents, all three artifacts remain explicitly characterized as unsigned, and `logs.db` is the only unrelated untracked path.

- [ ] **Step 3: Commit generated hash documentation only if changed**

```powershell
git add -- README.md docs/testing/game-2.0.2-smoke-test.md
git diff --cached --quiet
if ($LASTEXITCODE -ne 0) { git commit -m "docs: update hardened build hashes" }
```

- [ ] **Step 4: Report the manual security and compatibility gates**

Report the MSI, executable, and hook SHA-256 values. State that Defender behavior must be tested separately with explicit approval, trusted signing remains deferred, and game 2.0.2 compatibility remains unverified until the smoke-test checklist passes.

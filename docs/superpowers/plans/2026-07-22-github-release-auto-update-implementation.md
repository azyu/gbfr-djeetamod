# GitHub Release Auto-Update Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver signed, user-approved Djeeta MOD updates from `azyu/gbfr-djeetamod` GitHub Releases without installing while Granblue Fantasy: Relink is running or leaving the repeat-quest patch active.

**Architecture:** Tauri 1 reads a signed `latest.json` from the stable GitHub Release, while one React provider owns startup/manual checks and installation state for the management window. A Rust preparation command restores repeat quest and checks the exact game process immediately before JavaScript calls `installUpdate`; the canonical PowerShell packager produces and validates every signed updater asset, and a manually dispatched Windows GitHub Actions workflow creates a verified draft before optional publication.

**Tech Stack:** React 18, TypeScript, Vitest, Mantine 7, Tauri 1 updater API, Rust, PowerShell, NSIS, GitHub Actions and GitHub Releases.

## Global Constraints

- Repository, Actions, and Releases remain in the public `azyu/gbfr-djeetamod` repository; `origin` uses SSH and `awa` remains an upstream reference.
- Stable metadata endpoint is exactly `https://github.com/azyu/gbfr-djeetamod/releases/latest/download/latest.json`.
- Windows platform key is exactly `windows-x86_64`; bundles remain NSIS-only with `currentUser` installation and passive updater presentation.
- Update checks never block startup, startup-check failures are non-intrusive, and installation always requires explicit user approval.
- The exact `granblue_fantasy_relink.exe` process check—not frontend connection state—gates installation; Djeeta MOD never terminates the game.
- Repeat quest must restore and read back original instructions before an update can proceed while the game exists; `RunEvent::Exit` restoration remains defense in depth.
- `logs.db`, settings, and window geometry are user data and must not be read, modified, deleted, staged, or committed by this work.
- The updater private key and password are never committed or printed in logs; the public key is committed in Tauri configuration.
- `npm.cmd run package:nsis` remains the sole canonical release build and requires updater signing credentials after updater activation.
- Releases use matching `X.Y.Z` project versions and `vX.Y.Z` tags; an existing tag or published asset is never replaced for rollback.
- Node.js 20 and the pinned `nightly-2024-05-04` Rust toolchain remain required.
- Automated success does not establish Granblue Fantasy: Relink 2.0.2 compatibility; the offline/private manual smoke checklist remains authoritative.

---

## File Structure

- Create `src-tauri/src/update_install.rs`: exact process/read-back preparation command and pure readiness decision tests.
- Modify `src-tauri/src/repeat_quest.rs`: expose one crate-visible update restoration entry point that reuses the existing serialized patch transaction.
- Modify `src-tauri/src/main.rs`: register the update preparation module and Tauri command.
- Create `src/pages/useUpdater.tsx`: one provider/context for startup checks, manual checks, duplicate suppression, preparation, and install state.
- Create `src/components/UpdaterDialog.tsx`: confirmation, release notes, preparation/install progress, and actionable failure modal.
- Create `src/components/UpdaterSettings.tsx`: Settings-page version/status display and manual check action.
- Modify `src/pages/Logs.tsx`: mount the provider once for the management window and render the shared dialog.
- Modify `src/pages/Settings.tsx`: render the updater settings component without mixing update state into persistent meter settings.
- Create `src/pages/useUpdater.test.tsx`, `src/components/UpdaterSettings.test.tsx`, and `src/components/UpdaterDialog.test.tsx`: state-machine and UI regressions.
- Modify `src-tauri/lang/ko/ui.json`, `src-tauri/lang/en/ui.json`, and `src/pages/Settings.localization.test.ts`: Korean/English updater copy and key coverage.
- Modify `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, and `src/securityConfiguration.test.ts`: signed updater dependency/configuration contract.
- Modify `scripts/PackageHelpers.psm1`, `scripts/tests/PackageHelpers.Tests.ps1`, and `scripts/package.ps1`: version agreement, signed updater artifact selection, manifest generation, and machine-readable packaging summary.
- Create `src/releaseWorkflow.test.ts` and `.github/workflows/release.yaml`: static workflow safeguards and Windows draft-release publication.
- Modify `README.md` and `docs/testing/game-2.0.2-smoke-test.md`: bootstrap limitation, user flow, release assets, and manual acceptance evidence.

---

### Task 1: Provision the Djeeta MOD signing identity and enable the updater contract

**Files:**
- Modify: `src/securityConfiguration.test.ts`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `Cargo.lock`
- Create outside repository: `C:\Users\azyu\.djeeta-mod\updater.key`
- Create outside repository: `C:\Users\azyu\.djeeta-mod\updater.key.pub`

**Interfaces:**
- Consumes: the approved GitHub endpoint and Tauri 1 signer CLI already installed in `node_modules`.
- Produces: one committed Tauri updater public key and protected `TAURI_PRIVATE_KEY` / `TAURI_KEY_PASSWORD` values for local release builds and the GitHub `production` environment.

- [ ] **Step 1: Generate the key pair outside the repository**

Run from the repository and enter a unique password at the prompt:

```powershell
New-Item -ItemType Directory -Force -Path 'C:\Users\azyu\.djeeta-mod'
npm.cmd run tauri -- signer generate -w 'C:\Users\azyu\.djeeta-mod\updater.key'
```

Expected: the CLI reports a minisign public key; the private key and companion public-key file exist only under `C:\Users\azyu\.djeeta-mod`. Record the password in the user's password manager and make an encrypted offline backup before continuing. Do not display the private key in terminal output.

- [ ] **Step 2: Write the failing updater configuration regression**

Add this test to `src/securityConfiguration.test.ts`, retaining the existing NSIS assertions:

```ts
it("enables only the signed stable GitHub updater", () => {
  const config = JSON.parse(readRepositoryFile("src-tauri/tauri.conf.json")) as {
    tauri: {
      updater: {
        active: boolean;
        dialog: boolean;
        pubkey: string;
        endpoints: string[];
        windows: { installMode: string };
      };
    };
  };
  const cargo = readRepositoryFile("src-tauri/Cargo.toml");
  const inheritedUpstreamKey =
    "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IERDQjZEMTgxOEY4OTIwNDcKUldSSElJbVBnZEcyM1BSUklxWWRsWStXYnVsWU1mODY3YzZCWCtTZzJrUGJsZHpNY1h1S3hhc2cK";

  expect(config.tauri.updater).toMatchObject({
    active: true,
    dialog: false,
    endpoints: ["https://github.com/azyu/gbfr-djeetamod/releases/latest/download/latest.json"],
    windows: { installMode: "passive" },
  });
  expect(config.tauri.updater.pubkey).not.toBe(inheritedUpstreamKey);
  expect(config.tauri.updater.pubkey.length).toBeGreaterThan(40);
  expect(cargo).toMatch(/tauri = \{[^\n]*features = \[[^\]]*"updater"/);
});
```

- [ ] **Step 3: Run the focused test to verify RED**

Run:

```powershell
npm.cmd test -- --run src/securityConfiguration.test.ts
```

Expected: FAIL because updater is inactive, uses the upstream endpoint/key, uses `basicUi`, and Cargo lacks the updater feature.

- [ ] **Step 4: Apply the minimum updater configuration**

Add `"updater"` to the existing Tauri feature array in `src-tauri/Cargo.toml`. In `src-tauri/tauri.conf.json`, preserve every other bundle setting and set:

```json
"updater": {
  "active": true,
  "dialog": false,
  "pubkey": "the exact base64 public key printed by the signer command in Step 1",
  "endpoints": [
    "https://github.com/azyu/gbfr-djeetamod/releases/latest/download/latest.json"
  ],
  "windows": {
    "installMode": "passive"
  }
}
```

The quoted `pubkey` value must be the actual generated public-key output, not the explanatory sentence shown above. Run `cargo metadata --no-deps --format-version 1 | Out-Null` once after editing so Cargo may update the lockfile, then use `--locked` for verification.

- [ ] **Step 5: Run the focused test to verify GREEN**

Run:

```powershell
npm.cmd test -- --run src/securityConfiguration.test.ts
cargo metadata --locked --no-deps --format-version 1 | Out-Null
git diff --check
```

Expected: all commands exit 0; no private-key material appears in `git status` or `git diff`.

- [ ] **Step 6: Commit the signed updater contract**

```powershell
git add src/securityConfiguration.test.ts src-tauri/Cargo.toml src-tauri/tauri.conf.json Cargo.lock
git commit -m "feat: configure signed GitHub updates"
```

---

### Task 2: Gate installation on exact process state and repeat-quest restoration

**Files:**
- Create: `src-tauri/src/update_install.rs`
- Modify: `src-tauri/src/repeat_quest.rs`
- Modify: `src-tauri/src/main.rs`

**Interfaces:**
- Consumes: `equipment_probe::GAME_PROCESS_NAME` and `RepeatQuestState::set_enabled(false)` semantics.
- Produces: Tauri command `prepare_update_install() -> UpdateInstallReadiness`, serialized as `"ready"`, `"gameRunning"`, or `"repeatQuestRestoreFailed"`.

- [ ] **Step 1: Add failing pure readiness tests**

Create `src-tauri/src/update_install.rs` with the result enum and tests first:

```rust
use crate::repeat_quest::{RepeatQuestStatus, RepeatQuestStatusKind};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum UpdateInstallReadiness {
    Ready,
    GameRunning,
    RepeatQuestRestoreFailed,
}

fn decide_readiness(
    restore_status: Option<RepeatQuestStatus>,
    running_after_restore: bool,
) -> UpdateInstallReadiness {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repeat_quest::{RepeatQuestReason, RepeatQuestStatusKind};

    fn status(state: RepeatQuestStatusKind, reason: Option<RepeatQuestReason>) -> RepeatQuestStatus {
        RepeatQuestStatus { state, reason }
    }

    #[test]
    fn stopped_game_is_ready_without_restoration() {
        assert_eq!(decide_readiness(None, false), UpdateInstallReadiness::Ready);
    }

    #[test]
    fn restored_running_game_stays_blocked_until_closed() {
        assert_eq!(
            decide_readiness(Some(status(RepeatQuestStatusKind::Off, None)), true),
            UpdateInstallReadiness::GameRunning
        );
    }

    #[test]
    fn restoration_failure_blocks_a_still_running_game() {
        assert_eq!(
            decide_readiness(
                Some(status(
                    RepeatQuestStatusKind::Unavailable,
                    Some(RepeatQuestReason::RestoreFailed)
                )),
                true
            ),
            UpdateInstallReadiness::RepeatQuestRestoreFailed
        );
    }

    #[test]
    fn process_exit_during_restoration_is_ready() {
        assert_eq!(
            decide_readiness(
                Some(status(
                    RepeatQuestStatusKind::Unavailable,
                    Some(RepeatQuestReason::GameNotRunning)
                )),
                false
            ),
            UpdateInstallReadiness::Ready
        );
    }
}
```

- [ ] **Step 2: Register the module only and verify RED**

Add `mod update_install;` to `src-tauri/src/main.rs`, then run:

```powershell
cargo test --locked --package gbfr-logs update_install::tests
```

Expected: FAIL because `decide_readiness` is unimplemented.

- [ ] **Step 3: Implement the pure decision and reusable restoration entry point**

Implement the decision in `update_install.rs`:

```rust
fn decide_readiness(
    restore_status: Option<RepeatQuestStatus>,
    running_after_restore: bool,
) -> UpdateInstallReadiness {
    if !running_after_restore {
        return UpdateInstallReadiness::Ready;
    }
    match restore_status {
        Some(RepeatQuestStatus {
            state: RepeatQuestStatusKind::Off,
            ..
        }) => UpdateInstallReadiness::GameRunning,
        _ => UpdateInstallReadiness::RepeatQuestRestoreFailed,
    }
}
```

Add only this method to `impl RepeatQuestState` in `src-tauri/src/repeat_quest.rs`:

```rust
pub(crate) fn restore_for_update(&self) -> RepeatQuestStatus {
    self.set_enabled(false)
}
```

Do not call `restore_on_exit()` here because it returns no error and latches `cleanup_started`.

- [ ] **Step 4: Add the exact-process Tauri command**

Complete `src-tauri/src/update_install.rs` with imports and command:

```rust
use crate::{
    equipment_probe::GAME_PROCESS_NAME,
    repeat_quest::{RepeatQuestState, RepeatQuestStatus, RepeatQuestStatusKind},
};
use dll_syringe::process::{OwnedProcess, Process};

fn game_is_running() -> bool {
    OwnedProcess::find_first_by_name(GAME_PROCESS_NAME).is_some()
}

#[tauri::command]
pub(crate) async fn prepare_update_install(
    state: tauri::State<'_, RepeatQuestState>,
) -> UpdateInstallReadiness {
    // The second lookup closes the first-check/startup race before restoration.
    if !game_is_running() && !game_is_running() {
        return UpdateInstallReadiness::Ready;
    }

    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let restored = state.restore_for_update();
        decide_readiness(Some(restored), game_is_running())
    })
    .await
    .unwrap_or(UpdateInstallReadiness::RepeatQuestRestoreFailed)
}
```

Register `update_install::prepare_update_install` in `tauri::generate_handler!` in `src-tauri/src/main.rs`. Do not consult `ConnectionStatus` and do not terminate the game.

Add a repeat-quest regression using the existing fake backend: call `restore_for_update()`, enable again, then call `restore_on_exit()` and assert the final restoration still occurs. This proves update preparation did not latch `cleanup_started` when the user cancels an update.

- [ ] **Step 5: Run focused Rust tests and existing repeat-quest regressions**

Run:

```powershell
cargo test --locked --package gbfr-logs update_install::tests
cargo test --locked --package gbfr-logs repeat_quest::tests
```

Expected: PASS, including startup/exit restoration and busy-operation tests.

- [ ] **Step 6: Commit the installation gate**

```powershell
git add src-tauri/src/update_install.rs src-tauri/src/repeat_quest.rs src-tauri/src/main.rs
git commit -m "feat: prepare safely for update installation"
```

---

### Task 3: Build one shared updater state provider

**Files:**
- Create: `src/pages/useUpdater.tsx`
- Create: `src/pages/useUpdater.test.tsx`
- Modify: `src/pages/Logs.tsx`
- Modify: `src/pages/Logs.test.tsx`
- Modify: `src/pages/Logs.repeatQuest.test.tsx`

**Interfaces:**
- Consumes: Tauri `checkUpdate`, `installUpdate`, `getVersion`, and backend `prepare_update_install`.
- Produces: `UpdaterProvider`, `useUpdater()`, `UpdaterState`, `checkForUpdate("automatic" | "manual")`, `installAvailableUpdate()`, and `dismissUpdate()`.

- [ ] **Step 1: Write failing provider state tests**

Create `src/pages/useUpdater.test.tsx` using `renderHook`, `act`, `waitFor`, a wrapper containing `UpdaterProvider`, and hoisted mocks for `@tauri-apps/api/updater`, `@tauri-apps/api/app`, and `@tauri-apps/api/tauri`. Cover these exact cases:

```ts
it("checks once on provider mount and stays idle when the automatic check fails", async () => {});
it("reports upToDate after a successful manual check", async () => {});
it("retains the manifest when a newer version is available", async () => {});
it("shares one promise for concurrent checks", async () => {});
it("restores repeat quest and blocks install while the game is running", async () => {});
it("calls installUpdate only after backend readiness is ready", async () => {});
it("reports repeatQuestRestoreFailed without calling installUpdate", async () => {});
it("reports installFailed when installUpdate rejects", async () => {});
```

Use this observable controller contract in assertions:

```ts
export type UpdaterPhase =
  | "idle"
  | "checking"
  | "upToDate"
  | "available"
  | "preparing"
  | "installing"
  | "error";

export type UpdaterError = "checkFailed" | "gameRunning" | "repeatQuestRestoreFailed" | "installFailed";

export type UpdaterState = {
  phase: UpdaterPhase;
  currentVersion: string;
  manifest: UpdateManifest | null;
  error: UpdaterError | null;
};
```

- [ ] **Step 2: Run the provider tests to verify RED**

Run:

```powershell
npm.cmd test -- --run src/pages/useUpdater.test.tsx
```

Expected: FAIL because `UpdaterProvider` and `useUpdater` do not exist.

- [ ] **Step 3: Implement the provider and controller**

Create `src/pages/useUpdater.tsx`. The core public contract must be:

```tsx
type CheckMode = "automatic" | "manual";
type UpdateInstallReadiness = "ready" | "gameRunning" | "repeatQuestRestoreFailed";

export type UpdaterController = {
  state: UpdaterState;
  checkForUpdate: (mode: CheckMode) => Promise<void>;
  installAvailableUpdate: () => Promise<void>;
  dismissUpdate: () => void;
};

const UpdaterContext = createContext<UpdaterController | null>(null);

export const useUpdater = () => {
  const value = useContext(UpdaterContext);
  if (!value) throw new Error("useUpdater must be used inside UpdaterProvider");
  return value;
};
```

Implement one `checkPromiseRef` guard so concurrent calls return the same in-flight promise. On provider mount call `checkForUpdate("automatic")` once. Automatic failures log `console.warn` and return to `idle`; manual failures set `{ phase: "error", error: "checkFailed" }`. Preserve `result.manifest` only when `shouldUpdate` is true.

`installAvailableUpdate()` must set `preparing`, invoke `prepare_update_install`, and branch exactly as follows:

```ts
const readiness = await invoke<UpdateInstallReadiness>("prepare_update_install");
if (readiness !== "ready") {
  setState((current) => ({ ...current, phase: "error", error: readiness }));
  return;
}
setState((current) => ({ ...current, phase: "installing", error: null }));
await installUpdate();
```

Map a rejected install to `installFailed`. Do not call `relaunch`; Tauri's Windows updater controls restart. Use a disposed ref to prevent late async state writes after unmount.

Allow `installAvailableUpdate()` whenever a manifest is retained, including a retry after a blocking error. `dismissUpdate()` returns to `idle` and clears the manifest/error so **나중에** does not reopen the same dialog during that process lifetime.

- [ ] **Step 4: Mount the provider once for the management window**

In `src/pages/Logs.tsx`, keep existing layout behavior in an inner component and export one wrapper:

```tsx
const Layout = () => (
  <UpdaterProvider>
    <LayoutContent />
  </UpdaterProvider>
);
```

The provider must not wrap the compact meter route, and `Settings.tsx` must not create a second updater instance.

In `Logs.test.tsx` and `Logs.repeatQuest.test.tsx`, mock `UpdaterProvider` as a pass-through component so those focused layout tests do not issue network/update calls:

```tsx
import type { ReactNode } from "react";

vi.mock("./useUpdater", () => ({
  UpdaterProvider: ({ children }: { children: ReactNode }) => children,
  useUpdater: () => ({
    state: { phase: "idle", currentVersion: "0.1.1", manifest: null, error: null },
    checkForUpdate: vi.fn(),
    installAvailableUpdate: vi.fn(),
    dismissUpdate: vi.fn(),
  }),
}));
```

- [ ] **Step 5: Run provider and existing layout tests**

Run:

```powershell
npm.cmd test -- --run src/pages/useUpdater.test.tsx src/pages/Logs.test.tsx src/pages/Logs.repeatQuest.test.tsx
```

Expected: PASS with exactly one startup check per mounted management layout and no repeat-quest layout regression.

- [ ] **Step 6: Commit shared update state**

```powershell
git add src/pages/useUpdater.tsx src/pages/useUpdater.test.tsx src/pages/Logs.tsx src/pages/Logs.test.tsx src/pages/Logs.repeatQuest.test.tsx
git commit -m "feat: manage update checks in the logs window"
```

---

### Task 4: Add Korean/English update confirmation and Settings UI

**Files:**
- Create: `src/components/UpdaterDialog.tsx`
- Create: `src/components/UpdaterSettings.tsx`
- Create: `src/components/UpdaterSettings.test.tsx`
- Create: `src/components/UpdaterDialog.test.tsx`
- Modify: `src/pages/Logs.tsx`
- Modify: `src/pages/Settings.tsx`
- Modify: `src-tauri/lang/ko/ui.json`
- Modify: `src-tauri/lang/en/ui.json`
- Modify: `src/pages/Settings.localization.test.ts`

**Interfaces:**
- Consumes: `useUpdater()` from Task 3.
- Produces: a non-blocking manual check surface and a user-approved update modal with no raw endpoint or secret data.

- [ ] **Step 1: Add failing localization and component tests**

Extend `Settings.localization.test.ts` with exact `ui.updater` objects:

```ts
const expectedKoreanUpdater = {
  title: "업데이트",
  current: "현재 버전 v{{version}}",
  check: "업데이트 확인",
  checking: "업데이트를 확인하는 중입니다.",
  "up-to-date": "최신 버전을 사용 중입니다.",
  available: "새 버전 v{{version}}을 사용할 수 있습니다.",
  notes: "릴리스 노트",
  later: "나중에",
  install: "업데이트",
  preparing: "업데이트를 준비하는 중입니다.",
  installing: "업데이트를 설치하는 중입니다.",
  "check-failed": "업데이트를 확인하지 못했습니다.",
  "game-running": "게임을 종료한 후 다시 업데이트해 주세요.",
  "repeat-quest-restore-failed": "무한 퀘스트 반복 설정을 복구하지 못해 업데이트를 중단했습니다.",
  "install-failed": "업데이트를 설치하지 못했습니다.",
};
```

Add the corresponding English values and assert `english["updater"]` and `korean["updater"]` equal the expected objects. Create `UpdaterSettings.test.tsx` and mock `useUpdater`; verify current version, manual check click, loading/disabled states, available version, and translated error text. Create `UpdaterDialog.test.tsx` to prove **나중에** calls `dismissUpdate`, **업데이트** calls `installAvailableUpdate` exactly once, release notes render only when non-empty, and `checkFailed` does not open the installation modal.

- [ ] **Step 2: Run the UI tests to verify RED**

Run:

```powershell
npm.cmd test -- --run src/components/UpdaterSettings.test.tsx src/components/UpdaterDialog.test.tsx src/pages/Settings.localization.test.ts
```

Expected: FAIL because the translation keys and components do not exist.

- [ ] **Step 3: Add translation resources**

Add the exact Korean object above as `ui.updater` in `src-tauri/lang/ko/ui.json`. Add the same keys in English:

```json
"updater": {
  "title": "Updates",
  "current": "Current version v{{version}}",
  "check": "Check for updates",
  "checking": "Checking for updates.",
  "up-to-date": "You are using the latest version.",
  "available": "Version v{{version}} is available.",
  "notes": "Release notes",
  "later": "Later",
  "install": "Update",
  "preparing": "Preparing the update.",
  "installing": "Installing the update.",
  "check-failed": "Could not check for updates.",
  "game-running": "Close the game and try the update again.",
  "repeat-quest-restore-failed": "The update was stopped because Unlimited Repeat Quest could not be restored.",
  "install-failed": "Could not install the update."
}
```

- [ ] **Step 4: Implement the Settings surface and modal**

`UpdaterSettings.tsx` renders a dedicated `Fieldset`, current version, phase text, and one check button. It calls `checkForUpdate("manual")`; checking, preparing, and installing disable the button.

`UpdaterDialog.tsx` renders a Mantine `Modal` only for `available`, `preparing`, `installing`, and the blocking `gameRunning`, `repeatQuestRestoreFailed`, and `installFailed` errors. `checkFailed` remains inline in Settings. Preserve the manifest across blocking errors so closing the game and pressing **업데이트** retries preparation. Its action contract is:

```tsx
<Button variant="default" onClick={dismissUpdate} disabled={state.phase === "installing"}>
  {t("ui.updater.later")}
</Button>
<Button onClick={() => void installAvailableUpdate()} loading={state.phase === "preparing" || state.phase === "installing"}>
  {t("ui.updater.install")}
</Button>
```

Show `manifest.body` under the release-notes label only when it is non-empty. Never render the manifest URL, signature, or a raw exception.

Add `<UpdaterSettings />` after the existing meter-settings fieldset in `Settings.tsx`. Render `<UpdaterDialog />` once beside the existing `Toaster` inside the provider in `Logs.tsx`.

- [ ] **Step 5: Run focused UI and layout tests**

Run:

```powershell
npm.cmd test -- --run src/pages/useUpdater.test.tsx src/components/UpdaterSettings.test.tsx src/components/UpdaterDialog.test.tsx src/pages/Settings.localization.test.ts src/pages/Logs.test.tsx src/pages/Logs.repeatQuest.test.tsx
```

Expected: PASS; the startup check remains non-blocking and installation still requires the update button.

- [ ] **Step 6: Commit update UX**

```powershell
git add src/components/UpdaterDialog.tsx src/components/UpdaterDialog.test.tsx src/components/UpdaterSettings.tsx src/components/UpdaterSettings.test.tsx src/pages/Logs.tsx src/pages/Settings.tsx src-tauri/lang/ko/ui.json src-tauri/lang/en/ui.json src/pages/Settings.localization.test.ts
git commit -m "feat: add update controls and confirmation"
```

---

### Task 5: Test and implement release packaging primitives

**Files:**
- Modify: `scripts/PackageHelpers.psm1`
- Modify: `scripts/tests/PackageHelpers.Tests.ps1`

**Interfaces:**
- Consumes: four semantic version strings, fresh NSIS bundle files, a versioned HTTPS asset URL, and `.sig` file contents.
- Produces: `Assert-ReleaseVersionAgreement`, `Assert-UpdaterSigningEnvironment`, `Select-ProductNsisUpdaterArtifacts`, and `New-TauriUpdaterManifest`.

- [ ] **Step 1: Add failing PowerShell helper tests**

Append table-driven cases to `PackageHelpers.Tests.ps1` that require:

```powershell
Assert-Equal (Assert-ReleaseVersionAgreement -RequestedVersion '0.1.2' -PackageVersion '0.1.2' -CargoVersion '0.1.2' -TauriVersion '0.1.2') '0.1.2' 'Matching release versions failed.'
Assert-Throws { Assert-ReleaseVersionAgreement -RequestedVersion '0.1.2' -PackageVersion '0.1.2' -CargoVersion '0.1.1' -TauriVersion '0.1.2' } 'Cargo mismatch must fail.'
Assert-Throws { Assert-ReleaseVersionAgreement -RequestedVersion 'v0.1.2' -PackageVersion '0.1.2' -CargoVersion '0.1.2' -TauriVersion '0.1.2' } 'Prefixed version must fail.'
Assert-Throws { Assert-ReleaseVersionAgreement -RequestedVersion '0.1.2-beta.1' -PackageVersion '0.1.2-beta.1' -CargoVersion '0.1.2-beta.1' -TauriVersion '0.1.2-beta.1' } 'Prerelease must fail.'
```

Create fresh archive/signature fake artifacts and assert one matching pair is returned; missing, duplicate, stale, wrong-product, and wrong-version pairs must throw. Generate a manifest and assert:

```powershell
$manifest = New-TauriUpdaterManifest -Version '0.1.2' -Notes 'Release notes' -PublishedAt ([datetime]'2026-07-22T00:00:00Z') -ArchiveUrl 'https://github.com/azyu/gbfr-djeetamod/releases/download/v0.1.2/Djeeta%20MOD_0.1.2_x64-setup.nsis.zip' -Signature 'signed-content'
$parsed = $manifest | ConvertFrom-Json
Assert-Equal $parsed.version '0.1.2' 'Manifest version failed.'
Assert-Equal $parsed.platforms.'windows-x86_64'.signature 'signed-content' 'Manifest signature failed.'
Assert-Equal $parsed.platforms.'windows-x86_64'.url 'https://github.com/azyu/gbfr-djeetamod/releases/download/v0.1.2/Djeeta%20MOD_0.1.2_x64-setup.nsis.zip' 'Manifest URL failed.'
```

Empty signatures, non-HTTPS URLs, and URLs whose tag does not match `Version` must throw. Test `Assert-UpdaterSigningEnvironment` with a fake environment map rather than reading real secrets.

- [ ] **Step 2: Run helper tests to verify RED**

Run:

```powershell
npm.cmd run test:package-helpers
```

Expected: FAIL because the four helper functions are undefined.

- [ ] **Step 3: Implement the four pure helpers**

Implement strict `X.Y.Z` agreement, non-empty signing values, exactly one fresh archive/signature basename pair, and JSON generation. The manifest object must have this exact shape before `ConvertTo-Json -Depth 5`:

```powershell
[ordered]@{
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
```

Return an object with `Archive` and `Signature` from `Select-ProductNsisUpdaterArtifacts`. Export all four functions explicitly in `Export-ModuleMember`. Do not loosen the existing installer selection or hash replacement checks.

- [ ] **Step 4: Run helper tests to verify GREEN**

Run:

```powershell
npm.cmd run test:package-helpers
git diff --check
```

Expected: `Package helper tests passed.` and exit 0.

- [ ] **Step 5: Commit packaging primitives**

```powershell
git add scripts/PackageHelpers.psm1 scripts/tests/PackageHelpers.Tests.ps1
git commit -m "feat: validate signed updater artifacts"
```

---

### Task 6: Extend the canonical NSIS packager with updater outputs

**Files:**
- Modify: `scripts/package.ps1`
- Modify: `scripts/tests/PackageHelpers.Tests.ps1`
- Modify generated during release: `README.md`
- Modify generated during release: `docs/testing/game-2.0.2-smoke-test.md`
- Generate ignored artifact: `target/release/latest.json`
- Generate ignored artifact: `target/release/package-summary.json`

**Interfaces:**
- Consumes: optional `-RequestedVersion`, optional `-ReleaseNotesPath`, `TAURI_PRIVATE_KEY`, `TAURI_KEY_PASSWORD`, and Task 5 helpers.
- Produces: verified installer, updater ZIP, `.sig`, `latest.json`, hash-document changes, and machine-readable `package-summary.json`.

- [ ] **Step 1: Add a failing static packaging regression**

Extend the existing `exposes only the verified NSIS packaging command` test in `src/securityConfiguration.test.ts` to require these exact strings from `scripts/package.ps1`:

```ts
expect(packagingScript).toContain("Assert-UpdaterSigningEnvironment");
expect(packagingScript).toContain("Select-ProductNsisUpdaterArtifacts");
expect(packagingScript).toContain("New-TauriUpdaterManifest");
expect(packagingScript).toContain("package-summary.json");
```

- [ ] **Step 2: Run the packaging contract test to verify RED**

Run:

```powershell
npm.cmd test -- --run src/securityConfiguration.test.ts
```

Expected: FAIL because `package.ps1` does not handle updater outputs.

- [ ] **Step 3: Add version and signing preflight before expensive work**

Add these parameters at the top of `package.ps1`:

```powershell
param(
    [string]$RequestedVersion,
    [string]$ReleaseNotesPath
)
```

Read `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`, then call `Assert-ReleaseVersionAgreement`. If `RequestedVersion` is empty, pass the package version so local canonical builds still require internal agreement. Call:

```powershell
Assert-UpdaterSigningEnvironment -Values @{
    TAURI_PRIVATE_KEY = $env:TAURI_PRIVATE_KEY
    TAURI_KEY_PASSWORD = $env:TAURI_KEY_PASSWORD
}
```

before `npm ci`. A canonical package without both signing values must fail immediately with no build or document mutation. When `ReleaseNotesPath` is supplied, require one existing file and read it as UTF-8; otherwise use an empty notes string.

- [ ] **Step 4: Select, hash, and describe signed updater artifacts**

After the one Tauri NSIS build, select the normal installer as before and pass every `*.nsis.zip*` artifact to `Select-ProductNsisUpdaterArtifacts`. Read the signature with UTF-8, URL-encode the actual archive filename with `[uri]::EscapeDataString`, construct the versioned URL, and write `target/release/latest.json` with UTF-8 and no BOM.

Write `target/release/package-summary.json` with this exact property contract:

```powershell
[ordered]@{
    Version = $productVersion
    InstallerPath = $installer.FullName
    InstallerSHA256 = $installerHash
    HookPath = $releaseHookPath
    HookSHA256 = $releaseHookHash
    UpdaterArchivePath = $updater.Archive.FullName
    UpdaterArchiveSHA256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $updater.Archive.FullName).Hash
    UpdaterSignaturePath = $updater.Signature.FullName
    LatestJsonPath = $latestJsonPath
}
```

Preserve the existing README/smoke-test hash replacement, hook equality check, `git diff --check`, and final human-readable output.

- [ ] **Step 5: Run focused tests without invoking a release build**

Run:

```powershell
npm.cmd run test:package-helpers
npm.cmd test -- --run src/securityConfiguration.test.ts
```

Expected: PASS. Do not run `package:nsis` until the local signing environment is loaded.

- [ ] **Step 6: Run one signed canonical package and inspect outputs**

Load `TAURI_PRIVATE_KEY` from `C:\Users\azyu\.djeeta-mod\updater.key` without echoing it and set `TAURI_KEY_PASSWORD` through a masked interactive entry:

```powershell
$env:TAURI_PRIVATE_KEY = [IO.File]::ReadAllText('C:\Users\azyu\.djeeta-mod\updater.key')
$securePassword = Read-Host 'Updater key password' -AsSecureString
$passwordPointer = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($securePassword)
try {
    $env:TAURI_KEY_PASSWORD = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($passwordPointer)
    npm.cmd run package:nsis -- -RequestedVersion 0.1.1
}
finally {
    [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($passwordPointer)
    Remove-Item Env:TAURI_PRIVATE_KEY -ErrorAction SilentlyContinue
    Remove-Item Env:TAURI_KEY_PASSWORD -ErrorAction SilentlyContinue
}
```

Expected: one fresh setup EXE, one `.nsis.zip`, one matching `.sig`, `target/release/latest.json`, and `target/release/package-summary.json`; release/bundled hook hashes match; README and smoke-test installer/hook hashes are updated together.

- [ ] **Step 7: Commit the canonical packager and generated hash documents together**

```powershell
git add scripts/package.ps1 scripts/tests/PackageHelpers.Tests.ps1 src/securityConfiguration.test.ts README.md docs/testing/game-2.0.2-smoke-test.md
git commit -m "feat: package signed update releases"
```

Do not add files under `target/`, private keys, passwords, or `logs.db`.

---

### Task 7: Add a guarded GitHub Release workflow

**Files:**
- Create: `src/releaseWorkflow.test.ts`
- Create: `.github/workflows/release.yaml`

**Interfaces:**
- Consumes: manual `version` and `publish` inputs, production signing secrets, canonical package summary, and repository `GITHUB_TOKEN` with `contents: write`.
- Produces: one `vX.Y.Z` tag, one verified GitHub draft containing exactly the four required assets, and an optional stable publication.

- [ ] **Step 1: Write the failing static workflow safeguards**

Create `src/releaseWorkflow.test.ts` to read `.github/workflows/release.yaml` and assert it contains:

```ts
expect(workflow).toContain("workflow_dispatch:");
expect(workflow).toContain("contents: write");
expect(workflow).toContain("environment: production");
expect(workflow).toContain("TAURI_PRIVATE_KEY");
expect(workflow).toContain("TAURI_KEY_PASSWORD");
expect(workflow).toContain("npm.cmd run package:nsis -- -RequestedVersion");
expect(workflow).toContain("gh release create");
expect(workflow).toContain("--draft");
expect(workflow).toContain("latest.json");
expect(workflow).toContain("gh release edit");
expect(workflow).not.toMatch(/uses:\s+[^\s]+@(v\d+|main|master)\b/);
```

- [ ] **Step 2: Run the workflow test to verify RED**

Run:

```powershell
npm.cmd test -- --run src/releaseWorkflow.test.ts
```

Expected: FAIL because the release workflow does not exist.

- [ ] **Step 3: Create the manual Windows workflow with pinned actions**

Create `.github/workflows/release.yaml` with these top-level controls:

```yaml
name: Release
on:
  workflow_dispatch:
    inputs:
      version:
        description: Exact project version without the v prefix
        required: true
        type: string
      publish:
        description: Publish after verification instead of leaving a draft
        required: true
        default: false
        type: boolean

concurrency:
  group: djeeta-mod-release
  cancel-in-progress: false

jobs:
  release:
    runs-on: windows-latest
    environment: production
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4
        with:
          ref: main
          fetch-depth: 0
      - uses: actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020 # v4
        with:
          node-version: 20
          cache: npm
```

Set job environment values without printing them:

```yaml
    env:
      GH_TOKEN: ${{ github.token }}
      TAURI_PRIVATE_KEY: ${{ secrets.TAURI_PRIVATE_KEY }}
      TAURI_KEY_PASSWORD: ${{ secrets.TAURI_KEY_PASSWORD }}
```

- [ ] **Step 4: Implement build, release commit, tag, and draft creation**

The PowerShell steps must perform this exact order:

```powershell
$version = '${{ inputs.version }}'
$tag = "v$version"
git rev-parse "refs/tags/$tag" 2>$null
if ($LASTEXITCODE -eq 0) { throw "Tag already exists: $tag" }
gh release view $tag 2>$null
if ($LASTEXITCODE -eq 0) { throw "Release already exists: $tag" }
$releaseNotesPath = Join-Path $env:RUNNER_TEMP 'djeeta-release-notes.md'
gh api --method POST "repos/${{ github.repository }}/releases/generate-notes" -f tag_name=$tag -f target_commitish=main --jq .body | Set-Content -LiteralPath $releaseNotesPath -Encoding utf8
npm.cmd run package:nsis -- -RequestedVersion $version -ReleaseNotesPath $releaseNotesPath
```

Require `git diff --name-only` to contain only `README.md` and `docs/testing/game-2.0.2-smoke-test.md`, require both together, configure `github-actions[bot]`, commit them, push `HEAD:main`, create annotated `vX.Y.Z`, and push the tag. Read `target/release/package-summary.json`, then create a draft with generated notes and upload:

```powershell
gh release create $tag --draft --verify-tag --generate-notes --title "Djeeta MOD $tag"
gh release upload $tag $summary.InstallerPath $summary.UpdaterArchivePath $summary.UpdaterSignaturePath $summary.LatestJsonPath
```

Any failure after draft creation leaves it unpublished for manual inspection. The workflow never deletes or overwrites an existing tag/release.

- [ ] **Step 5: Verify remote assets before optional publication**

Query the draft with `gh api repos/${{ github.repository }}/releases/tags/$tag`. Require exactly the four uploaded asset names, compare every available GitHub `sha256:` digest with the corresponding local SHA-256, download the remote `latest.json`, and assert its `windows-x86_64.url` and signature equal the local archive URL and `.sig` contents.

Only after all assertions pass, publish conditionally:

```powershell
if ('${{ inputs.publish }}' -eq 'true') {
    gh release edit $tag --draft=false --latest
}
```

- [ ] **Step 6: Run the static workflow and full frontend checks**

Run:

```powershell
npm.cmd test -- --run src/releaseWorkflow.test.ts src/securityConfiguration.test.ts
npm.cmd run format-check
npm.cmd run lint
npm.cmd run tsc
npm.cmd test -- --run
npm.cmd run build
```

Expected: every command exits 0.

- [ ] **Step 7: Commit the release workflow**

```powershell
git add .github/workflows/release.yaml src/releaseWorkflow.test.ts
git commit -m "ci: publish verified signed releases"
```

---

### Task 8: Document bootstrap, release operation, and manual update acceptance

**Files:**
- Modify: `README.md`
- Modify: `docs/testing/game-2.0.2-smoke-test.md`

**Interfaces:**
- Consumes: updater UX, release asset contract, and `0.1.1 -> 0.1.2` acceptance path.
- Produces: Korean/English user guidance and a reproducible maintainer smoke checklist.

- [ ] **Step 1: Add the exact user-facing update contract to README**

Document in Korean and English:

```text
Djeeta MOD checks GitHub Releases once when the management window starts and can also check from Settings. It never installs without confirmation. Close Granblue Fantasy: Relink before installation; if Unlimited Repeat Quest is enabled, Djeeta MOD restores the original instructions before allowing the update.
```

State that updater-inactive `0.1.1` installations require one manual installer run and that rebuilding the same version cannot trigger an update.

- [ ] **Step 2: Extend the smoke checklist with concrete evidence fields**

Add unchecked entries for:

```markdown
- [ ] Updater-enabled `0.1.1` detects stable `0.1.2` and shows its version/notes.
- [ ] **나중에** leaves `0.1.1` running and data unchanged.
- [ ] Offline, missing-manifest, and invalid-signature failures do not block the meter.
- [ ] Installation is refused while `granblue_fantasy_relink.exe` remains running.
- [ ] With repeat quest ON, update preparation reads both sites back as original before prompting for game exit.
- [ ] After installation, the app restarts as `0.1.2`; logs, settings, and window geometry remain present.
- [ ] GitHub Release contains the installer, `.nsis.zip`, `.sig`, and `latest.json` whose URL/signature agree.
```

Keep the existing installer/hook SHA-256 labels intact so `package.ps1` can still update exactly one of each.

- [ ] **Step 3: Verify documentation structure and commit**

Run:

```powershell
npm.cmd run test:package-helpers
git diff --check
```

Expected: PASS and no duplicate hash labels.

```powershell
git add README.md docs/testing/game-2.0.2-smoke-test.md
git commit -m "docs: explain signed automatic updates"
```

---

### Task 9: Complete automated verification and stage the first GitHub draft

**Files:**
- Verify only: all implementation files above
- External configuration: GitHub `production` environment in `azyu/gbfr-djeetamod`
- External artifact: draft release `v0.1.1`

**Interfaces:**
- Consumes: protected signing values and the release workflow.
- Produces: green local verification, pushed implementation commits, and a verified but unpublished updater-enabled `v0.1.1` draft for manual installation.

- [ ] **Step 1: Configure protected GitHub signing secrets**

In repository Settings, set **Actions → General → Workflow permissions** to **Read and write permissions**. Create the `production` environment, add an environment protection reviewer, and add exactly `TAURI_PRIVATE_KEY` and `TAURI_KEY_PASSWORD`. Paste the private-key file contents and its password directly into GitHub's secret forms; do not paste them into chat, a commit, a command argument, or workflow logs.

- [ ] **Step 2: Run the required local verification from a clean signing environment**

Run the narrow checks first, then the full project contract:

```powershell
npm.cmd run test:package-helpers
npm.cmd test -- --run src/securityConfiguration.test.ts src/releaseWorkflow.test.ts src/pages/useUpdater.test.tsx src/components/UpdaterSettings.test.tsx src/components/UpdaterDialog.test.tsx src/pages/Settings.localization.test.ts
npm.cmd run format-check
npm.cmd run lint
npm.cmd run tsc
npm.cmd test -- --run
npm.cmd run build
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
```

Expected: every command exits 0. Treat any formatting, type, frontend, Rust, repeat-quest, or packaging-helper failure as blocking.

- [ ] **Step 3: Run and independently verify the signed canonical package**

Load the private key and masked password for the lifetime of this command only:

```powershell
$env:TAURI_PRIVATE_KEY = [IO.File]::ReadAllText('C:\Users\azyu\.djeeta-mod\updater.key')
$securePassword = Read-Host 'Updater key password' -AsSecureString
$passwordPointer = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($securePassword)
try {
    $env:TAURI_KEY_PASSWORD = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($passwordPointer)
    npm.cmd run package:nsis -- -RequestedVersion 0.1.1
}
finally {
    [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($passwordPointer)
    Remove-Item Env:TAURI_PRIVATE_KEY -ErrorAction SilentlyContinue
    Remove-Item Env:TAURI_KEY_PASSWORD -ErrorAction SilentlyContinue
}
```

Independently verify one installer, one updater ZIP, one `.sig`, valid `latest.json`, equal `target/release/hook.dll` and `src-tauri/hook.dll` SHA-256 values, and matching installer/hook hashes in both README and the smoke-test document. Commit only the two generated hash-document changes together if their values changed.

- [ ] **Step 4: Push implementation commits and run a draft-only workflow**

```powershell
git push origin main
```

From GitHub Actions, run **Release** with `version = 0.1.1` and `publish = false`, approve the `production` environment, then verify the workflow finishes successfully and `v0.1.1` remains a draft. Download and manually install this draft once on the test machine; a previously distributed updater-inactive `0.1.1` cannot discover it.

- [ ] **Step 5: Stop before the first stable publication**

Report the draft URL, local and remote asset hashes, automated verification results, and remaining manual `0.1.1 -> 0.1.2` checklist items. Publishing `v0.1.1`, bumping all three project versions to `0.1.2`, and publishing the first live update are separate user-visible release actions and require explicit user confirmation.

---

## Plan Self-Review Checklist

- Every design section maps to a task: repository/release flow (Tasks 5-9), updater signing/config (Task 1), UI (Tasks 3-4), game/repeat-quest safety (Task 2), bootstrap/version policy (Tasks 5-9), failure handling (Tasks 2-7), and acceptance (Tasks 8-9).
- No task reads or mutates `logs.db`; no task terminates the game or uses `ConnectionState` as an install gate.
- Frontend readiness strings exactly match Rust `#[serde(rename_all = "camelCase")]` output.
- The provider is mounted once in `Logs.tsx`; Settings consumes it and does not issue a duplicate startup check.
- The package summary property names used by the workflow exactly match those produced by `package.ps1`.
- Release action references use the resolved full SHAs for `actions/checkout@v4` and `actions/setup-node@v4`.
- The plan stops at a draft and does not infer permission to publish a stable binary or create `0.1.2`.

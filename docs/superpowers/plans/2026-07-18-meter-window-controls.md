# Meter Window Controls Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep the draggable damage meter above the game but out of the taskbar, and let users persistently show or hide it from a switch in the Djeeta MOD management window.

**Architecture:** The existing `main` Tauri window remains the compact meter and receives an explicit `set_meter_enabled(enabled)` command. The existing `logs` window becomes the single taskbar-visible management window; its Zustand persisted settings own the user's meter visibility choice and its navigation renders the switch. The meter's always-on-top policy is fixed, while click-through starts disabled so its existing drag header works.

**Tech Stack:** Tauri 1/Rust, React 18, TypeScript, Mantine, Zustand persist, Vitest/Testing Library, Cargo tests

## Global Constraints

- Work in the current `codex/trait-overflow-analysis` branch without a worktree, as explicitly requested by the user.
- The meter window is always-on-top and hidden from the Windows taskbar.
- The management window is titled `Djeeta MOD` and remains taskbar-visible.
- Meter visibility defaults to ON and persists across application restarts.
- Click-through defaults to OFF; the existing tray toggle remains available after positioning.
- Remove the tray always-on-top toggle because always-on-top is a meter invariant.
- Preserve the existing battle-record functionality under the Korean label `전투 기록`.
- Do not stage or delete the runtime-generated `logs.db` file.
- Finish with format, lint, TypeScript, frontend test/build, Rust test/build, DLL hash, and manual game checks.

---

## File Structure

- `protocol/src/lib.rs`: give equipment source fields frontend-safe JSON names without changing bincode order.
- `src-tauri/src/equipment/mod.rs`: assert every nested source field is camelCase in the JSON response.
- `src/pages/EquipmentAnalysis.tsx`: tolerate one stale snake_case response so the management window cannot blank during backend hot reload.
- `src/pages/EquipmentAnalysis.test.tsx`: reproduce and prevent the stale-response blank screen.
- `src-tauri/tauri.conf.json`: declare fixed window policy and titles.
- `src-tauri/src/main.rs`: expose explicit meter visibility control, remove mutable always-on-top state, and default click-through to OFF.
- `src/stores/useMeterSettingsStore.ts`: persist `meter_enabled` with a default value of `true`.
- `src/pages/useMeterVisibility.ts`: synchronize the persisted visibility choice with the Tauri window and update it only after successful commands.
- `src/pages/useMeterVisibility.test.tsx`: test startup sync, successful changes, and command failures.
- `src/pages/Logs.tsx`: render the meter switch and renamed navigation items.
- `src/pages/Logs.test.tsx`: test navigation labels and switch interaction.
- `src/pages/Settings.localization.test.ts`: require Korean and English navigation labels and prevent hardcoded menu copy.
- `src-tauri/lang/ko/ui.json`, `src-tauri/lang/en/ui.json`: define menu text.
- `src/components/compact-meter/CompactDamageMeter.test.tsx`: prove an empty/waiting meter still renders its draggable header.
- `docs/testing/game-2.0.2-smoke-test.md`: record manual window checks without claiming full compatibility.

---

### Task 1: Complete the equipment-response blank-screen fix

**Files:**
- Modify: `protocol/src/lib.rs`
- Modify: `src-tauri/src/equipment/mod.rs`
- Modify: `src/pages/EquipmentAnalysis.tsx`
- Modify: `src/pages/EquipmentAnalysis.test.tsx`

**Interfaces:**
- Consumes: `protocol::EquippedTraitSource` and `EquipmentAnalysisResponse`
- Produces: camelCase JSON keys `itemId`, `traitId`, `traitLevel`; stale snake_case display fallback

- [ ] **Step 1: Preserve the observed failing regression tests**

The backend assertion must require nested source keys:

```rust
assert_eq!(source["itemId"], 2);
assert_eq!(source["traitId"], 1);
assert_eq!(source["traitLevel"], 65);
assert!(source.get("item_id").is_none());
```

The frontend fixture must pass `item_id`, `trait_id`, and `trait_level` through an `unknown` cast and assert that `70 / 65` and `5 초과` still render.

- [ ] **Step 2: Run the focused tests and retain the RED evidence**

Run:

```powershell
C:\Users\azyu\.cargo\bin\cargo.exe test --locked --package gbfr-logs equipment::tests::response_uses_frontend_safe_camel_case_enum_values -- --exact
npm test -- --run src/pages/EquipmentAnalysis.test.tsx
```

Expected historical RED: backend `itemId` is `Null`; frontend throws while reading `source.itemId.toString`.

- [ ] **Step 3: Keep the minimal JSON and stale-payload implementation**

Apply camelCase only as a serde representation change; do not reorder fields:

```rust
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EquippedTraitSource {
    pub kind: EquipmentSourceKind,
    pub slot: u8,
    pub item_id: u32,
    pub trait_id: u32,
    pub trait_level: u32,
}
```

In the React renderer, resolve `itemId ?? item_id` and `traitLevel ?? trait_level`; render `????????` or `—` only if neither representation exists.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run the two commands from Step 2.

Expected: one Rust test passes and four `EquipmentAnalysis` tests pass without unhandled React errors.

- [ ] **Step 5: Commit the regression fix**

```powershell
git add -- protocol/src/lib.rs src-tauri/src/equipment/mod.rs src/pages/EquipmentAnalysis.tsx src/pages/EquipmentAnalysis.test.tsx
git commit -m "fix: preserve equipment analysis rendering"
```

---

### Task 2: Fix the Windows window policy

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/src/main.rs`
- Create: `src/pages/WindowConfiguration.test.ts`

**Interfaces:**
- Consumes: Tauri windows labeled `main` and `logs`
- Produces: `set_meter_enabled(enabled: bool) -> Result<(), String>` and fixed startup window policy

- [ ] **Step 1: Write failing configuration and Rust policy tests**

Create `src/pages/WindowConfiguration.test.ts` to parse `src-tauri/tauri.conf.json` and assert:

```ts
const main = config.tauri.windows.find((window) => window.label === "main");
const management = config.tauri.windows.find((window) => window.label === "logs");
expect(main?.alwaysOnTop).toBe(true);
expect(main?.skipTaskbar).toBe(true);
expect(management?.title).toBe("Djeeta MOD");
expect(management?.skipTaskbar ?? false).toBe(false);
```

In `src-tauri/src/main.rs`, define and test a pure decision boundary:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MeterWindowAction { Show, Hide }

#[test]
fn meter_visibility_maps_to_explicit_window_actions() {
    assert_eq!(meter_window_action(true), MeterWindowAction::Show);
    assert_eq!(meter_window_action(false), MeterWindowAction::Hide);
}

#[test]
fn click_through_starts_disabled_for_dragging() {
    assert!(!DEFAULT_CLICK_THROUGH);
}
```

- [ ] **Step 2: Run tests and verify RED**

```powershell
npm test -- --run src/pages/WindowConfiguration.test.ts
C:\Users\azyu\.cargo\bin\cargo.exe test --locked --package gbfr-logs meter_visibility_maps_to_explicit_window_actions
C:\Users\azyu\.cargo\bin\cargo.exe test --locked --package gbfr-logs click_through_starts_disabled_for_dragging
```

Expected: TypeScript test fails because `skipTaskbar` and the management title are absent; Rust compilation fails because the policy symbols do not exist.

- [ ] **Step 3: Implement the fixed policy and explicit command**

Set the Tauri window properties:

```json
{
  "label": "main",
  "alwaysOnTop": true,
  "skipTaskbar": true
}
```

and change the `logs` title to `Djeeta MOD`.

Add:

```rust
const DEFAULT_CLICK_THROUGH: bool = false;

fn meter_window_action(enabled: bool) -> MeterWindowAction {
    if enabled { MeterWindowAction::Show } else { MeterWindowAction::Hide }
}

#[tauri::command]
fn set_meter_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    let window = app.get_window("main").ok_or("meter window not found")?;
    match meter_window_action(enabled) {
        MeterWindowAction::Show => {
            window.set_always_on_top(true).map_err(|error| error.to_string())?;
            window.show().map_err(|error| error.to_string())
        }
        MeterWindowAction::Hide => window.hide().map_err(|error| error.to_string()),
    }
}
```

Register the command. Remove `AlwaysOnTop`, `toggle_always_on_top`, its tray item, and the `on-pinned` listener. Initialize `ClickThrough` with `DEFAULT_CLICK_THROUGH` and show its tray checkmark only when the state is true.

- [ ] **Step 4: Run focused tests and verify GREEN**

```powershell
npm test -- --run src/pages/WindowConfiguration.test.ts
C:\Users\azyu\.cargo\bin\cargo.exe test --locked --package gbfr-logs meter_visibility_maps_to_explicit_window_actions
C:\Users\azyu\.cargo\bin\cargo.exe test --locked --package gbfr-logs click_through_starts_disabled_for_dragging
```

Expected: all focused tests pass.

- [ ] **Step 5: Commit the window policy**

```powershell
git add -- src-tauri/tauri.conf.json src-tauri/src/main.rs src/pages/WindowConfiguration.test.ts src/pages/useMeter.ts
git commit -m "feat: define meter window policy"
```

---

### Task 3: Persist and synchronize meter visibility

**Files:**
- Modify: `src/stores/useMeterSettingsStore.ts`
- Create: `src/pages/useMeterVisibility.ts`
- Create: `src/pages/useMeterVisibility.test.tsx`

**Interfaces:**
- Consumes: Tauri command `set_meter_enabled` with `{ enabled: boolean }`
- Produces: `useMeterVisibility(): { meterEnabled: boolean; setMeterEnabled(enabled: boolean): Promise<void> }`

- [ ] **Step 1: Write failing hook tests**

Mock only the unavoidable Tauri boundary. Reset the Zustand store before each test and assert:

```ts
expect(useMeterSettingsStore.getState().meter_enabled).toBe(true);

const { result } = renderHook(() => useMeterVisibility());
await waitFor(() => expect(invoke).toHaveBeenCalledWith("set_meter_enabled", { enabled: true }));

await act(() => result.current.setMeterEnabled(false));
expect(invoke).toHaveBeenLastCalledWith("set_meter_enabled", { enabled: false });
expect(useMeterSettingsStore.getState().meter_enabled).toBe(false);
```

For rejection, make `invoke` reject and assert `meter_enabled` remains `true`.

- [ ] **Step 2: Run tests and verify RED**

```powershell
npm test -- --run src/pages/useMeterVisibility.test.tsx
```

Expected: compilation fails because `meter_enabled` and `useMeterVisibility` do not exist.

- [ ] **Step 3: Implement the persisted setting and hook**

Add to `MeterSettings` and its defaults:

```ts
meter_enabled: boolean;
// DEFAULT_METER_SETTINGS
meter_enabled: true,
```

Implement the hook so startup invokes the current persisted value. `setMeterEnabled` must await a successful command before calling `setMeterSettings({ meter_enabled: enabled })`; a rejected command leaves the persisted state unchanged.

- [ ] **Step 4: Run focused tests and verify GREEN**

```powershell
npm test -- --run src/pages/useMeterVisibility.test.tsx
```

Expected: default, startup synchronization, success, and failure tests pass.

- [ ] **Step 5: Commit visibility persistence**

```powershell
git add -- src/stores/useMeterSettingsStore.ts src/pages/useMeterVisibility.ts src/pages/useMeterVisibility.test.tsx
git commit -m "feat: persist meter visibility"
```

---

### Task 4: Build the management navigation

**Files:**
- Modify: `src/pages/Logs.tsx`
- Create: `src/pages/Logs.test.tsx`
- Modify: `src/pages/Settings.localization.test.ts`
- Modify: `src-tauri/lang/ko/ui.json`
- Modify: `src-tauri/lang/en/ui.json`
- Modify: `src/components/compact-meter/CompactDamageMeter.test.tsx`

**Interfaces:**
- Consumes: `useMeterVisibility` from Task 3
- Produces: translated management navigation and meter visibility switch

- [ ] **Step 1: Write failing navigation and waiting-meter tests**

Render `Logs` with router and Tauri listeners mocked. Assert the Korean labels `데미지 미터`, `진 특성 상한 분석`, `전투 기록`, and `설정` are present. Assert the meter switch starts checked, clicking its row calls `setMeterEnabled(false)`, and clicking the switch does not double-toggle through row bubbling.

Extend localization fixtures to require:

```json
{
  "damage-meter": "데미지 미터",
  "battle-records": "전투 기록",
  "settings": "설정"
}
```

with English equivalents `Damage Meter`, `Battle Records`, and `Settings`.

Render `CompactDamageMeter` with `rows={[]}` and assert the translated meter header is present and has `data-tauri-drag-region`.

- [ ] **Step 2: Run tests and verify RED**

```powershell
npm test -- --run src/pages/Logs.test.tsx src/pages/Settings.localization.test.ts src/components/compact-meter/CompactDamageMeter.test.tsx
```

Expected: labels remain hardcoded, the switch is absent, and new translation keys are missing.

- [ ] **Step 3: Implement the menu and translations**

Use a Mantine `Switch` in the first `NavLink`:

```tsx
<NavLink
  label={t("ui.navigation.damage-meter")}
  leftSection={<Gauge size="1rem" />}
  rightSection={
    <Switch
      aria-label={t("ui.navigation.damage-meter")}
      checked={meterEnabled}
      onClick={(event) => event.stopPropagation()}
      onChange={(event) => void setMeterEnabled(event.currentTarget.checked)}
    />
  }
  onClick={() => void setMeterEnabled(!meterEnabled)}
/>
```

Rename the home link to `ui.navigation.battle-records`, retain the existing `/logs` route, retain `/logs/equipment`, and translate Settings. Keep the current settings page and encounter-save navigation unchanged.

- [ ] **Step 4: Run focused tests and verify GREEN**

```powershell
npm test -- --run src/pages/Logs.test.tsx src/pages/Settings.localization.test.ts src/components/compact-meter/CompactDamageMeter.test.tsx
```

Expected: all management-navigation, translation, switch, and drag-header tests pass.

- [ ] **Step 5: Commit the management UI**

```powershell
git add -- src/pages/Logs.tsx src/pages/Logs.test.tsx src/pages/Settings.localization.test.ts src-tauri/lang/ko/ui.json src-tauri/lang/en/ui.json src/components/compact-meter/CompactDamageMeter.test.tsx
git commit -m "feat: control meter from management menu"
```

---

### Task 5: Full verification and game smoke test

**Files:**
- Modify: `docs/testing/game-2.0.2-smoke-test.md`
- Modify only if packaging succeeds and the project contract requires current artifact hashes: `README.md`

**Interfaces:**
- Consumes: completed Tasks 1–4
- Produces: verified build artifacts and recorded manual results

- [ ] **Step 1: Run all frontend quality gates**

```powershell
npm run format-check
npm run lint
npm run tsc
npm test -- --run
npm run build
```

Expected: every command exits 0. Existing Vite bundle-size notices are warnings, not failures.

- [ ] **Step 2: Run all Rust quality gates**

Load the Visual Studio 2022 x64 developer environment if the linker is unavailable, then run:

```powershell
C:\Users\azyu\.cargo\bin\cargo.exe build --release --locked --package hook
C:\Users\azyu\.cargo\bin\cargo.exe test --workspace --all-targets --locked
```

Expected: hook release build and every workspace test pass. Existing dead-code warnings from disabled auxiliary 2.0 hooks do not fail the build.

- [ ] **Step 3: Copy and verify the hook artifact after the game exits**

```powershell
Copy-Item -LiteralPath target\release\hook.dll -Destination src-tauri\hook.dll -Force
Get-FileHash -Algorithm SHA256 target\release\hook.dll,src-tauri\hook.dll
```

Expected: both SHA-256 values are identical.

- [ ] **Step 4: Build the MSI**

```powershell
npm run tauri build -- --bundles msi
```

Expected: command exits 0 and produces a Windows MSI under `target/release/bundle/msi`.

- [ ] **Step 5: Run the local game smoke test**

Verify in Granblue Fantasy: Relink Endless Ragnarok 2.0.2:

1. Taskbar shows only `Djeeta MOD`, not a separate meter entry.
2. Management window title is `Djeeta MOD`.
3. Left menu shows `데미지 미터`, `진 특성 상한 분석`, `전투 기록`, `설정`.
4. Meter ON shows the meter above the game even while waiting for combat.
5. Meter header drags while click-through is OFF.
6. Tray click-through ON prevents dragging and lets game clicks pass through.
7. Meter OFF hides the meter; ON restores it at its saved position.
8. Restart preserves meter visibility.
9. Narmaya equipment analysis renders without a blank screen and shows Damage Cap `70 / 65`, `5 초과`.

- [ ] **Step 6: Record evidence without overclaiming compatibility**

Append the date, executable hash, hook hash, MSI hash, and each pass/fail result to `docs/testing/game-2.0.2-smoke-test.md`. Update `README.md` artifact hashes only when the MSI and copied hook are final. Do not mark overall 2.0.2 compatibility verified unless the complete project smoke checklist passes.

- [ ] **Step 7: Commit verification records**

```powershell
git add -- docs/testing/game-2.0.2-smoke-test.md README.md
git commit -m "docs: record meter window smoke test"
```

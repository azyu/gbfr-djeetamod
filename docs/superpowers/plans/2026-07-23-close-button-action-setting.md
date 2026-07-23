# Close Button Action Setting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users choose whether the management window's X button minimizes Djeeta MOD to the tray or quits the application.

**Architecture:** Persist an explicit close action in the existing Zustand settings store and synchronize it to an application-wide Rust atomic value through one React hook. The native close handler maps the management window and atomic value to a small, unit-tested decision enum; only the management window can exit through this option.

**Tech Stack:** React 18, TypeScript, Zustand, Mantine 7, Vitest, Tauri 1, Rust

## Global Constraints

- The default is `minimize-to-tray`, including for existing persisted settings that lack the new field.
- The option applies only to the `logs` management window; other close requests retain hide-and-prevent behavior.
- The tray menu's `Quit` item always exits.
- The UI and approved copy are localized in Korean and English.
- Do not rename the `meter-settings` storage key or the existing store.
- Add tests before changing lifecycle behavior.

---

### Task 1: Persist and Synchronize the Close Action

**Files:**
- Modify: `src/stores/useMeterSettingsStore.ts`
- Create: `src/pages/useCloseButtonAction.ts`
- Create: `src/pages/useCloseButtonAction.test.tsx`
- Modify: `src/App.tsx`

**Interfaces:**
- Produces: `CloseButtonAction = "minimize-to-tray" | "quit"`
- Produces: persisted `close_button_action: CloseButtonAction`
- Produces: `useCloseButtonAction(): void`, which invokes `set_close_to_tray` with `{ enabled: boolean }`
- Consumes later: `useSettings` and `SettingsPage` read and update `close_button_action`

- [ ] **Step 1: Write the failing synchronization tests**

Create `src/pages/useCloseButtonAction.test.tsx` using the existing jsdom Vitest setup:

```tsx
import { useMeterSettingsStore } from "@/stores/useMeterSettingsStore";
import { invoke } from "@tauri-apps/api";
import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import useCloseButtonAction from "./useCloseButtonAction";

vi.mock("@tauri-apps/api", () => ({ invoke: vi.fn().mockResolvedValue(undefined) }));

beforeEach(() => {
  localStorage.clear();
  useMeterSettingsStore.setState({ close_button_action: "minimize-to-tray" });
  vi.mocked(invoke).mockClear();
});

it("defaults to minimizing the management window to the tray", async () => {
  renderHook(() => useCloseButtonAction());

  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("set_close_to_tray", { enabled: true })
  );
});

it("synchronizes a persisted quit selection", async () => {
  renderHook(() => useCloseButtonAction());

  act(() => useMeterSettingsStore.getState().set({ close_button_action: "quit" }));

  await waitFor(() =>
    expect(invoke).toHaveBeenLastCalledWith("set_close_to_tray", { enabled: false })
  );
  expect(useMeterSettingsStore.getState().close_button_action).toBe("quit");
});
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```powershell
npm.cmd test -- --run src/pages/useCloseButtonAction.test.tsx
```

Expected: FAIL because `useCloseButtonAction` and `close_button_action` do not exist.

- [ ] **Step 3: Add the minimal persisted state and synchronization hook**

In `src/stores/useMeterSettingsStore.ts`, export the type, add the field to
`MeterSettings`, and add its default:

```ts
export type CloseButtonAction = "minimize-to-tray" | "quit";

interface MeterSettings {
  // existing fields
  close_button_action: CloseButtonAction;
}

const DEFAULT_METER_SETTINGS: MeterSettings = {
  // existing defaults
  close_button_action: "minimize-to-tray",
};
```

Create `src/pages/useCloseButtonAction.ts`:

```ts
import { useMeterSettingsStore } from "@/stores/useMeterSettingsStore";
import { invoke } from "@tauri-apps/api";
import { useEffect } from "react";

export default function useCloseButtonAction() {
  const closeButtonAction = useMeterSettingsStore((state) => state.close_button_action);

  useEffect(() => {
    invoke("set_close_to_tray", {
      enabled: closeButtonAction === "minimize-to-tray",
    }).catch((error) => console.error("Failed to synchronize close button action:", error));
  }, [closeButtonAction]);
}
```

Call `useCloseButtonAction()` once at the top of the `App` component before
rendering the router. This makes startup and later store changes use the same
synchronization path.

- [ ] **Step 4: Run the focused test and verify GREEN**

Run:

```powershell
npm.cmd test -- --run src/pages/useCloseButtonAction.test.tsx
```

Expected: both tests PASS.

- [ ] **Step 5: Commit the frontend state boundary**

```powershell
git add -- src/stores/useMeterSettingsStore.ts src/pages/useCloseButtonAction.ts src/pages/useCloseButtonAction.test.tsx src/App.tsx
git commit -m "feat: synchronize close button preference"
```

### Task 2: Add the General Settings UI and Localized Copy

**Files:**
- Modify: `src/pages/Settings.tsx`
- Modify: `src/pages/useSettings.ts`
- Modify: `src/pages/Settings.localization.test.ts`
- Modify: `src-tauri/lang/en/ui.json`
- Modify: `src-tauri/lang/ko/ui.json`

**Interfaces:**
- Consumes: `CloseButtonAction` and `close_button_action` from Task 1
- Produces: a controlled Mantine `Select` that stores either `"minimize-to-tray"` or `"quit"`

- [ ] **Step 1: Extend the localization regression test first**

Add these exact entries to `expectedEnglish` and `expectedKorean` in
`src/pages/Settings.localization.test.ts`:

```ts
// English
"general-settings": "General Settings",
"close-button-action": "Close Button Action",
"close-button-action-description": "Choose what the management window's X button does.",
"close-button-minimize-to-tray": "Minimize to Tray",
"close-button-quit": "Quit Application",

// Korean
"general-settings": "일반 설정",
"close-button-action": "닫기 버튼 동작",
"close-button-action-description": "관리 창의 X 버튼을 눌렀을 때 수행할 동작을 선택합니다.",
"close-button-minimize-to-tray": "트레이로 최소화",
"close-button-quit": "프로그램 종료",
```

Also assert that the `general-settings` fieldset appears before
`meter-settings` and that `Settings.tsx` passes `close_button_action` as the
controlled select value.

- [ ] **Step 2: Run the localization test and verify RED**

Run:

```powershell
npm.cmd test -- --run src/pages/Settings.localization.test.ts
```

Expected: FAIL because the translation keys and general settings fieldset are
missing.

- [ ] **Step 3: Expose the setting through `useSettings`**

Select `close_button_action` from `useMeterSettingsStore`, return it from the
hook, and reuse `setMeterSettings` for changes. Import `CloseButtonAction` in
`Settings.tsx` so the nullable Mantine value can be narrowed explicitly:

```ts
onChange={(value) => {
  if (value) {
    setMeterSettings({ close_button_action: value as CloseButtonAction });
  }
}}
```

- [ ] **Step 4: Render the general settings fieldset**

Add a `Fieldset` above the existing meter fieldset, move the current language
`Select` into it, and render:

```tsx
<Select
  label={t("ui.close-button-action")}
  description={t("ui.close-button-action-description")}
  data={[
    { value: "minimize-to-tray", label: t("ui.close-button-minimize-to-tray") },
    { value: "quit", label: t("ui.close-button-quit") },
  ]}
  value={close_button_action}
  allowDeselect={false}
  onChange={(value) => {
    if (value) {
      setMeterSettings({ close_button_action: value as CloseButtonAction });
    }
  }}
/>
```

Add `mt="md"` to the existing meter fieldset so the two groups have the same
spacing used before the updater section.

- [ ] **Step 5: Add the exact Korean and English JSON values**

Add the five flat `ui` keys from Step 1 to both locale files. Preserve valid
JSON and existing key ordering near `language` and `meter-settings`.

- [ ] **Step 6: Run the focused frontend tests and verify GREEN**

Run:

```powershell
npm.cmd test -- --run src/pages/Settings.localization.test.ts src/pages/useCloseButtonAction.test.tsx
```

Expected: all tests PASS.

- [ ] **Step 7: Commit the UI**

```powershell
git add -- src/pages/Settings.tsx src/pages/useSettings.ts src/pages/Settings.localization.test.ts src-tauri/lang/en/ui.json src-tauri/lang/ko/ui.json
git commit -m "feat: add close button action setting"
```

### Task 3: Apply the Preference in the Native Close Handler

**Files:**
- Modify: `src-tauri/src/main.rs`

**Interfaces:**
- Consumes: frontend command `set_close_to_tray(enabled: bool)`
- Produces: `CloseRequestAction::{Hide, Exit}`
- Produces: `close_request_action(window_label: &str, close_to_tray: bool) -> CloseRequestAction`

- [ ] **Step 1: Write the failing Rust decision tests**

Add the helper imports and tests to the existing `#[cfg(test)]` module:

```rust
#[test]
fn management_close_hides_when_close_to_tray_is_enabled() {
    assert_eq!(
        close_request_action("logs", true),
        CloseRequestAction::Hide
    );
}

#[test]
fn management_close_exits_when_close_to_tray_is_disabled() {
    assert_eq!(
        close_request_action("logs", false),
        CloseRequestAction::Exit
    );
}

#[test]
fn unrelated_window_close_keeps_existing_hide_behavior() {
    assert_eq!(
        close_request_action("main", false),
        CloseRequestAction::Hide
    );
}
```

- [ ] **Step 2: Run the focused Rust test and verify RED**

Run:

```powershell
cargo test --package gbfr-logs management_close --locked
```

Expected: FAIL because the helper and enum do not exist.

- [ ] **Step 3: Implement the close decision and command**

Near the existing atomic state declarations, add:

```rust
struct CloseToTray(AtomicBool);

const DEFAULT_CLOSE_TO_TRAY: bool = true;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CloseRequestAction {
    Hide,
    Exit,
}

fn close_request_action(window_label: &str, close_to_tray: bool) -> CloseRequestAction {
    if window_label == "logs" && !close_to_tray {
        CloseRequestAction::Exit
    } else {
        CloseRequestAction::Hide
    }
}

#[tauri::command]
fn set_close_to_tray(state: State<CloseToTray>, enabled: bool) {
    state.0.store(enabled, Ordering::Release);
}
```

Manage `CloseToTray(AtomicBool::new(DEFAULT_CLOSE_TO_TRAY))`, register
`set_close_to_tray` in `generate_handler!`, and replace the unconditional close
handler body with:

```rust
let close_to_tray = event
    .window()
    .state::<CloseToTray>()
    .0
    .load(Ordering::Acquire);

match close_request_action(event.window().label(), close_to_tray) {
    CloseRequestAction::Hide => {
        event.window().hide().unwrap();
        api.prevent_close();
    }
    CloseRequestAction::Exit => {
        let handle = event.window().app_handle();
        let _ = handle.save_window_state(StateFlags::all());
        handle.exit(0);
    }
}
```

- [ ] **Step 4: Run focused Rust and frontend tests**

Run:

```powershell
cargo test --package gbfr-logs management_close --locked
npm.cmd test -- --run src/pages/useCloseButtonAction.test.tsx src/pages/Settings.localization.test.ts
```

Expected: all focused tests PASS.

- [ ] **Step 5: Commit the native lifecycle change**

```powershell
git add -- src-tauri/src/main.rs
git commit -m "feat: honor close button action"
```

### Task 4: Full Verification

**Files:**
- No production file changes expected

**Interfaces:**
- Consumes: completed frontend and Rust implementation
- Produces: verification evidence required by the maintainer guide

- [ ] **Step 1: Run all required frontend checks**

Run in order:

```powershell
npm.cmd run format-check
npm.cmd run lint
npm.cmd run tsc
npm.cmd test -- --run
npm.cmd run build
```

Expected: every command exits 0.

- [ ] **Step 2: Load the Visual Studio developer environment if needed**

If `cl.exe` is not available, locate Visual Studio 2022 with `vswhere.exe` and
run subsequent Rust commands from a shell initialized by `VsDevCmd.bat`.

- [ ] **Step 3: Run all required Rust checks**

```powershell
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
```

Expected: both commands exit 0.

- [ ] **Step 4: Inspect the final scope**

Run:

```powershell
git status --short
git diff main...HEAD --stat
git log --oneline main..HEAD
```

Expected: only the approved design, plan, close-action implementation, tests,
and locale changes appear; the pre-existing unstaged `AGENTS.md` remains
uncommitted.

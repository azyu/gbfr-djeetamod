# Shared Game Connection Status Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show one authoritative, localized game-connection status at the right edge of the management header and remove duplicate page-level game-not-running messages.

**Architecture:** A focused `useConnectionState` hook registers one management-window listener before reading the current backend state, ignores a stale initial response after a newer event, and disposes safely. `Logs` owns this hook and renders the header; settings stops subscribing, while repeat quest retains its independent refresh listener but suppresses only its duplicate `gameNotRunning` presentation.

**Tech Stack:** React 18, TypeScript, Mantine 7, Tauri 1 event/command APIs, i18next, Vitest, Testing Library

## Global Constraints

- Keep the existing backend `ConnectionState` variants and hook lifecycle unchanged.
- Keep compact-meter connection handling unchanged because it runs in a separate WebView.
- Do not persist or infer connection state in the frontend.
- Display every state in the management header.
- Use the exact Korean labels from the approved design.
- Add no icons, badges, animation, or new status colors.
- Preserve unrelated working-tree changes and do not stage `logs.db`.

---

## File Structure

- Create `src/pages/useConnectionState.ts`: management-window connection subscription and initial-state read.
- Create `src/pages/useConnectionState.test.tsx`: lifecycle, event ordering, failure, and cleanup tests for the hook.
- Modify `src/pages/Logs.tsx`: single hook owner, left/right header layout, and duplicate repeat-quest reason suppression.
- Modify `src/pages/Logs.test.tsx`: header state rendering and layout coverage.
- Modify `src/pages/Logs.repeatQuest.test.tsx`: duplicate/common versus feature-specific reason coverage.
- Modify `src/pages/useSettings.ts`: remove the settings-owned connection subscription.
- Modify `src/pages/Settings.tsx`: remove the settings-page connection line.
- Modify `src/pages/Settings.localization.test.ts`: exact bilingual connection labels and source ownership assertions.
- Modify `src-tauri/lang/ko/ui.json`: approved Korean connection labels.
- Modify `src-tauri/lang/en/ui.json`: equivalent English connection labels.

### Task 1: Management Connection-State Hook

**Files:**
- Create: `src/pages/useConnectionState.ts`
- Create: `src/pages/useConnectionState.test.tsx`

**Interfaces:**
- Consumes: Tauri `invoke<ConnectionState>("get_connection_state")` and `listen<ConnectionState>("connection-state", callback)`.
- Produces: `export default function useConnectionState(): ConnectionState`.

- [ ] **Step 1: Write the failing hook tests**

Create `src/pages/useConnectionState.test.tsx` with hoisted mocks that capture the `connection-state` callback and unlisten function. Cover these exact cases:

```tsx
it("reads the current state after registering the listener", async () => {
  const { result } = renderHook(() => useConnectionState());

  expect(result.current).toBe("searching");
  await waitFor(() => expect(result.current).toBe("connected"));
  expect(mocks.listen).toHaveBeenCalledWith("connection-state", expect.any(Function));
  expect(mocks.invoke).toHaveBeenCalledWith("get_connection_state");
  expect(mocks.listen.mock.invocationCallOrder[0]).toBeLessThan(mocks.invoke.mock.invocationCallOrder[0]);
});

it("uses an event that arrives before the initial read resolves", async () => {
  const initial = deferred<ConnectionState>();
  mocks.invoke.mockReturnValueOnce(initial.promise);
  const { result } = renderHook(() => useConnectionState());
  await waitFor(() => expect(mocks.listeners.has("connection-state")).toBe(true));

  act(() => mocks.listeners.get("connection-state")?.({ payload: "disconnected" }));
  initial.resolve("connected");

  await waitFor(() => expect(result.current).toBe("disconnected"));
});

it("keeps searching when the initial read fails", async () => {
  mocks.invoke.mockRejectedValueOnce(new Error("invoke failed"));
  const { result } = renderHook(() => useConnectionState());
  await waitFor(() => expect(mocks.invoke).toHaveBeenCalled());
  expect(result.current).toBe("searching");
});

it("unsubscribes and ignores late initial results after unmount", async () => {
  const initial = deferred<ConnectionState>();
  mocks.invoke.mockReturnValueOnce(initial.promise);
  const { unmount } = renderHook(() => useConnectionState());
  await waitFor(() => expect(mocks.listeners.has("connection-state")).toBe(true));

  unmount();
  initial.resolve("connected");

  await waitFor(() => expect(mocks.unlisten).toHaveBeenCalledTimes(1));
});
```

Define `deferred<T>()` in the test file as a small promise helper, default `invoke` to `connected`, and reset all mocks/maps in `beforeEach`.

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```powershell
npm test -- --run src/pages/useConnectionState.test.tsx
```

Expected: FAIL because `./useConnectionState` does not exist.

- [ ] **Step 3: Implement the minimal hook**

Create `src/pages/useConnectionState.ts`:

```ts
import { ConnectionState } from "@/types";
import { invoke } from "@tauri-apps/api";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

export default function useConnectionState(): ConnectionState {
  const [connectionState, setConnectionState] = useState<ConnectionState>("searching");

  useEffect(() => {
    let disposed = false;
    let eventSeen = false;
    let unsubscribe: (() => void) | undefined;

    void listen<ConnectionState>("connection-state", (event) => {
      eventSeen = true;
      if (!disposed) setConnectionState(event.payload);
    })
      .then(async (unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        unsubscribe = unlisten;

        try {
          const currentState = await invoke<ConnectionState>("get_connection_state");
          if (!disposed && !eventSeen) setConnectionState(currentState);
        } catch {
          // Keep the stable searching state until a backend event arrives.
        }
      })
      .catch(() => undefined);

    return () => {
      disposed = true;
      unsubscribe?.();
    };
  }, []);

  return connectionState;
}
```

- [ ] **Step 4: Run the focused test and verify GREEN**

Run:

```powershell
npm test -- --run src/pages/useConnectionState.test.tsx
```

Expected: 4 tests pass.

- [ ] **Step 5: Commit the hook**

```powershell
git add -- src/pages/useConnectionState.ts src/pages/useConnectionState.test.tsx
git commit -m "feat: centralize management connection state"
```

### Task 2: Header Status and Localization

**Files:**
- Modify: `src/pages/Logs.tsx`
- Modify: `src/pages/Logs.test.tsx`
- Modify: `src/pages/Settings.localization.test.ts`
- Modify: `src-tauri/lang/ko/ui.json`
- Modify: `src-tauri/lang/en/ui.json`

**Interfaces:**
- Consumes: `useConnectionState(): ConnectionState` from Task 1 and `t("ui.connection.<state>")`.
- Produces: a management header with left title controls and one right-aligned state label.

- [ ] **Step 1: Add failing header and localization tests**

In `src/pages/Logs.test.tsx`, add `connectionState` to the hoisted mocks, mock `./useConnectionState`, reset it to `searching` in `beforeEach`, add all four `ui.connection.*` translations to the translation mock, and add:

```tsx
it.each([
  ["searching", "게임을 찾는 중입니다"],
  ["connected", "게임에 연결되었습니다"],
  ["disconnected", "게임 실행 중이 아닙니다"],
  ["unsupported", "지원하지 않는 게임 버전입니다"],
])("shows the %s game state in the management header", (state, label) => {
  mocks.connectionState = state as ConnectionState;
  renderLayout();

  const header = screen.getByRole("banner");
  expect(within(header).getByText("Djeeta MOD")).toBeTruthy();
  expect(within(header).getByText(label)).toBeTruthy();
});
```

Import `ConnectionState` from `@/types` and `within` from Testing Library. Add a source assertion that the outer header `Group` uses `justify="space-between"` and `wrap="nowrap"`.

In `src/pages/Settings.localization.test.ts`, define and assert these exact objects:

```ts
const expectedEnglishConnection = {
  searching: "Looking for the game",
  connected: "Connected to the game",
  disconnected: "The game is not running",
  unsupported: "This game version is not supported",
};

const expectedKoreanConnection = {
  searching: "게임을 찾는 중입니다",
  connected: "게임에 연결되었습니다",
  disconnected: "게임 실행 중이 아닙니다",
  unsupported: "지원하지 않는 게임 버전입니다",
};
```

Expected assertions: both locale objects equal these maps, and `Logs.tsx` contains ``t(`ui.connection.${connectionState}`)``.

- [ ] **Step 2: Run the focused tests and verify RED**

Run:

```powershell
npm test -- --run src/pages/Logs.test.tsx src/pages/Settings.localization.test.ts
```

Expected: FAIL because the header does not own connection state and the locale strings do not match.

- [ ] **Step 3: Add the header owner and exact translations**

In `src/pages/Logs.tsx`, import `useConnectionState`, call it once in `Layout`, and replace the header group with:

```tsx
<Group h="100%" px="sm" justify="space-between" wrap="nowrap">
  <Group gap="sm" wrap="nowrap">
    <Burger opened={mobileOpened} onClick={toggleMobile} hiddenFrom="sm" size="sm" />
    <Burger opened={desktopOpened} onClick={toggleDesktop} visibleFrom="sm" size="sm" />
    <Text>Djeeta MOD</Text>
  </Group>
  <Text size="sm" ta="right" truncate>
    {t(`ui.connection.${connectionState}`)}
  </Text>
</Group>
```

Replace the four `ui.connection` values in Korean and English with the exact maps from Step 1. Do not alter other localization keys.

- [ ] **Step 4: Run the focused tests and verify GREEN**

Run:

```powershell
npm test -- --run src/pages/useConnectionState.test.tsx src/pages/Logs.test.tsx src/pages/Settings.localization.test.ts
```

Expected: all focused tests pass.

- [ ] **Step 5: Commit the header and localization**

```powershell
git add -- src/pages/Logs.tsx src/pages/Logs.test.tsx src/pages/Settings.localization.test.ts src-tauri/lang/ko/ui.json src-tauri/lang/en/ui.json
git commit -m "feat: show game status in management header"
```

### Task 3: Remove Duplicate Page-Level Status

**Files:**
- Modify: `src/pages/useSettings.ts`
- Modify: `src/pages/Settings.tsx`
- Modify: `src/pages/Logs.tsx`
- Modify: `src/pages/Logs.repeatQuest.test.tsx`
- Modify: `src/pages/Settings.localization.test.ts`

**Interfaces:**
- Consumes: the header status implemented in Task 2 and existing `RepeatQuestStatus.reason` values.
- Produces: no settings-owned connection state and no sidebar copy of the common `gameNotRunning` condition.

- [ ] **Step 1: Write failing ownership and reason tests**

Change the existing repeat-quest reason test in `src/pages/Logs.repeatQuest.test.tsx` to assert that `gameNotRunning` is absent while the switch remains disabled:

```tsx
it("leaves the common game-not-running state to the header", () => {
  mocks.repeatStatus = { state: "unavailable", reason: "gameNotRunning" };
  renderLayout();

  expect(screen.queryByText("게임 실행 중이 아닙니다.")).toBeNull();
  expect((screen.getByRole("switch", { name: "무한 퀘스트 반복" }) as HTMLInputElement).disabled).toBe(true);
});
```

Add a feature-specific reason test using `accessDenied`:

```tsx
it("keeps a repeat-quest-specific failure below the switch", () => {
  mocks.repeatStatus = { state: "unavailable", reason: "accessDenied" };
  renderLayout();

  expect(screen.getByText("현재 권한으로 게임 코드를 변경할 수 없습니다.")).toBeTruthy();
});
```

Add both Korean strings to the translation mock. In `src/pages/Settings.localization.test.ts`, assert that neither `Settings.tsx` nor `useSettings.ts` contains `connectionState`, `connection-state`, or `get_connection_state`.

- [ ] **Step 2: Run the focused tests and verify RED**

Run:

```powershell
npm test -- --run src/pages/Logs.repeatQuest.test.tsx src/pages/Settings.localization.test.ts
```

Expected: FAIL because the sidebar still renders `gameNotRunning` and settings still owns the subscription.

- [ ] **Step 3: Remove only the duplicate ownership and presentation**

In `src/pages/useSettings.ts`:

- remove `ConnectionState`, `listen`, `useEffect`, and `useState` imports that exist only for connection status;
- remove the `connectionState` state and connection-listener effect;
- remove `connectionState` from the returned object.

In `src/pages/Settings.tsx`, remove `connectionState` from the hook destructure and remove:

```tsx
<Text size="sm" c="dimmed">
  {t(`ui.connection.${connectionState}`)}
</Text>
```

In `src/pages/Logs.tsx`, narrow the existing reason condition:

```tsx
{repeatQuest.status?.reason && repeatQuest.status.reason !== "gameNotRunning" && (
  <Text size="xs" c="red" px="sm" pb="xs">
    {t(`ui.game-features.repeat-quest.reason.${repeatQuest.status.reason}`)}
  </Text>
)}
```

Do not remove the repeat-quest connection listener; it refreshes backend-observed patch state rather than presenting the common connection label.

- [ ] **Step 4: Run the focused regression tests and verify GREEN**

Run:

```powershell
npm test -- --run src/pages/Logs.repeatQuest.test.tsx src/pages/Settings.localization.test.ts src/pages/Logs.test.tsx src/pages/useRepeatQuest.test.tsx src/pages/useConnectionState.test.tsx src/pages/useCompactMeter.test.tsx
```

Expected: all focused tests pass.

- [ ] **Step 5: Commit duplicate removal**

```powershell
git add -- src/pages/useSettings.ts src/pages/Settings.tsx src/pages/Logs.tsx src/pages/Logs.repeatQuest.test.tsx src/pages/Settings.localization.test.ts
git commit -m "fix: remove duplicate game connection messages"
```

### Task 4: Full Verification and Manual Layout Check

**Files:**
- Verify only; no source changes expected.

**Interfaces:**
- Consumes: completed Tasks 1-3.
- Produces: verification evidence for the merged management-window behavior.

- [ ] **Step 1: Run frontend static checks**

```powershell
npm run format-check
npm run lint
npm run tsc
```

Expected: all commands exit 0.

- [ ] **Step 2: Run full automated regression and production build**

```powershell
npm test -- --run
npm run build
cargo test --workspace --all-targets --locked
```

Expected: all tests pass and the production frontend build exits 0. Existing non-fatal chunk-size and Rust dead-code warnings may remain.

- [ ] **Step 3: Verify the management window at 800x600**

Run the development app using the repository's existing Tauri workflow, open the management window, and confirm:

- `Djeeta MOD` remains readable on the left;
- each simulated or naturally observed connection state remains readable on the right;
- the disconnected label is exactly `게임 실행 중이 아닙니다`;
- settings no longer repeats the state;
- repeat quest does not repeat the disconnected message;
- sidebar and main scroll behavior remain unchanged.

- [ ] **Step 4: Check scope and working-tree preservation**

```powershell
git diff --check
git status --short
git log -5 --oneline
```

Expected: no whitespace errors; only the known unrelated user files and `logs.db` remain outside the feature commits.

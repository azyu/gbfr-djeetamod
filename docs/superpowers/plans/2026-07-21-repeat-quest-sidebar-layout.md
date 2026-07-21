# Repeat Quest Sidebar Layout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move `무한 퀘스트 반복` to the management sidebar, keep `설정` anchored at the bottom, and make long management pages vertically scrollable at any window height.

**Architecture:** `useRepeatQuest` becomes a self-contained, non-persistent frontend controller that refreshes on the existing Tauri `connection-state` event. `Logs.tsx` becomes the only UI owner of the switch. The AppShell sidebar uses a scrollable growing section above a fixed settings section, while `AppShell.Main` owns the management window's vertical scrolling.

**Tech Stack:** React 18, TypeScript, Mantine 7 AppShell/ScrollArea/NavLink/Switch, Tauri 1 event and invoke APIs, Vitest, Testing Library, CSS.

## Global Constraints

- Remove the repeat-quest control from `SettingsPage`; do not render duplicate controls.
- Keep repeat-quest state non-persistent and backend-observed.
- Do not change the Rust patch implementation, permissions, startup restoration, or exit restoration.
- Keep `설정` outside the growing sidebar section so it remains fixed at the bottom.
- Reuse the existing global WebKit scrollbar styling.
- Preserve unrelated changes in `Cargo.toml`, inventory scanner documents, `src-tauri/src/equipment_probe/inventory.rs`, and untracked `logs.db`.
- Work inline in the current branch; do not create a worktree or use subagents unless the user explicitly changes that decision.

---

### Task 1: Make the sidebar the single repeat-quest UI owner

**Files:**
- Create: `src/pages/useRepeatQuest.test.tsx`
- Create: `src/pages/Logs.repeatQuest.test.tsx`
- Modify: `src/pages/useRepeatQuest.ts`
- Modify: `src/pages/Logs.tsx`
- Modify: `src/pages/Settings.tsx`
- Modify: `src/pages/Settings.localization.test.ts`
- Delete: `src/pages/Settings.repeatQuest.test.tsx`

**Interfaces:**
- Consumes: Tauri commands `get_repeat_quest_status` and `set_repeat_quest_enabled`; event `connection-state`.
- Produces: `useRepeatQuest()` returning `{ status, pending, setEnabled }` without a connection-state parameter.
- Produces: one sidebar switch labelled by `ui.game-features.repeat-quest.label`.

- [ ] **Step 1: Write failing hook tests**

Create `src/pages/useRepeatQuest.test.tsx` with a mocked `invoke` and `listen`, then exercise the real hook:

```tsx
it("refreshes the backend-observed status on a connection-state event", async () => {
  const { result } = renderHook(() => useRepeatQuest());
  await waitFor(() => expect(result.current.status?.state).toBe("off"));

  mocks.status = { state: "on", reason: null };
  await act(async () => mocks.listeners.get("connection-state")?.({ payload: "connected" }));

  await waitFor(() => expect(result.current.status?.state).toBe("on"));
  expect(mocks.invoke).toHaveBeenLastCalledWith("get_repeat_quest_status");
});
```

Also retain direct hook coverage for initial pending state, `{ enabled: true }`, backend-observed enable results, and rejected invokes preserving the last observed state with reason `internal`.

- [ ] **Step 2: Run the hook test and verify RED**

Run:

```powershell
npm test -- --run src/pages/useRepeatQuest.test.tsx
```

Expected: FAIL because `useRepeatQuest` still requires a connection-state argument and does not subscribe to `connection-state` itself.

- [ ] **Step 3: Implement self-contained refresh subscription**

In `useRepeatQuest.ts`, expose `useRepeatQuest()` without arguments. Keep one local `refresh` callback that invokes `get_repeat_quest_status`. On mount, call `refresh`, then subscribe to `connection-state` and call `refresh` for every event. Dispose the listener on unmount and ignore late command results after disposal. Keep `setEnabled` unchanged in contract:

```ts
const setEnabled = useCallback(async (enabled: boolean) => {
  setPending(true);
  try {
    setStatus(await invoke<RepeatQuestStatus>("set_repeat_quest_enabled", { enabled }));
  } catch {
    setStatus((previous) =>
      previous ? { ...previous, reason: "internal" } : { state: "unavailable", reason: "internal" }
    );
  } finally {
    setPending(false);
  }
}, []);
```

- [ ] **Step 4: Run the hook test and verify GREEN**

Run:

```powershell
npm test -- --run src/pages/useRepeatQuest.test.tsx
```

Expected: all hook lifecycle, refresh, pending, and failure tests pass.

- [ ] **Step 5: Write failing sidebar ownership tests**

Create `src/pages/Logs.repeatQuest.test.tsx` by rendering `Logs` in a `MemoryRouter` with `useMeterVisibility` and `useRepeatQuest` mocked. Require the switch to follow the damage-meter row and invoke only `setEnabled`:

```tsx
const meter = screen.getByRole("switch", { name: "데미지 미터" });
const repeat = screen.getByRole("switch", { name: "무한 퀘스트 반복" });
expect(meter.compareDocumentPosition(repeat) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();

fireEvent.click(repeat);
expect(mocks.setRepeatEnabled).toHaveBeenCalledWith(true);
```

Add a source assertion that `Settings.tsx` does not contain `useRepeatQuest` or `ui.game-features.repeat-quest.label`. Update `Settings.localization.test.ts` to require the repeat-quest translation keys in `Logs.tsx`, not `Settings.tsx`.

- [ ] **Step 6: Run sidebar tests and verify RED**

Run:

```powershell
npm test -- --run src/pages/Logs.repeatQuest.test.tsx src/pages/Settings.localization.test.ts
```

Expected: FAIL because the switch is still rendered only by `SettingsPage`.

- [ ] **Step 7: Move the switch to `Logs.tsx`**

Remove `Switch`, `useRepeatQuest`, the repeat state, and the game-feature `Fieldset` from `Settings.tsx`. In `Logs.tsx`, call `const repeatQuest = useRepeatQuest();` and add this immediately after the damage-meter `NavLink`:

```tsx
<NavLink
  label={t("ui.game-features.repeat-quest.label")}
  rightSection={
    <Switch
      aria-label={t("ui.game-features.repeat-quest.label")}
      checked={repeatQuest.status?.state === "on"}
      disabled={repeatQuest.pending || repeatQuest.status === null || repeatQuest.status.state === "unavailable"}
      onClick={(event) => event.stopPropagation()}
      onChange={(event) => void repeatQuest.setEnabled(event.currentTarget.checked)}
    />
  }
/>
{repeatQuest.status?.reason && (
  <Text size="xs" c="red" px="sm" pb="xs">
    {t(`ui.game-features.repeat-quest.reason.${repeatQuest.status.reason}`)}
  </Text>
)}
```

Do not give the row a route or a parent click handler; only the switch changes state.

- [ ] **Step 8: Run sidebar and regression tests and verify GREEN**

Run:

```powershell
npm test -- --run src/pages/useRepeatQuest.test.tsx src/pages/Logs.repeatQuest.test.tsx src/pages/Logs.test.tsx src/pages/Settings.localization.test.ts src/securityConfiguration.test.ts
```

Expected: the hook, sidebar ownership, existing navigation, localization, and security tests all pass.

- [ ] **Step 9: Commit the single-owner sidebar control**

```powershell
git add -- src/pages/useRepeatQuest.ts src/pages/useRepeatQuest.test.tsx src/pages/Logs.tsx src/pages/Logs.repeatQuest.test.tsx src/pages/Settings.tsx src/pages/Settings.repeatQuest.test.tsx src/pages/Settings.localization.test.ts
git commit -m "fix: move repeat quest toggle to sidebar"
```

---

### Task 2: Bound sidebar and main-content scrolling

**Files:**
- Modify: `src/pages/Logs.tsx`
- Modify: `src/pages/Logs.css`
- Modify: `src/pages/Logs.test.tsx`

**Interfaces:**
- Consumes: the AppShell hierarchy from Task 1.
- Produces: a growing scrollable navigation section, a fixed bottom settings section, and `.log-main` vertical overflow.

- [ ] **Step 1: Write failing layout-boundary tests**

Extend `Logs.test.tsx` with source-level assertions that capture the structural contract:

```tsx
it("keeps settings below the scrollable navigation section", () => {
  const source = readFileSync(resolve(process.cwd(), "src/pages/Logs.tsx"), "utf8");
  expect(source).toContain("<AppShell.Section grow component={ScrollArea}>");
  expect(source.indexOf('to="/logs/settings"')).toBeGreaterThan(source.indexOf("</AppShell.Section>"));
});

it("gives management content its own vertical scrollbar", () => {
  const source = readFileSync(resolve(process.cwd(), "src/pages/Logs.tsx"), "utf8");
  const css = readFileSync(resolve(process.cwd(), "src/pages/Logs.css"), "utf8");
  expect(source).toContain('<AppShell.Main className="log-main">');
  expect(css).toMatch(/\.log-window\s*\{[^}]*height:\s*100vh;[^}]*overflow:\s*hidden;/s);
  expect(css).toMatch(/\.log-main\s*\{[^}]*height:\s*100vh;[^}]*overflow-y:\s*auto;/s);
});
```

- [ ] **Step 2: Run the layout tests and verify RED**

Run:

```powershell
npm test -- --run src/pages/Logs.test.tsx
```

Expected: FAIL because the growing section is not a `ScrollArea`, `AppShell.Main` has no scroll class, and `.log-window` is not viewport-bounded.

- [ ] **Step 3: Implement the scroll boundaries**

Import `ScrollArea` from Mantine. Change only the growing section and main element:

```tsx
<AppShell.Section grow component={ScrollArea}>
  {/* navigation and repeat-quest control */}
</AppShell.Section>
<AppShell.Section>
  {/* fixed settings navigation */}
</AppShell.Section>

<AppShell.Main className="log-main">
  <Outlet />
</AppShell.Main>
```

Update `Logs.css`:

```css
.log-window {
  height: 100vh;
  overflow: hidden;
  background-color: #252525;
}

.log-main {
  height: 100vh;
  overflow-y: auto;
}
```

- [ ] **Step 4: Run focused tests and verify GREEN**

Run:

```powershell
npm test -- --run src/pages/Logs.test.tsx src/pages/Logs.repeatQuest.test.tsx src/pages/Settings.localization.test.ts
npm run tsc
npm run format-check
npm run lint
```

Expected: all focused tests pass and TypeScript, Prettier, and ESLint exit 0.

- [ ] **Step 5: Commit the scroll layout**

```powershell
git add -- src/pages/Logs.tsx src/pages/Logs.css src/pages/Logs.test.tsx
git commit -m "fix: keep management navigation reachable"
```

---

### Task 3: Verify real window sizes and repeat-quest operation

**Files:**
- Modify only if personally observed: `docs/testing/game-2.0.2-smoke-test.md`

**Interfaces:**
- Consumes: completed sidebar and scroll behavior from Tasks 1 and 2.
- Produces: automated regression evidence and manual 800×600/short-window observations.

- [ ] **Step 1: Run the complete frontend and Rust regression**

```powershell
npm test -- --run
npm run build
cargo test --workspace --all-targets --locked
```

Expected: every command exits 0. Existing unrelated inventory warnings may remain, but no new repeat-quest or layout failures are permitted.

- [ ] **Step 2: Verify the default management window**

Run `npm run tauri dev` normally with the pinned 2.0.2 game executable. At 800×600, verify:

- `무한 퀘스트 반복` is visible immediately below `데미지 미터`;
- `설정` remains visible at the sidebar bottom;
- Settings content scrolls to its last field;
- the repeat switch starts OFF and can be enabled and restored OFF.

- [ ] **Step 3: Verify a shorter management window**

Resize the management window shorter than 600 logical pixels. Verify the growing navigation section scrolls independently if needed, `설정` remains fixed at the bottom, and the main content scrollbar remains usable.

- [ ] **Step 4: Verify normal-exit restoration**

Enable `무한 퀘스트 반복`, exit through the tray menu, restart Djeeta MOD while the game remains open, and confirm the sidebar switch reports OFF. Do not record compatibility unless the result is personally observed.

- [ ] **Step 5: Record only observed smoke-test evidence**

If all live checks were personally observed, append dated rows to `docs/testing/game-2.0.2-smoke-test.md`. If any check was not performed, leave it unchecked and do not create a documentation commit.

- [ ] **Step 6: Review scope before handoff**

```powershell
git diff --check
git status --short
git log -5 --oneline
```

Confirm that no unrelated inventory files or `logs.db` were staged, and do not claim game 2.0.2 compatibility unless every required manual scenario passed.

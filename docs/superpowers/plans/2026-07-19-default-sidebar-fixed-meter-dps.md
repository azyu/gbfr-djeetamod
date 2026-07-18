# Default Sidebar, Fixed Meter, and DPS Label Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Open the desktop management sidebar by default, keep the compact meter at its scaled four-row size without scrollbars, and label Korean damage-per-second views as `DPS`.

**Architecture:** Preserve the existing Mantine layout and Tauri two-window architecture. Change only initial layout state, meter window policy and root overflow behavior, and one Korean translation value; reuse the existing `MeterGeometry` calculation for scaled size enforcement.

**Tech Stack:** React 18, TypeScript, Mantine 7, Zustand, Vitest/Testing Library, Tauri 1, Rust, WiX.

## Global Constraints

- Desktop sidebar starts expanded; mobile sidebar remains collapsed.
- Sidebar state is not persisted.
- Meter reference geometry remains `330x145` at `x45/y470` for 1920x1080 and scales within `0.75..=1.5`.
- Meter remains always-on-top, omitted from the taskbar, draggable from its header, and click-through starts disabled.
- Meter renders at most four party rows and never exposes document scrollbars.
- Korean `ui.logs.damage-per-second` is `DPS`; other languages are unchanged.
- Use the project toolchain: Node.js 20 and `nightly-2024-05-04` from `rust-toolchain.toml`.
- Do not modify or stage the runtime `logs.db` file.

---

### Task 1: Default-open desktop sidebar

**Files:**
- Modify: `src/pages/Logs.tsx`
- Test: `src/pages/Logs.test.tsx`

**Interfaces:**
- Consumes: Mantine `useDisclosure(initialState)` and `AppShell.navbar.collapsed`.
- Produces: `desktopOpened === true` on first render while `mobileOpened === false`.

- [ ] **Step 1: Write the failing layout test**

Add `readFileSync` and `resolve` imports to `Logs.test.tsx`, then assert the two explicit initial states:

```tsx
it("starts with mobile navigation closed and desktop navigation open", () => {
  const source = readFileSync(resolve(process.cwd(), "src/pages/Logs.tsx"), "utf8");
  expect(source).toMatch(/mobileOpened[\s\S]*useDisclosure\(\)/);
  expect(source).toMatch(/desktopOpened[\s\S]*useDisclosure\(true\)/);
});
```

- [ ] **Step 2: Run the focused test and verify RED**

Run: `npm test -- --run src/pages/Logs.test.tsx`

Expected: FAIL because the recorded values are `[false, false]`.

- [ ] **Step 3: Implement the minimal initial-state change**

In `src/pages/Logs.tsx`, change only the desktop disclosure initializer:

```tsx
const [desktopOpened, { toggle: toggleDesktop }] = useDisclosure(true);
```

- [ ] **Step 4: Run the focused test and verify GREEN**

Run: `npm test -- --run src/pages/Logs.test.tsx`

Expected: all `Logs.test.tsx` tests PASS.

- [ ] **Step 5: Commit**

```powershell
git add -- src/pages/Logs.tsx src/pages/Logs.test.tsx
git commit -m "feat: open management sidebar by default"
```

---

### Task 2: Fixed four-row meter without scrollbars

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/src/main.rs`
- Modify: `src/styles.css`
- Modify: `src/components/compact-meter/CompactDamageMeter.css`
- Modify: `src/pages/Meter.tsx`
- Test: `src/pages/WindowConfiguration.test.ts`
- Create: `src/pages/Meter.test.tsx`

**Interfaces:**
- Consumes: `meter_geometry(screen_width, screen_height) -> MeterGeometry` and `CompactDamageMeter`.
- Produces: `set_meter_size(window: &tauri::Window) -> Result<(), String>` and an always-rendered meter header.

- [ ] **Step 1: Write failing meter policy tests**

Extend `WindowConfiguration.test.ts`:

```ts
it("fixes the meter to its scaled four-row size", () => {
  const config = JSON.parse(readFileSync(resolve(process.cwd(), "src-tauri/tauri.conf.json"), "utf8"));
  const meter = config.tauri.windows.find((window: { label: string }) => window.label === "main");
  const backend = readFileSync(resolve(process.cwd(), "src-tauri/src/main.rs"), "utf8");
  const styles = readFileSync(resolve(process.cwd(), "src/styles.css"), "utf8");

  expect(meter.resizable).toBe(false);
  expect(meter.width).toBe(330);
  expect(meter.height).toBe(145);
  expect(backend).toContain("set_meter_size(&window)?;");
  expect(styles).toMatch(/html,\s*body,\s*#root[\s\S]*overflow:\s*hidden/);
});
```

Create `Meter.test.tsx`, mock `useCompactMeter` to return no rows, and assert the title still renders:

```tsx
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => (key === "ui.compact-meter.title" ? "파티 데미지" : key),
  }),
}));

vi.mock("./useCompactMeter", () => ({
  default: () => ({ rows: [], transparency: 0.5 }),
}));

it("keeps the draggable meter header visible while waiting", () => {
  render(<Meter />);
  expect(screen.getByText("파티 데미지")).toBeTruthy();
});
```

- [ ] **Step 2: Run focused tests and verify RED**

Run: `npm test -- --run src/pages/WindowConfiguration.test.ts src/pages/Meter.test.tsx`

Expected: FAIL because the window remains resizable, startup does not enforce size, root overflow is not hidden, and `Meter` returns `null` for empty rows.

- [ ] **Step 3: Implement fixed window and content policy**

Set the main window's `resizable` field to `false` in `tauri.conf.json`.

Add a size-only helper in `main.rs` that preserves the dragged position:

```rust
fn set_meter_size(window: &tauri::Window) -> Result<(), String> {
    let monitor = window
        .current_monitor()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No monitor available for the meter window".to_string())?;
    let screen = monitor.size().to_logical::<f64>(monitor.scale_factor());
    let geometry = meter_geometry(screen.width, screen.height);
    window
        .set_size(Size::Logical(LogicalSize {
            width: geometry.width,
            height: geometry.height,
        }))
        .map_err(|error| error.to_string())
}
```

Call `set_meter_size(&window)?;` in Tauri setup before applying click-through. Reuse the helper from `reset_meter_geometry` so reset still restores position and size.

In `styles.css` add:

```css
html,
body,
#root {
  width: 100%;
  height: 100%;
  overflow: hidden;
}
```

In `CompactDamageMeter.css`, add `box-sizing: border-box` and `height: 100%` to `.compact-meter`. In `Meter.tsx`, remove the empty-row early return and always render `CompactDamageMeter`.

- [ ] **Step 4: Run focused frontend and Rust tests and verify GREEN**

Run:

```powershell
npm test -- --run src/pages/WindowConfiguration.test.ts src/pages/Meter.test.tsx src/components/compact-meter/CompactDamageMeter.test.tsx
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --locked --package gbfr-logs --bin gbfr-logs
```

Expected: frontend focused tests PASS and 38 backend tests PASS.

- [ ] **Step 5: Commit**

```powershell
git add -- src-tauri/tauri.conf.json src-tauri/src/main.rs src/styles.css src/components/compact-meter/CompactDamageMeter.css src/pages/Meter.tsx src/pages/Meter.test.tsx src/pages/WindowConfiguration.test.ts
git commit -m "feat: fix compact meter geometry"
```

---

### Task 3: Rename Korean damage-per-second label

**Files:**
- Modify: `src-tauri/lang/ko/ui.json`
- Test: `src/pages/Settings.localization.test.ts`

**Interfaces:**
- Consumes: `ui.logs.damage-per-second` through the existing i18next resource loader.
- Produces: Korean display value `DPS` for graph titles, legends, and associated labels.

- [ ] **Step 1: Write the failing localization test**

Add to `Settings.localization.test.ts`:

```ts
it("uses DPS for the Korean damage-per-second graph label", () => {
  const ko = readJson("ko/ui.json");
  expect(ko.logs["damage-per-second"]).toBe("DPS");
  expect(JSON.stringify(ko)).not.toContain("초당 메디지");
});
```

- [ ] **Step 2: Run the focused test and verify RED**

Run: `npm test -- --run src/pages/Settings.localization.test.ts`

Expected: FAIL because the current Korean value is `초당 메디지`.

- [ ] **Step 3: Change only the Korean translation value**

In `src-tauri/lang/ko/ui.json`:

```json
"damage-per-second": "DPS"
```

- [ ] **Step 4: Run the focused test and verify GREEN**

Run: `npm test -- --run src/pages/Settings.localization.test.ts`

Expected: all localization tests PASS.

- [ ] **Step 5: Commit**

```powershell
git add -- src-tauri/lang/ko/ui.json src/pages/Settings.localization.test.ts
git commit -m "fix: label Korean damage graph as DPS"
```

---

### Task 4: Full verification, packaging, and hashes

**Files:**
- Modify generated resource: `src-tauri/hook.dll`
- Modify: `README.md` only if its recorded artifact hash changes
- Modify: `docs/testing/game-2.0.2-smoke-test.md` only with results actually observed

**Interfaces:**
- Consumes: all prior tasks and the required verification contract in `AGENTS.md`.
- Produces: verified frontend/Rust artifacts, MSI, and matching bundled hook hash.

- [ ] **Step 1: Run all frontend gates**

```powershell
npm ci
npm run format-check
npm run lint
npm run tsc
npm test -- --run
npm run build
```

Expected: every command exits 0; the Vite large-chunk warning may remain.

- [ ] **Step 2: Run all Rust gates**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" build --release --locked --package hook
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --workspace --all-targets --locked
```

Expected: both commands exit 0; existing disabled-hook dead-code warnings may remain.

- [ ] **Step 3: Synchronize the packaged hook**

After the game process releases the existing DLL:

```powershell
Copy-Item -LiteralPath target/release/hook.dll -Destination src-tauri/hook.dll -Force
```

Expected: `Get-FileHash` reports identical SHA-256 values for both DLLs.

- [ ] **Step 4: Build the MSI**

Run: `npm run tauri build -- --bundles msi`

Expected: exit 0 and an MSI under `target/release/bundle/msi/`.

- [ ] **Step 5: Record only verified artifact results**

Compute SHA-256 for `target/release/hook.dll`, `src-tauri/hook.dll`, and the MSI. Update `README.md` and `docs/testing/game-2.0.2-smoke-test.md` only when their existing artifact sections require the new values. Do not mark game 2.0.2 compatibility complete unless the manual checklist is fully satisfied.

- [ ] **Step 6: Run final repository checks and commit artifact records**

```powershell
git diff --check
git status --short
```

Expected: no whitespace errors and `logs.db` remains untracked and unstaged.

# Development Safety and Packaging Automation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent equipment-analysis contract regressions and blank windows, automate the complete local MSI workflow, and align the manual game checklist with current behavior.

**Architecture:** A single JSON fixture is verified by Rust serialization tests and consumed through a handwritten TypeScript normalizer. A top-level React error boundary provides last-resort recovery. A PowerShell module contains testable pure helpers while one orchestration script runs every required build gate, packages the MSI, verifies the hook, and updates artifact hashes.

**Tech Stack:** Rust/Serde, TypeScript/React 18, Vitest/Testing Library, PowerShell 5.1+, npm/Vite, Cargo, Tauri 1/WiX.

## Global Constraints

- Work on `codex/development-safety-automation`, not `master`.
- Target Granblue Fantasy: Relink Endless Ragnarok 2.0.2 on Windows x64.
- Node 20 is supported; versions below 20 fail, and newer versions emit a warning.
- Add no runtime or test dependencies.
- Do not stop the game automatically, inject a DLL manually, stage changes, commit from scripts, push, delete files, or modify `logs.db`.
- Do not claim game 2.0.2 compatibility or check manual smoke-test boxes automatically.
- Preserve the existing append-only bincode protocol order.

---

### Task 1: Establish the shared equipment-analysis contract

**Files:**
- Create: `src/fixtures/equipment-analysis-response.json`
- Create: `src/equipmentAnalysisContract.ts`
- Create: `src/equipmentAnalysisContract.test.ts`
- Modify: `src-tauri/src/equipment/mod.rs`
- Modify: `src/pages/EquipmentAnalysis.tsx`
- Modify: `src/pages/EquipmentAnalysis.test.tsx`

**Interfaces:**
- Produces: `normalizeEquipmentAnalysisResponse(value: unknown): EquipmentAnalysisResponse`.
- Consumes: `EquipmentAnalysisResponse` and nested types from `src/types.ts`.
- Contract fixture: one Narmaya response with trait ID `3696775008` (`0xDC584F60`), total 70, max 65, overflow 5.

- [ ] **Step 1: Add a Rust contract test before the fixture exists**

In `src-tauri/src/equipment/mod.rs`, add a test named `equipment_response_matches_the_frontend_contract_fixture`. It must construct an `EquipmentState::for_test([(0xDC58_4F60, 65)])`, apply a complete Narmaya snapshot using character key `0xE705_3919`, and compare `serde_json::to_value(state.response())` to this include:

```rust
let expected: serde_json::Value = serde_json::from_str(include_str!(
    "../../../src/fixtures/equipment-analysis-response.json"
))
.unwrap();
assert_eq!(serde_json::to_value(state.response()).unwrap(), expected);
```

Use five sources in order with levels 15, 15, 11, 15, and 14; alternate `SigilPrimary` and `SigilSecondary`, use slots 0 through 4 and item IDs 1 through 5.

- [ ] **Step 2: Run the Rust test to verify RED**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --locked --package gbfr-logs equipment_response_matches_the_frontend_contract_fixture
```

Expected: compilation fails because `src/fixtures/equipment-analysis-response.json` does not exist.

- [ ] **Step 3: Add the exact shared fixture**

Create `src/fixtures/equipment-analysis-response.json`:

```json
{
  "connected": true,
  "characters": [
    {
      "characterType": "Pl1400",
      "status": "complete",
      "traits": [
        {
          "traitId": 3696775008,
          "totalLevel": 70,
          "maxLevel": 65,
          "overflowLevel": 5,
          "state": "overflow",
          "sources": [
            { "kind": "sigilPrimary", "slot": 0, "itemId": 1, "traitId": 3696775008, "traitLevel": 15 },
            { "kind": "sigilSecondary", "slot": 1, "itemId": 2, "traitId": 3696775008, "traitLevel": 15 },
            { "kind": "sigilPrimary", "slot": 2, "itemId": 3, "traitId": 3696775008, "traitLevel": 11 },
            { "kind": "sigilSecondary", "slot": 3, "itemId": 4, "traitId": 3696775008, "traitLevel": 15 },
            { "kind": "sigilPrimary", "slot": 4, "itemId": 5, "traitId": 3696775008, "traitLevel": 14 }
          ]
        }
      ]
    }
  ]
}
```

- [ ] **Step 4: Run the Rust contract test to verify GREEN**

Run the command from Step 2.

Expected: one matching test passes and the serialized Rust response exactly equals the fixture.

- [ ] **Step 5: Write failing TypeScript normalizer tests**

Create `src/equipmentAnalysisContract.test.ts`. Import the JSON fixture as `unknown` and assert:

```ts
expect(normalizeEquipmentAnalysisResponse(fixture)).toEqual(fixture);
expect(normalizeEquipmentAnalysisResponse(null)).toEqual({ connected: false, characters: [] });
```

Add a malformed response whose trait is valid but whose `sources` contain one snake-case legacy object and one `null`; assert that the trait remains, `sources` becomes `[]`, and no exception is thrown.

- [ ] **Step 6: Run the TypeScript contract test to verify RED**

Run:

```powershell
npm test -- --run src/equipmentAnalysisContract.test.ts
```

Expected: FAIL because `normalizeEquipmentAnalysisResponse` does not exist.

- [ ] **Step 7: Implement the minimal normalizer**

Create `src/equipmentAnalysisContract.ts` with:

```ts
export const normalizeEquipmentAnalysisResponse = (value: unknown): EquipmentAnalysisResponse => {
  if (!isRecord(value)) return { connected: false, characters: [] };
  return {
    connected: value.connected === true,
    characters: Array.isArray(value.characters) ? value.characters.flatMap(normalizeCharacter) : [],
  };
};
```

Add private guards for records, non-negative integers, `CharacterType` strings or `{ Unknown: number }`, character statuses, trait states, and source kinds. `normalizeCharacter`, `normalizeTrait`, and `normalizeSource` return zero or one element so `flatMap` drops malformed entries. A valid trait survives malformed sources with only valid sources retained. `maxLevel` accepts a non-negative integer or `null`; every other numeric field must be a non-negative integer.

- [ ] **Step 8: Normalize every Tauri payload before storing it**

In `src/pages/EquipmentAnalysis.tsx`:

- Import `normalizeEquipmentAnalysisResponse`.
- Change the initial invoke and listener payload types to `unknown`.
- Pass both payloads through the normalizer before `loadResponse`.
- Catch initial invoke rejection so it does not become an unhandled promise rejection.
- Delete `LegacyEquippedTraitSource`, `sourceTraitLevel`, and snake-case fallback logic.
- Render `source.itemId` and `source.traitLevel` directly because only normalized sources reach the table.

- [ ] **Step 9: Render the shared fixture and malformed payload in the page tests**

Update `src/pages/EquipmentAnalysis.test.tsx` so the Tauri mock stores `unknown`, imports the fixture, and uses it for the overflow test. Assert `70 / 65`, `5 초과`, Narmaya, and at least one `+15` source. Keep a malformed snake-case source test and assert the trait renders without throwing while the malformed source is omitted.

- [ ] **Step 10: Run focused frontend and Rust tests**

Run:

```powershell
npm test -- --run src/equipmentAnalysisContract.test.ts src/pages/EquipmentAnalysis.test.tsx
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --locked --package gbfr-logs equipment_response_matches_the_frontend_contract_fixture
```

Expected: all focused tests pass.

- [ ] **Step 11: Commit the contract boundary**

```powershell
git add -- src/fixtures/equipment-analysis-response.json src/equipmentAnalysisContract.ts src/equipmentAnalysisContract.test.ts src-tauri/src/equipment/mod.rs src/pages/EquipmentAnalysis.tsx src/pages/EquipmentAnalysis.test.tsx
git commit -m "test: enforce equipment analysis contract"
```

### Task 2: Add a top-level React error boundary

**Files:**
- Create: `src/components/AppErrorBoundary.tsx`
- Create: `src/components/AppErrorBoundary.test.tsx`
- Modify: `src/main.tsx`

**Interfaces:**
- Produces: `AppErrorBoundary` with props `{ children: ReactNode; onReload?: () => void }`.
- Default recovery action: `window.location.reload()`.

- [ ] **Step 1: Write the failing boundary test**

Create `src/components/AppErrorBoundary.test.tsx`. Render a child component that throws `new Error("render failed")` inside the boundary with `onReload={reload}`. Suppress the expected `console.error` during the test. Assert the fallback contains `Djeeta MOD`, `화면을 표시할 수 없습니다`, and a `다시 불러오기` button; clicking it calls `reload` once.

- [ ] **Step 2: Run the test to verify RED**

Run:

```powershell
npm test -- --run src/components/AppErrorBoundary.test.tsx
```

Expected: FAIL because the component does not exist.

- [ ] **Step 3: Implement the boundary**

Implement a React class component with `getDerivedStateFromError()` setting `{ hasError: true }`. The fallback uses semantic HTML and a plain button. `onReload` is optional; its default calls `window.location.reload()`. Do not expose the exception or stack trace in the user-facing fallback.

- [ ] **Step 4: Wrap the whole application**

In `src/main.tsx`, place `<App />` inside `<AppErrorBoundary>` while keeping the existing Mantine and modals providers outside it.

- [ ] **Step 5: Run the focused test and type-check**

```powershell
npm test -- --run src/components/AppErrorBoundary.test.tsx
npm run tsc
```

Expected: the boundary test and type-check pass.

- [ ] **Step 6: Commit the boundary**

```powershell
git add -- src/components/AppErrorBoundary.tsx src/components/AppErrorBoundary.test.tsx src/main.tsx
git commit -m "feat: recover from management render failures"
```

### Task 3: Add testable PowerShell packaging automation

**Files:**
- Create: `scripts/PackageHelpers.psm1`
- Create: `scripts/package.ps1`
- Create: `scripts/tests/PackageHelpers.Tests.ps1`
- Modify: `package.json`

**Interfaces:**
- `Get-NodeMajorVersion -Version <string> -> int`
- `Assert-SupportedNodeVersion -Version <string> -> void`
- `Assert-GameNotRunning -Processes <object[]> -> void`
- `Set-ArtifactHashesInText -Text <string> -MsiHash <64 hex> -HookHash <64 hex> -> string`
- `Invoke-NativeCommand -FilePath <string> -Arguments <string[]> -> void`

- [ ] **Step 1: Write helper tests before the module exists**

Create `scripts/tests/PackageHelpers.Tests.ps1` with `$ErrorActionPreference = 'Stop'`, import `../PackageHelpers.psm1`, and local `Assert-Equal` and `Assert-Throws` functions. Test:

- `v20.11.1` parses to 20.
- Node 19 throws.
- Node 20 succeeds.
- Node 24 succeeds while emitting a warning.
- A non-empty fake game-process array throws; an empty array succeeds.
- README-style and smoke-test-style MSI and hook hashes are each replaced exactly once.
- Missing or duplicate hash labels throw instead of silently changing unexpected text.

Print `Package helper tests passed.` only after every assertion succeeds.

- [ ] **Step 2: Run helper tests to verify RED**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/tests/PackageHelpers.Tests.ps1
```

Expected: FAIL because `PackageHelpers.psm1` does not exist.

- [ ] **Step 3: Implement the helper module**

Create `scripts/PackageHelpers.psm1` and export the five functions listed above. Requirements:

- Version parsing accepts an optional leading `v` and requires a numeric major.
- Versions below 20 throw; versions above 20 use `Write-Warning`.
- Game preflight accepts injected process objects for deterministic tests.
- Hash replacement validates both inputs against `^[A-Fa-f0-9]{64}$`, uppercases them, requires exactly one MSI label and one ``hook.dll`` label, and preserves all other text.
- Native invocation uses `& $FilePath @Arguments` and throws when `$LASTEXITCODE` is non-zero.

- [ ] **Step 4: Run helper tests to verify GREEN**

Run the command from Step 2.

Expected: exit 0 and `Package helper tests passed.`.

- [ ] **Step 5: Add npm entry points**

Add to `package.json`:

```json
"test:package-helpers": "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/tests/PackageHelpers.Tests.ps1",
"package:msi": "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package.ps1"
```

- [ ] **Step 6: Implement `scripts/package.ps1`**

The script must:

1. Enable strict mode and terminating errors.
2. Resolve the repository root with `Resolve-Path (Join-Path $PSScriptRoot '..')` and `Push-Location` inside `try/finally`.
3. Require `$env:OS -eq 'Windows_NT'`, `package.json`, `src-tauri/tauri.conf.json`, and both hash documents.
4. Call `Assert-GameNotRunning` with `Get-Process -Name granblue_fantasy_relink -ErrorAction SilentlyContinue` before builds.
5. Resolve `node.exe`, `npm.cmd`, Cargo from `Get-Command` or `$env:USERPROFILE\.cargo\bin\cargo.exe`, and `git.exe`; add Cargo's directory to the process `PATH`.
6. Validate `node --version`.
7. Run helper tests, every required npm gate, the locked release hook build, and locked workspace tests through `Invoke-NativeCommand`.
8. Copy the release hook, build only the configured application binary, require exactly one freshly produced Djeeta MOD MSI matching the configured product and version, calculate hashes, and require hook equality.
9. Update `README.md` and `docs/testing/game-2.0.2-smoke-test.md` with `Set-ArtifactHashesInText`, writing UTF-8 without BOM via `System.Text.UTF8Encoding($false)`.
10. Run `git diff --check` and print a final `PSCustomObject` containing `MsiPath`, `MsiSHA256`, `HookSHA256`, and `HookHashesEqual`.

Use the exact required gate order from `AGENTS.md`. The script must not catch and downgrade failures.

- [ ] **Step 7: Check script syntax without packaging**

Run:

```powershell
$errors = $null
[System.Management.Automation.Language.Parser]::ParseFile(
  (Resolve-Path scripts/package.ps1),
  [ref]$null,
  [ref]$errors
) | Out-Null
if ($errors.Count -ne 0) { $errors | Format-List | Out-String | Write-Error }
npm run test:package-helpers
```

Expected: no parser errors and helper tests pass.

- [ ] **Step 8: Commit the packaging automation**

```powershell
git add -- package.json scripts/PackageHelpers.psm1 scripts/package.ps1 scripts/tests/PackageHelpers.Tests.ps1
git commit -m "build: automate verified MSI packaging"
```

### Task 4: Align the manual smoke test with current behavior

**Files:**
- Modify: `docs/testing/game-2.0.2-smoke-test.md`
- Modify: `README.md`

**Interfaces:**
- Consumes: Current window policy and shared Narmaya contract fixture.
- Produces: Manual checks that remain unchecked until a real offline/private game run.

- [ ] **Step 1: Verify the obsolete scenario is present**

```powershell
rg -n "클릭 통과 전환|이동·크기 변경" docs/testing/game-2.0.2-smoke-test.md
```

Expected: the current obsolete checklist row is printed.

- [ ] **Step 2: Replace and extend the checklist**

Remove the click-through/resize row. Add unchecked rows for:

- Damage Meter switch hides and restores the meter.
- Dragging `파티 데미지` moves the fixed-size meter and no scrollbar appears.
- Only `Djeeta MOD` management appears on the taskbar.
- The meter stays always on top and management Always on Top defaults off.
- Opening Narmaya in equipment formation produces Damage Cap `70 / 65` and `5 초과`.

Update the geometry row to say the meter is fixed at the scaled four-row size rather than resizable. Add a short README source-build note that `npm run package:msi` performs the complete verified local package workflow and updates hashes; retain the individual commands for transparency.

- [ ] **Step 3: Verify no manual check was marked complete**

```powershell
if (Select-String -Path docs/testing/game-2.0.2-smoke-test.md -Pattern '\| \[x\] \|' -CaseSensitive:$false) {
  throw 'Manual smoke checks must remain unchecked.'
}
if (Select-String -Path docs/testing/game-2.0.2-smoke-test.md -Pattern '클릭 통과 전환|이동·크기 변경') {
  throw 'Obsolete window behavior remains.'
}
git diff --check -- README.md docs/testing/game-2.0.2-smoke-test.md
```

Expected: exit 0 with no completed manual checks, obsolete behavior, or whitespace errors.

- [ ] **Step 4: Commit the smoke-test update**

```powershell
git add -- README.md docs/testing/game-2.0.2-smoke-test.md
git commit -m "docs: align game smoke test with window policy"
```

### Task 5: Execute the complete package workflow and verify final state

**Files:**
- Modify automatically if hashes change: `README.md`
- Modify automatically if hashes change: `docs/testing/game-2.0.2-smoke-test.md`
- Do not modify or commit: `logs.db`

**Interfaces:**
- Consumes: Tasks 1–4 and a stopped game process.
- Produces: one fresh Djeeta MOD MSI, equal hook hashes, current hash records, and full verification evidence.

- [ ] **Step 1: Run the complete package command**

```powershell
npm run package:msi
```

Expected: every required npm and Cargo gate passes, exactly one fresh Djeeta MOD MSI is produced, hook hashes are equal, and the command prints the final hashes. If the game is running, stop here and ask the user to exit it; do not stop it automatically.

- [ ] **Step 2: Verify the artifact and documentation values independently**

Calculate the configured Djeeta MOD product MSI hash and both hook hashes with `Get-FileHash`. Assert the hook hashes are equal and that README plus the smoke-test document each contain the current MSI and hook hashes.

- [ ] **Step 3: Inspect the complete diff and status**

```powershell
git diff --check
git status --short
git diff --stat master...HEAD
```

Expected: no whitespace errors, only intentional project files differ from `master`, and `logs.db` remains the only unrelated untracked file.

- [ ] **Step 4: Commit refreshed hashes only if changed**

```powershell
git add -- README.md docs/testing/game-2.0.2-smoke-test.md
git diff --cached --quiet
if ($LASTEXITCODE -ne 0) { git commit -m "docs: refresh automated package hashes" }
```

- [ ] **Step 5: Run final focused verification after the final commit**

```powershell
npm run lint
npm run tsc
npm test -- --run
npm run build
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --workspace --all-targets --locked
npm run test:package-helpers
```

Expected: lint and type-check exit 0,  all frontend and Rust tests pass, the production build succeeds, and helper tests pass.

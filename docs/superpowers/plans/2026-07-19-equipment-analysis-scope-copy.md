# Equipment Analysis Scope Copy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the equipment-analysis screen explicitly identify both the twelve equipped sigils included in totals and the weapon, wrightstone, summons, and master traits that remain excluded.

**Architecture:** Keep the existing `EquipmentAnalysis` component and translation key unchanged. Protect the approved copy with a focused test that reads the packaged Korean and English locale JSON files directly, then change only those two translation values.

**Tech Stack:** TypeScript, Vitest, JSON localization resources, Tauri 1 packaging

## Global Constraints

- Korean copy must be exactly: `현재는 장착 진 12개의 주·보조 특성만 합산합니다. 무기·가호석·소환석·마스터 특성은 아직 포함되지 않습니다.`
- English copy must be exactly: `Currently, only primary and secondary traits from the 12 equipped sigils are totaled. Weapon and wrightstone, summons, and master traits are not included yet.`
- Do not change equipment capture, trait totals, component placement, or styling.
- Keep `logs.db` untracked and untouched.

---

### Task 1: Protect and update the equipment-analysis scope copy

**Files:**
- Create: `src/pages/EquipmentAnalysis.localization.test.ts`
- Modify: `src-tauri/lang/ko/ui.json:20`
- Modify: `src-tauri/lang/en/ui.json:20`

**Interfaces:**
- Consumes: the existing `ui.equipment-analysis.scope` key loaded by `EquipmentAnalysis.tsx`.
- Produces: exact Korean and English strings through the same translation key; no API or type changes.

- [ ] **Step 1: Write the failing localization test**

Create `src/pages/EquipmentAnalysis.localization.test.ts`:

```ts
import { readFileSync } from "node:fs";
import { expect, it } from "vitest";

const readScope = (language: "ko" | "en") => {
  const path = new URL(`../../src-tauri/lang/${language}/ui.json`, import.meta.url);
  const locale = JSON.parse(readFileSync(path, "utf8")) as {
    ui: { "equipment-analysis": { scope: string } };
  };

  return locale.ui["equipment-analysis"].scope;
};

it("explains the included and excluded equipment-analysis sources in both languages", () => {
  expect(readScope("ko")).toBe(
    "현재는 장착 진 12개의 주·보조 특성만 합산합니다. 무기·가호석·소환석·마스터 특성은 아직 포함되지 않습니다.",
  );
  expect(readScope("en")).toBe(
    "Currently, only primary and secondary traits from the 12 equipped sigils are totaled. Weapon and wrightstone, summons, and master traits are not included yet.",
  );
});
```

- [ ] **Step 2: Run the focused test and verify RED**

Run: `npm test -- --run src/pages/EquipmentAnalysis.localization.test.ts`

Expected: FAIL because both locale files still contain the shorter copy that does not name excluded sources.

- [ ] **Step 3: Apply the minimal translation changes**

Set `src-tauri/lang/ko/ui.json`:

```json
"scope": "현재는 장착 진 12개의 주·보조 특성만 합산합니다. 무기·가호석·소환석·마스터 특성은 아직 포함되지 않습니다."
```

Set `src-tauri/lang/en/ui.json`:

```json
"scope": "Currently, only primary and secondary traits from the 12 equipped sigils are totaled. Weapon and wrightstone, summons, and master traits are not included yet."
```

- [ ] **Step 4: Run the focused test and verify GREEN**

Run: `npm test -- --run src/pages/EquipmentAnalysis.localization.test.ts`

Expected: one test file and one test pass.

- [ ] **Step 5: Commit the tested copy change**

```powershell
git add -- src/pages/EquipmentAnalysis.localization.test.ts src-tauri/lang/ko/ui.json src-tauri/lang/en/ui.json
git commit -m "fix: clarify equipment analysis scope"
```

---

### Task 2: Run complete release verification and refresh the MSI

**Files:**
- Modify if generated hash changes: `README.md`
- Modify if generated hash changes: `docs/testing/game-2.0.2-smoke-test.md`
- Verify: `target/release/hook.dll`
- Verify: `src-tauri/hook.dll`
- Generate: `target/release/bundle/msi/Djeeta MOD_0.1.0_x64_en-US.msi`

**Interfaces:**
- Consumes: the tested locale resources from Task 1 and the repository package helper.
- Produces: a verified MSI plus matching release/bundled hook hashes and current documented package hashes.

- [ ] **Step 1: Run the authoritative package workflow**

Run: `npm run package:msi`

Expected: format, lint, TypeScript, frontend tests, Vite build, release hook build, Rust workspace tests, Tauri MSI build, and hook hash equality all succeed.

- [ ] **Step 2: Independently verify hashes and repository scope**

```powershell
$releaseHash = (Get-FileHash -Algorithm SHA256 -LiteralPath 'target/release/hook.dll').Hash
$bundledHash = (Get-FileHash -Algorithm SHA256 -LiteralPath 'src-tauri/hook.dll').Hash
$msiHash = (Get-FileHash -Algorithm SHA256 -LiteralPath 'target/release/bundle/msi/Djeeta MOD_0.1.0_x64_en-US.msi').Hash
if ($releaseHash -ne $bundledHash) { throw 'Hook hashes differ.' }
Select-String -Path 'README.md','docs/testing/game-2.0.2-smoke-test.md' -Pattern $releaseHash,$msiHash
git diff --check
git status --short
```

Expected: hook hashes match, both hashes occur in both documents, diff check succeeds, and `logs.db` remains the only unrelated untracked path.

- [ ] **Step 3: Commit generated hash documentation only if changed**

```powershell
git add -- README.md docs/testing/game-2.0.2-smoke-test.md
git diff --cached --quiet
if ($LASTEXITCODE -ne 0) { git commit -m "docs: update packaged build hashes" }
```

- [ ] **Step 4: Report the manual compatibility gate**

State that automated verification passed, provide the MSI path and SHA-256 values, and keep Granblue Fantasy: Relink 2.0.2 compatibility unverified until `docs/testing/game-2.0.2-smoke-test.md` is completed in-game.

# Compact Meter Four-Row Layout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fit all four compact damage-meter player rows inside the existing 330×145 window without reducing text size.

**Architecture:** Preserve the existing four-row data and React rendering paths. Add a static CSS contract regression test, then correct header box sizing and reduce each row by one pixel so the layout fits with margin.

**Tech Stack:** React, CSS, Vitest, Node.js 20

## Global Constraints

- Keep the compact meter window at 330×145.
- Keep row text at 11px and header text at 12px.
- Keep the existing damage ordering and maximum of four rows.
- Do not change backend, hook, protocol, or game compatibility behavior.
- Keep `logs.db` untracked and untouched.

---

### Task 1: Lock and fix the four-row CSS layout

**Files:**
- Create: `src/components/compact-meter/CompactDamageMeter.layout.test.ts`
- Modify: `src/components/compact-meter/CompactDamageMeter.css`

**Interfaces:**
- Consumes: the existing `.compact-meter__header` and `.compact-meter__row` CSS selectors.
- Produces: a 25px border-box header and four 28px rows within the existing 145px window.

- [ ] **Step 1: Add the failing CSS contract test**

Create `src/components/compact-meter/CompactDamageMeter.layout.test.ts`:

```ts
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, it } from "vitest";

it("fits four player rows inside the fixed-height meter", () => {
  const css = readFileSync(resolve(process.cwd(), "src/components/compact-meter/CompactDamageMeter.css"), "utf8");

  expect(css).toMatch(/\.compact-meter__header\s*\{[^}]*box-sizing:\s*border-box;/s);
  expect(css).toMatch(/\.compact-meter__row\s*\{[^}]*height:\s*28px;/s);
});
```

- [ ] **Step 2: Run the test and verify RED**

Run:

```powershell
npm.cmd test -- --run src/components/compact-meter/CompactDamageMeter.layout.test.ts
```

Expected: FAIL because the header lacks `box-sizing: border-box` and the row height is still 29px.

- [ ] **Step 3: Apply the minimal CSS fix**

Modify `src/components/compact-meter/CompactDamageMeter.css`:

```css
.compact-meter__header {
  box-sizing: border-box;
  height: 25px;
  padding: 4px 8px;
  font-size: 12px;
  font-weight: 700;
}

.compact-meter__row {
  position: relative;
  height: 28px;
  background: rgba(255, 255, 255, 0.06);
}
```

- [ ] **Step 4: Run focused tests and verify GREEN**

Run:

```powershell
npm.cmd test -- --run src/components/compact-meter/CompactDamageMeter.layout.test.ts src/components/compact-meter/CompactDamageMeter.test.tsx src/components/compact-meter/compactMeterModel.test.ts src/pages/WindowConfiguration.test.ts
```

Expected: 4 test files pass, including four-row ordering, fixed window geometry, content rendering, and the new CSS contract.

- [ ] **Step 5: Run full frontend verification**

Run:

```powershell
npm.cmd run format-check
npm.cmd run lint
npm.cmd run tsc
npm.cmd test -- --run
npm.cmd run build
git diff --check
```

Expected: formatting, lint, TypeScript, all frontend tests, production build, and whitespace checks pass. Existing non-failing Tauri metadata, React Router future-flag, and Vite chunk-size warnings may remain.

- [ ] **Step 6: Review scope and commit**

Run:

```powershell
git status --short
git diff -- src/components/compact-meter/CompactDamageMeter.css src/components/compact-meter/CompactDamageMeter.layout.test.ts
git add -- src/components/compact-meter/CompactDamageMeter.css src/components/compact-meter/CompactDamageMeter.layout.test.ts
git commit -m "fix: fit four compact meter rows"
```

Expected: the implementation commit contains only the CSS fix and its regression test; `logs.db` remains untracked.


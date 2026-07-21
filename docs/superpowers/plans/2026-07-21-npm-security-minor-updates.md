# NPM Security Minor Updates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce npm audit findings using only updates allowed by the existing major-version ranges.

**Architecture:** Keep npm and `package-lock.json` as the package-management source of truth. Let npm select advisory fixes within declared semver ranges, then verify the resulting dependency graph and frontend toolchain.

**Tech Stack:** npm, React 18, TypeScript, Vite 5, Vitest 1, Tauri 1

## Global Constraints

- Do not use `npm audit fix --force`.
- Do not introduce a direct dependency major-version upgrade.
- Preserve the existing npm/package-lock workflow.
- Do not modify Rust or hook behavior.

---

### Task 1: Apply safe audit fixes

**Files:**
- Modify: `package-lock.json`
- Modify only if npm requires it within existing ranges: `package.json`

**Interfaces:**
- Consumes: the current `package.json` semver ranges and npm advisory database
- Produces: an npm dependency graph containing only non-major direct dependency updates

- [ ] **Step 1: Apply npm's semver-compatible security fixes**

Run: `npm audit fix`

Expected: npm updates the lock file without requesting or applying `--force`.

- [ ] **Step 2: Inspect dependency manifest changes**

Run: `git diff -- package.json package-lock.json`

Expected: no direct dependency crosses its existing major version.

- [ ] **Step 3: Re-run the security audit**

Run: `npm audit --json`

Expected: findings resolvable within the existing semver ranges are removed; major-only fixes may remain.

### Task 2: Verify the frontend toolchain

**Files:**
- Test: `package.json` scripts and the existing frontend test suite

**Interfaces:**
- Consumes: the dependency graph produced by Task 1
- Produces: evidence that formatting, linting, typing, tests, and production build still succeed

- [ ] **Step 1: Check formatting**

Run: `npm run format-check`

Expected: exit code 0.

- [ ] **Step 2: Run lint and type checks**

Run: `npm run lint` and `npm run tsc`

Expected: both commands exit with code 0.

- [ ] **Step 3: Run frontend tests**

Run: `npm test -- --run`

Expected: all Vitest tests pass.

- [ ] **Step 4: Build the frontend**

Run: `npm run build`

Expected: the production bundle completes with exit code 0.

- [ ] **Step 5: Review final scope**

Run: `git status --short` and `git diff --stat`

Expected: only the approved dependency files and these planning documents are changed; pre-existing `logs.db` remains untouched.

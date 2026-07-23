# Node and npm Security Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the EOL Node.js 20 toolchain with Node.js 24 LTS and remove all currently reported npm vulnerabilities without introducing unrelated application upgrades.

**Architecture:** Express Node.js 24 as one repository-wide contract in package metadata, local version selection, packaging validation, documentation, and CI. Upgrade only the Vite/Vitest toolchain required to reach patched versions, preserve Vite 6 to avoid the unrelated Vite 8/Rolldown migration, and make `npm audit` a repeatable CI gate.

**Tech Stack:** Node.js 24, npm 11, Vite 6.4.2, Vitest 4.1.10, GitHub Actions, Windows PowerShell 5, Vitest configuration tests.

## Global Constraints

- Do not modify or stage the user's existing `AGENTS.md` change.
- Keep Tauri 1, React 18, Mantine 7, and application dependencies outside the vulnerable Vite/Vitest chain unchanged.
- Use Node.js major 24 locally, in packaging, and in every JavaScript CI job.
- Use Vite `^6.4.2`, Vitest `^4.1.10`, `@vitejs/plugin-react` `^4.7.0`, and `@types/node` `^24.0.0`.
- Keep release and CI action references pinned to full commit SHAs.
- Keep `nightly-2024-05-04` as the Rust build toolchain; Rust dependency remediation is a separate security stage.
- Commit only after focused tests, `npm audit`, the complete frontend verification suite, and the production build pass.

---

### Task 1: Define and enforce the Node.js 24 toolchain

**Files:**
- Create: `.nvmrc`
- Modify: `package.json`
- Modify: `scripts/PackageHelpers.psm1`
- Modify: `scripts/tests/PackageHelpers.Tests.ps1`
- Modify: `README.md`
- Modify: `.github/workflows/ci.yaml`
- Modify: `.github/workflows/release.yaml`
- Create: `src/toolchainSecurity.test.ts`

**Interfaces:**
- Consumes: `Get-NodeMajorVersion -Version <string>`.
- Produces: `Assert-SupportedNodeVersion` accepting Node 24 and rejecting every other major, plus a repository contract discoverable through `.nvmrc` and `package.json#engines`.

- [ ] **Step 1: Write the failing toolchain contract test**

Create `src/toolchainSecurity.test.ts` that reads repository files and asserts:

```ts
expect(read(".nvmrc").trim()).toBe("24");
expect(packageJson.engines).toEqual({ node: ">=24.0.0 <25" });
expect(packageJson.packageManager).toBe("npm@11.13.0");
expect(helper).toContain("Node.js 24 is required");
expect(read("README.md")).toContain("Node.js 24");
expect(read(".github/workflows/ci.yaml").match(/node-version: 24/g)).toHaveLength(4);
expect(read(".github/workflows/release.yaml")).toContain("node-version: 24");
```

Also assert that CI contains no `actions/checkout@v`, `actions/setup-node@v`, `Swatinem/rust-cache@v`, or `rustup update nightly`.

- [ ] **Step 2: Run the focused tests and verify RED**

Run:

```powershell
npm.cmd test -- --run src/toolchainSecurity.test.ts
npm.cmd run test:package-helpers
```

Expected: the Vitest file fails because `.nvmrc` and Node 24 metadata do not exist; the PowerShell test still treats Node 20 as supported.

- [ ] **Step 3: Implement the Node.js 24 contract**

Add `.nvmrc` containing `24`. Add to `package.json`:

```json
"packageManager": "npm@11.13.0",
"engines": {
  "node": ">=24.0.0 <25"
}
```

Change `Assert-SupportedNodeVersion` to require major 24 and update the helper tests so Node 20 fails and `v24.16.0` passes without warnings. Change README and both GitHub workflows from Node 20 to Node 24.

Pin CI actions to:

```yaml
actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4
actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020 # v4
Swatinem/rust-cache@e18b497796c12c097a38f9edb9d0641fb99eee32 # v2
```

Replace the mutable nightly update with:

```yaml
- run: rustup toolchain install nightly-2024-05-04 --profile minimal
```

- [ ] **Step 4: Run focused tests and verify GREEN**

Run:

```powershell
npm.cmd test -- --run src/toolchainSecurity.test.ts src/releaseWorkflow.test.ts
npm.cmd run test:package-helpers
```

Expected: all focused tests pass under Node 24.

---

### Task 2: Upgrade the vulnerable npm development toolchain

**Files:**
- Modify: `package.json`
- Modify: `package-lock.json`
- Modify: `src/toolchainSecurity.test.ts`

**Interfaces:**
- Consumes: Node.js 24 contract from Task 1.
- Produces: a lockfile with zero `npm audit` vulnerabilities and npm script `audit:npm`.

- [ ] **Step 1: Add failing dependency-security assertions**

Extend `src/toolchainSecurity.test.ts` with:

```ts
expect(packageJson.scripts["audit:npm"]).toBe("npm audit");
expect(packageJson.devDependencies).toMatchObject({
  "@types/node": "^24.0.0",
  "@vitejs/plugin-react": "^4.7.0",
  vite: "^6.4.2",
  vitest: "^4.1.10",
});
expect(read(".github/workflows/ci.yaml")).toContain("npm run audit:npm");
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```powershell
npm.cmd test -- --run src/toolchainSecurity.test.ts
```

Expected: FAIL because the secure dependency versions and audit script are absent.

- [ ] **Step 3: Update only the vulnerable toolchain packages**

Add `"audit:npm": "npm audit"` to `package.json`, update the four exact development dependency ranges listed in Global Constraints, and add `npm run audit:npm` after `npm ci` in the TypeScript CI job.

Regenerate installed dependencies and the lockfile:

```powershell
npm.cmd install
```

- [ ] **Step 4: Verify the audit and focused tests**

Run:

```powershell
npm.cmd audit
npm.cmd test -- --run src/toolchainSecurity.test.ts src/securityConfiguration.test.ts src/releaseWorkflow.test.ts
```

Expected: `npm audit` reports `found 0 vulnerabilities`; focused tests pass.

---

### Task 3: Verify and commit the Node/npm security stage

**Files:**
- Verify all files changed by Tasks 1-2.
- Preserve: `AGENTS.md`

**Interfaces:**
- Consumes: Node.js 24 contract and patched npm lockfile.
- Produces: one verified implementation commit.

- [ ] **Step 1: Run package and frontend verification**

```powershell
npm.cmd run test:package-helpers
npm.cmd run format-check
npm.cmd run lint
npm.cmd run tsc
npm.cmd test -- --run
npm.cmd run build
npm.cmd audit
```

Expected: every command exits 0; Vitest reports all test files and tests passing; npm reports zero vulnerabilities.

- [ ] **Step 2: Inspect dependency resolution and the final diff**

```powershell
npm.cmd ls vite vitest @vitejs/plugin-react @types/node
git diff --check
git status --short
```

Expected: Vite resolves to at least 6.4.2, Vitest to at least 4.1.10, the React plugin to at least 4.7.0, Node types to major 24, and only intended files plus the existing user change to `AGENTS.md` appear.

- [ ] **Step 3: Commit only the implementation files**

```powershell
git add -- .nvmrc package.json package-lock.json scripts/PackageHelpers.psm1 scripts/tests/PackageHelpers.Tests.ps1 README.md .github/workflows/ci.yaml .github/workflows/release.yaml src/toolchainSecurity.test.ts docs/superpowers/plans/2026-07-23-node-npm-security.md
git commit -m "fix: move build tooling to Node 24"
```

Expected: commit succeeds without staging `AGENTS.md`.

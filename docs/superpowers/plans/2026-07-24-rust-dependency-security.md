# Rust Dependency Security Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove all 11 RustSec vulnerabilities from the workspace lockfile and add a repeatable audit gate while preserving Tauri 1 and the existing bincode wire protocol.

**Architecture:** Update only the vulnerable crates and their minimum required parent packages instead of accepting the 324-package result of an unrestricted `cargo update`. Move the pinned nightly to a Rust 1.88-capable date because the patched `time` and `plist` releases require Rust 1.88, update the directly used `dll-syringe` crate to a version compatible with that nightly while disabling unused RPC and cross-bitness features, then run a hash-verified cargo-audit binary against both lockfiles locally and in CI.

**Tech Stack:** Rust nightly-2025-06-27, Cargo, Tauri 1, cargo-audit 0.22.2, Windows PowerShell 5, RustSec advisory database.

## Global Constraints

- Do not modify or stage the user's existing `AGENTS.md` change.
- Preserve Tauri major version 1 and do not perform an unrestricted `cargo update`.
- Preserve `protocol::Message` variant ordering and bincode 1 wire compatibility.
- Keep `bincode = "1.3"` because RUSTSEC-2025-0141 is an unmaintained warning, not a vulnerability; migration requires a separate protocol design and compatibility plan.
- Target zero RustSec vulnerabilities. Existing unmaintained, unsound, and yanked warnings that cannot be removed without a Tauri major or protocol migration must be reported, not silently ignored.
- Use cargo-audit 0.22.2 Windows asset SHA-256 `0a7316540862c13d954f648917ceacca593747baed6eec180fafa590be2710ab`.
- Run the required hook release build and complete workspace Rust test suite before committing.

---

### Task 1: Pin a Rust 1.88-capable nightly and define the audit contract

**Files:**
- Modify: `rust-toolchain.toml`
- Modify: `package.json`
- Create: `scripts/audit-rust.ps1`
- Modify: `.github/workflows/ci.yaml`
- Modify: `src/toolchainSecurity.test.ts`

**Interfaces:**
- Consumes: root `Cargo.lock`, `protocol/Cargo.lock`, and the pinned cargo-audit release asset.
- Produces: npm command `audit:rust` that exits nonzero when either lockfile contains a RustSec vulnerability.

- [x] **Step 1: Write the failing Rust toolchain and audit test**

Extend `src/toolchainSecurity.test.ts` to assert:

```ts
expect(read("rust-toolchain.toml")).toContain('channel = "nightly-2025-06-27"');
expect(packageJson.scripts["audit:rust"]).toBe(
  "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/audit-rust.ps1",
);
expect(existsSync(resolve(process.cwd(), "scripts/audit-rust.ps1"))).toBe(true);
expect(ci).toContain("npm run audit:rust");
```

After the existence assertion, inspect the script and require the exact cargo-audit version, asset hash, root lockfile, protocol lockfile, and `Get-FileHash`.

- [x] **Step 2: Run the focused test and verify RED**

Run:

```powershell
npm.cmd test -- --run src/toolchainSecurity.test.ts
```

Expected: FAIL because the nightly date, audit script, npm command, and CI step are absent.

- [x] **Step 3: Implement the pinned audit runner**

Change `rust-toolchain.toml` to:

```toml
[toolchain]
channel = "nightly-2025-06-27"
```

Add the exact npm command asserted in Step 1. Create `scripts/audit-rust.ps1` that:

1. accepts no secrets or repository writes;
2. creates a unique directory under `[IO.Path]::GetTempPath()`;
3. downloads `cargo-audit-x86_64-pc-windows-msvc-v0.22.2.zip` from the official RustSec GitHub release;
4. compares its SHA-256 to the Global Constraints value;
5. expands the archive and locates `cargo-audit.exe`;
6. runs `audit --file Cargo.lock` and `audit --file protocol\Cargo.lock`;
7. preserves a nonzero result from either audit;
8. removes only its verified unique temporary directory in `finally`.

Add `npm run audit:rust` after the Rust builds and tests in `.github/workflows/ci.yaml`.

- [x] **Step 4: Install and verify the pinned nightly**

Run:

```powershell
rustup toolchain install nightly-2025-06-27 --profile minimal
rustc --version
cargo --version
npm.cmd test -- --run src/toolchainSecurity.test.ts
```

Expected: rustc is at least 1.88-compatible and the focused Vitest passes.

---

### Task 2: Replace the vulnerable Rust lockfile entries

**Files:**
- Modify: `Cargo.lock`

**Interfaces:**
- Consumes: the pinned Rust 1.88-capable nightly from Task 1.
- Produces: a root lockfile containing the minimum patched versions and zero RustSec vulnerabilities.

- [x] **Step 1: Capture the failing security test**

Run:

```powershell
npm.cmd run audit:rust
```

Expected: FAIL and report the existing 11 vulnerabilities in `bytes`, `crossbeam-channel`, `crossbeam-epoch`, `idna`, `quick-xml` (two advisories), `rkyv`, `tar` (two advisories), `time`, and `tracing-subscriber`.

- [x] **Step 2: Apply only targeted Cargo updates**

Run these exact commands:

```powershell
cargo update -p bytes --precise 1.11.1
cargo update -p crossbeam-channel --precise 0.5.15
cargo update -p crossbeam-epoch --precise 0.9.20
cargo update -p url --precise 2.5.8
cargo update -p plist --precise 1.10.0
cargo update -p quick-xml --precise 0.41.0
cargo update -p rkyv --precise 0.7.46
cargo update -p tar --precise 0.4.45
cargo update -p time --precise 0.3.47
cargo update -p tracing-subscriber --precise 0.3.20
cargo update -p tokio --precise 1.48.0
cargo update -p anyhow --precise 1.0.102
```

`tokio` and `anyhow` are included because the current versions have RustSec unsound warnings and are direct application dependencies. Do not run bare `cargo update`.

- [x] **Step 3: Verify the security test is GREEN**

Run:

```powershell
npm.cmd run audit:rust
cargo tree --workspace --locked --target all --invert time@0.3.47
cargo tree --workspace --locked --target all --invert quick-xml@0.41.0
```

Expected: both lockfiles report zero vulnerabilities; `time` and `quick-xml` resolve through the intended Tauri 1 dependency graph.

---

### Task 3: Build, test, and commit the Rust security stage

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src/toolchainSecurity.test.ts`
- Verify all files changed by Tasks 1-2.
- Preserve: `AGENTS.md`

**Interfaces:**
- Consumes: the patched lockfile and audit runner.
- Produces: one verified Rust dependency security commit.

- [x] **Step 1: Load the Visual Studio developer environment**

Use the existing project packaging helper or `VsDevCmd.bat` so MSVC, Windows SDK, and linker environment variables are available in the current PowerShell process before Rust builds.

- [x] **Step 2: Run focused and required Rust verification**

First add a regression assertion that `dll-syringe` is pinned to `0.17.1` with only the `syringe` feature, verify it fails against `0.15.2`, then update the manifest and lockfile. This is required because `dll-syringe 0.15.2` does not compile on the Rust 1.88-capable nightly.

```powershell
npm.cmd test -- --run src/toolchainSecurity.test.ts
npm.cmd run audit:rust
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
```

Expected: every command exits 0. The audit may print allowed warnings but must report zero vulnerabilities.

- [x] **Step 3: Run the frontend regression gate**

```powershell
npm.cmd run format-check
npm.cmd run lint
npm.cmd run tsc
npm.cmd test -- --run
npm.cmd run build
```

Expected: every command exits 0.

- [x] **Step 4: Inspect and commit the exact scope**

```powershell
git diff --check
git status --short
git diff -- Cargo.lock rust-toolchain.toml package.json scripts/audit-rust.ps1 .github/workflows/ci.yaml src-tauri/Cargo.toml src/toolchainSecurity.test.ts
git add -- Cargo.lock rust-toolchain.toml package.json scripts/audit-rust.ps1 .github/workflows/ci.yaml src-tauri/Cargo.toml src/toolchainSecurity.test.ts docs/superpowers/plans/2026-07-24-rust-dependency-security.md
git commit -m "fix: patch Rust dependency vulnerabilities"
```

Expected: the commit succeeds without staging `AGENTS.md`, and no protocol source changes.

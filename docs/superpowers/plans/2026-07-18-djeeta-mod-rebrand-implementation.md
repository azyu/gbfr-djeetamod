# Djeeta MOD Rebrand Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename the distributable product to `Djeeta MOD`, add reusable Korean maintainer documentation, and produce a freshly verified 0.1.0 MSI.

**Architecture:** Keep all hook, parser, compact-overlay, and connection behavior unchanged. Apply one coherent identity mapping across npm, Tauri, Rust metadata, and visible headings; then document the project and rebuild the existing Windows MSI pipeline so the published hashes describe the renamed artifact.

**Tech Stack:** React 18, TypeScript, Vite, Tauri 1, Rust nightly-2024-05-04, PowerShell, WiX MSI

## Global Constraints

- User-facing product name is exactly `Djeeta MOD`.
- npm package name is exactly `djeeta-mod`.
- Tauri identifier is exactly `com.azyu.djeeta-mod`.
- Release version remains `0.1.0` in npm, Tauri, and Cargo manifests.
- Game target remains Granblue Fantasy: Relink Endless Ragnarok 2.0.2, explicitly labeled as an unverified test build until the manual checklist passes.
- Do not change hook signatures, event aggregation, 250 ms UI publication, 150 ms bar animation, reward-boundary lifetime, geometry, or click-through behavior.
- Preserve `LICENSE` unchanged and retain credit for False Spring and onelittlechildawa.
- Work directly on `master`, as previously approved by the user.
- Use `apply_patch` for file edits.

---

### Task 1: Apply the Djeeta MOD Product Identity

**Files:**
- Modify: `package.json`
- Modify: `package-lock.json`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-hook/Cargo.toml`
- Modify: `src/components/Titlebar.tsx`
- Modify: `src/pages/Logs.tsx`
- Modify: `src/pages/Settings.tsx`

**Interfaces:**
- Consumes: existing product version `0.1.0` and Tauri bundle configuration.
- Produces: the exact public identity `Djeeta MOD`, npm identity `djeeta-mod`, and installer identity `com.azyu.djeeta-mod`.

- [ ] **Step 1: Run the identity check and verify it fails before the rename**

```powershell
$package = Get-Content -Raw package.json | ConvertFrom-Json
$tauri = Get-Content -Raw src-tauri/tauri.conf.json | ConvertFrom-Json
if ($package.name -ne 'djeeta-mod') { throw "npm name is $($package.name)" }
if ($tauri.package.productName -ne 'Djeeta MOD') { throw "product name is $($tauri.package.productName)" }
if ($tauri.tauri.bundle.identifier -ne 'com.azyu.djeeta-mod') { throw "identifier is $($tauri.tauri.bundle.identifier)" }
```

Expected: the command throws because the current values still use `GBFR Korean Damage Meter` and `gbfr-korean-damage-meter`.

- [ ] **Step 2: Apply the exact manifest mapping**

Use `apply_patch` to make these replacements:

```text
package.json and package-lock.json
  gbfr-korean-damage-meter -> djeeta-mod

src-tauri/tauri.conf.json
  productName: Djeeta MOD
  main title: Djeeta MOD
  logs title: Djeeta MOD - Logs
  identifier: com.azyu.djeeta-mod
  longDescription: Unverified test build of Djeeta MOD targeting Granblue Fantasy: Relink 2.0.2
  shortDescription: Djeeta MOD Test Build

src-tauri/Cargo.toml
  description: Djeeta MOD for Granblue Fantasy: Relink
  first author: Djeeta MOD contributors

src-hook/Cargo.toml
  CompanyName: Djeeta MOD contributors
  LegalCopyright: Copyright (C) 2024 False Spring; 2026 onelittlechildawa; 2026 Djeeta MOD contributors
```

Replace the visible headings in `Titlebar.tsx`, `Logs.tsx`, and `Settings.tsx` with the literal `Djeeta MOD`. Keep the Korean upstream-credit sentence under the settings heading.

- [ ] **Step 3: Format the modified JSON and TypeScript files**

```powershell
npx prettier --write package.json package-lock.json src-tauri/tauri.conf.json src/components/Titlebar.tsx src/pages/Logs.tsx src/pages/Settings.tsx
```

Expected: Prettier exits 0.

- [ ] **Step 4: Re-run the identity and stale-name checks**

```powershell
$package = Get-Content -Raw package.json | ConvertFrom-Json
$lock = Get-Content -Raw package-lock.json | ConvertFrom-Json
$tauri = Get-Content -Raw src-tauri/tauri.conf.json | ConvertFrom-Json
$rootLockPackage = $lock.packages.PSObject.Properties[''].Value
if ($package.name -ne 'djeeta-mod' -or $lock.name -ne 'djeeta-mod' -or $rootLockPackage.name -ne 'djeeta-mod') { throw 'npm identity mismatch' }
if ($tauri.package.productName -ne 'Djeeta MOD') { throw 'Tauri product name mismatch' }
if ($tauri.tauri.bundle.identifier -ne 'com.azyu.djeeta-mod') { throw 'Tauri identifier mismatch' }
$stale = rg -n 'GBFR Korean Damage Meter|gbfr-korean-damage-meter|com\.azyu\.gbfr-korean-damage-meter' package.json package-lock.json src-tauri src-hook src
if ($LASTEXITCODE -eq 0) { throw "Stale distributable name found:`n$stale" }
```

Expected: all checks exit 0 and no stale distributable name is found.

- [ ] **Step 5: Run the fast frontend gate**

```powershell
npm run format-check
npm run lint
npm run tsc
npm test -- --run
```

Expected: format, lint, and type checking exit 0; all 10 frontend tests pass.

- [ ] **Step 6: Commit the product identity**

```powershell
git add package.json package-lock.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-hook/Cargo.toml src/components/Titlebar.tsx src/pages/Logs.tsx src/pages/Settings.tsx
git commit -m "chore: rebrand product as Djeeta MOD"
```

Expected: one commit containing only manifest, metadata, and visible-name changes.

---

### Task 2: Add Maintainer Documentation and Package the Renamed MSI

**Files:**
- Modify: `README.md`
- Create: `AGENTS.md`
- Modify: `docs/testing/game-2.0.2-smoke-test.md`
- Preserve: `LICENSE`
- Generate, ignored: `target/release/hook.dll`
- Generate, ignored: `target/release/bundle/msi/Djeeta MOD_0.1.0_x64_en-US.msi`

**Interfaces:**
- Consumes: Task 1 product identity and the existing hook-to-MSI copy rule in `src-tauri/build.rs`.
- Produces: Korean end-user instructions, reusable agent handoff instructions, final MSI and matching SHA-256 records.

- [ ] **Step 1: Update README with the exact public name and performance disclosure**

Use `apply_patch` so the title is `# Djeeta MOD`, the install step says `Djeeta MOD를 실행합니다`, and the document contains this section before the risk section:

```markdown
## 성능 영향

Djeeta MOD는 게임의 그래픽 설정이나 렌더링 품질을 변경하지 않습니다. 다만 게임 프로세스의 피해 훅, named pipe 파싱, 별도 투명 WebView 오버레이가 CPU와 메모리를 사용합니다. 오버레이 표시는 250ms 간격으로 갱신되고 WebView GPU 가속은 비활성화되어 있어 예상 GPU 부담은 작지만, 실제 게임 비교 측정 전에는 성능 영향이 전혀 없다고 보증하지 않습니다.
```

Keep the unverified-test-build warning, DLL-injection warning, source-build commands, upstream links, license link, and build-hash section.

- [ ] **Step 2: Create the reusable root AGENTS.md**

Use `apply_patch` to create `AGENTS.md` with these exact operational sections and rules:

```markdown
# Djeeta MOD Maintainer Guide

## Product contract

- Public name: `Djeeta MOD`
- Package name: `djeeta-mod`
- Tauri identifier: `com.azyu.djeeta-mod`
- Version: `0.1.0`
- Target: Granblue Fantasy: Relink Endless Ragnarok 2.0.2 on Windows x64
- Default language: Korean
- Compatibility remains unverified until `docs/testing/game-2.0.2-smoke-test.md` is completed.

## Architecture

- `src-hook/`: injected Rust DLL; captures player identity, damage, and the reward boundary.
- `protocol/`: append-only bincode wire messages shared by the DLL and Tauri backend.
- `src-tauri/`: named-pipe client, encounter parser, persistence, Tauri commands, and Windows packaging.
- `src/`: React compact overlay and logs/settings UI.

## Behavioral invariants

- First accepted hit starts an encounter.
- All targets in one battle contribute to the same party totals.
- Inactivity must not split or hide a live encounter.
- The meter clears immediately before the reward UI.
- `HookStatus::Unsupported` is latched; later gameplay frames must not mark the connection ready.
- Unknown enemies are ignored unless a verified player identity owns the actor.
- The overlay shows at most four rows: Korean character name, cumulative damage/bar, and DPS.
- Presentation publishes every 250ms and bars transition over 150ms.
- 1920x1080 reset geometry is 330x145 at x45/y470.
- Normal mode is click-through.

## Toolchain

- Node.js 20
- Visual Studio 2022 C++ Build Tools and Windows SDK
- rustup toolchain from `rust-toolchain.toml` (`nightly-2024-05-04`)
- WebView2 and WiX as used by Tauri 1

Load the Visual Studio developer environment before Rust builds when the shell does not already expose MSVC.

## Required verification

```powershell
npm ci
npm run format-check
npm run lint
npm run tsc
npm test -- --run
npm run build
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
npm run tauri build -- --bundles msi
```

After packaging, require SHA-256 equality between `target/release/hook.dll` and `src-tauri/hook.dll`, then record the MSI and hook hashes in `README.md` and `docs/testing/game-2.0.2-smoke-test.md`.

## Change discipline

- Work on `master` only when the user explicitly requests it.
- Use tests before changing lifecycle, hook, parser, handshake, geometry, or throttling behavior.
- Append protocol variants; never reorder existing bincode variants.
- Preserve `LICENSE` and upstream credit for False Spring and onelittlechildawa.
- Do not claim game 2.0.2 compatibility before the manual smoke-test checklist passes in an offline or private session.
```

- [ ] **Step 3: Verify documentation identity and required content**

```powershell
rg -n 'Djeeta MOD|성능 영향|250ms|GPU 가속' README.md
rg -n 'Product contract|Behavioral invariants|Required verification|HookStatus::Unsupported|reward UI|LICENSE' AGENTS.md
$stale = rg -n 'GBFR Korean Damage Meter|gbfr-korean-damage-meter' README.md AGENTS.md docs/testing/game-2.0.2-smoke-test.md
if ($LASTEXITCODE -eq 0) { throw "Stale public name found:`n$stale" }
```

Expected: required sections are found and the stale-name check finds nothing.

- [ ] **Step 4: Run the complete automated gate**

Load the Visual Studio environment in the current PowerShell process, then run:

```powershell
npm ci
npm run format-check
npm run lint
npm run tsc
npm test -- --run
npm run build
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
npm run tauri build -- --bundles msi
```

Expected: all commands exit 0, 10 frontend tests pass, 54 Rust tests pass, and `target/release/bundle/msi/Djeeta MOD_0.1.0_x64_en-US.msi` exists.

- [ ] **Step 5: Calculate and verify final artifact hashes**

```powershell
$builtHook = Resolve-Path 'target\release\hook.dll'
$stagedHook = Resolve-Path 'src-tauri\hook.dll'
$msi = Get-ChildItem 'target\release\bundle\msi\Djeeta MOD_0.1.0_x64_en-US.msi'
$hookHash = (Get-FileHash -Algorithm SHA256 $builtHook).Hash
$stagedHash = (Get-FileHash -Algorithm SHA256 $stagedHook).Hash
$msiHash = (Get-FileHash -Algorithm SHA256 $msi.FullName).Hash
if ($hookHash -ne $stagedHash) { throw 'Staged hook does not match release hook' }
[pscustomobject]@{ HookSHA256 = $hookHash; MSISHA256 = $msiHash; MSIBytes = $msi.Length }
```

Expected: hook hashes are equal and the MSI hash and byte size are printed.

- [ ] **Step 6: Record the printed hashes using apply_patch**

Replace the existing MSI and `hook.dll` SHA-256 values in both `README.md` and `docs/testing/game-2.0.2-smoke-test.md` with the exact values printed in Step 5. Do not alter the unchecked manual-game rows.

- [ ] **Step 7: Re-run the final manifest, hash, and test proof**

```powershell
$package = Get-Content -Raw package.json | ConvertFrom-Json
$tauri = Get-Content -Raw src-tauri/tauri.conf.json | ConvertFrom-Json
if ($package.name -ne 'djeeta-mod' -or $tauri.package.productName -ne 'Djeeta MOD' -or $tauri.tauri.bundle.identifier -ne 'com.azyu.djeeta-mod') { throw 'Final identity mismatch' }
$hookHash = (Get-FileHash -Algorithm SHA256 'target\release\hook.dll').Hash
$stagedHash = (Get-FileHash -Algorithm SHA256 'src-tauri\hook.dll').Hash
$msi = Get-ChildItem 'target\release\bundle\msi\Djeeta MOD_0.1.0_x64_en-US.msi'
$msiHash = (Get-FileHash -Algorithm SHA256 $msi.FullName).Hash
$readme = Get-Content -Raw README.md
$smoke = Get-Content -Raw docs/testing/game-2.0.2-smoke-test.md
if ($hookHash -ne $stagedHash -or !$readme.Contains($hookHash) -or !$smoke.Contains($hookHash) -or !$readme.Contains($msiHash) -or !$smoke.Contains($msiHash)) { throw 'Final hash proof failed' }
git diff --check
npm run format-check
npm run lint
npm run tsc
npm test -- --run
cargo test --workspace --all-targets --locked
```

Expected: identity and hash checks exit 0, the diff check is clean, 10 frontend tests pass, and 54 Rust tests pass.

- [ ] **Step 8: Commit documentation and package records**

```powershell
git add README.md AGENTS.md docs/testing/game-2.0.2-smoke-test.md
git commit -m "docs: add Djeeta MOD maintainer guide"
git status --short
```

Expected: commit succeeds and the working tree is clean. Generated `target/` artifacts remain ignored.

---

## Manual Release Boundary

The renamed MSI is a test build, not a confirmed compatible release. Complete every row in `docs/testing/game-2.0.2-smoke-test.md` in an offline or private game session before removing the unverified wording or claiming official 2.0.2 compatibility.

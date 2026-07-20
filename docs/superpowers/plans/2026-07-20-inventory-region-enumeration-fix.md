# Inventory Region Enumeration Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the debug inventory probe enumerate only the native Windows application address range so actual-game capture no longer ends with `INTERNAL stage=region-enumeration`.

**Architecture:** Add a small pure inclusive-bound helper in the existing memory reader, then obtain the native minimum and maximum addresses from `GetNativeSystemInfo` before calling `VirtualQueryEx`. Preserve fail-closed query handling and all existing read-only process rights.

**Tech Stack:** Rust, `windows` crate 0.52, Win32 `GetNativeSystemInfo`, existing Cargo workspace tests, Tauri debug runtime.

## Global Constraints

- Keep the application manifest at `requestedExecutionLevel level="asInvoker"`.
- Keep process access at `PROCESS_QUERY_INFORMATION | PROCESS_VM_READ`.
- Do not add process write, memory operation, thread creation, or remote code execution rights.
- Keep `VirtualQueryEx` failures inside the native address range fail-closed.
- Do not change scanner performance, hook injection behavior, or Equipment Analysis UI in this stage.
- Do not claim game 2.0.2 full-inventory compatibility from this fix alone.
- Do not modify or stage the existing `logs.db` file.

---

### Task 1: Use the native Windows application address range

**Files:**
- Modify: `src-tauri/Cargo.toml:45`
- Modify: `src-tauri/src/equipment_probe/memory.rs:13-29,158-191,347-430`
- Test: `src-tauri/src/equipment_probe/memory.rs` unit-test module

**Interfaces:**
- Consumes: Win32 `GetNativeSystemInfo(*mut SYSTEM_INFO)` and `SYSTEM_INFO::{lpMinimumApplicationAddress, lpMaximumApplicationAddress}`.
- Produces: `fn address_is_in_application_range(address: usize, minimum: usize, maximum: usize) -> bool` and a `RemoteProcess::readable_private_regions` implementation bounded by the native range.

- [ ] **Step 1: Write the failing inclusive-bound unit test**

Add the helper import and this test to the existing `memory.rs` test module:

```rust
#[test]
fn application_address_range_is_inclusive() {
    let minimum = 0x1_0000;
    let maximum = 0x7fff_fffe_ffff;

    assert!(!address_is_in_application_range(minimum - 1, minimum, maximum));
    assert!(address_is_in_application_range(minimum, minimum, maximum));
    assert!(address_is_in_application_range(maximum, minimum, maximum));
    assert!(!address_is_in_application_range(maximum + 1, minimum, maximum));
}
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```powershell
cargo test --locked --package gbfr-logs equipment_probe::memory::tests::application_address_range_is_inclusive
```

Expected: compilation fails because `address_is_in_application_range` does not exist. This proves the new test exercises the missing boundary behavior.

- [ ] **Step 3: Add the Windows system-information feature**

Extend the existing `windows` dependency feature list in `src-tauri/Cargo.toml` with:

```toml
"Win32_System_SystemInformation"
```

Do not reorder or change the dependency version.

- [ ] **Step 4: Implement the minimal inclusive helper**

Add next to `MemoryRegion`:

```rust
fn address_is_in_application_range(address: usize, minimum: usize, maximum: usize) -> bool {
    minimum <= address && address <= maximum
}
```

- [ ] **Step 5: Replace the hard-coded range with `GetNativeSystemInfo`**

Import the API and type:

```rust
SystemInformation::{GetNativeSystemInfo, SYSTEM_INFO},
```

At the start of `readable_private_regions`, initialize and populate the structure:

```rust
pub(crate) fn readable_private_regions(&self) -> Result<Vec<MemoryRegion>, MemoryReadError> {
    let mut system_info = SYSTEM_INFO::default();
    unsafe { GetNativeSystemInfo(&mut system_info) };
    let minimum_address = system_info.lpMinimumApplicationAddress as usize;
    let maximum_address = system_info.lpMaximumApplicationAddress as usize;
    if minimum_address > maximum_address {
        return Err(MemoryReadError::Windows(
            "native application address range is invalid".to_owned(),
        ));
    }

    let mut regions = Vec::new();
    let mut address = minimum_address;
    while address_is_in_application_range(address, minimum_address, maximum_address) {
        let mut info = MEMORY_BASIC_INFORMATION::default();
        let queried = unsafe {
            VirtualQueryEx(
                self.handle.0,
                Some(address as *const c_void),
                &mut info,
                std::mem::size_of_val(&info),
            )
        };
        if queried == 0 {
            return Err(windows_error(windows::core::Error::from_win32()));
        }
        if info.RegionSize == 0 {
            return Err(MemoryReadError::Windows(
                "VirtualQueryEx returned an empty region".to_owned(),
            ));
        }
        if is_readable_private_region(info.State.0, info.Type.0, info.Protect.0) {
            regions.push(MemoryRegion {
                base_address: info.BaseAddress as usize,
                size: info.RegionSize,
            });
        }
        address = (info.BaseAddress as usize)
            .checked_add(info.RegionSize)
            .ok_or(MemoryReadError::InvalidPe("memory region range overflow"))?;
    }
    Ok(regions)
}
```

Do not special-case `ERROR_INVALID_PARAMETER`; the loop must prevent the expected terminal out-of-range query.

- [ ] **Step 6: Run focused tests and verify GREEN**

Run:

```powershell
cargo test --locked --package gbfr-logs equipment_probe::memory::tests
cargo test --locked --package gbfr-logs equipment_probe::inventory::tests
```

Expected: all memory and inventory probe tests pass, including the new inclusive-bound test.

- [ ] **Step 7: Run security assertions**

Run:

```powershell
npm test -- --run src/securityConfiguration.test.ts
```

Expected: all security tests pass and continue to reject write/operation/thread rights and remote mutation APIs.

- [ ] **Step 8: Format, inspect, and commit**

Run:

```powershell
rustfmt --edition 2021 src-tauri/src/equipment_probe/memory.rs
git diff --check
git diff -- src-tauri/Cargo.toml src-tauri/src/equipment_probe/memory.rs
```

Stage only the two implementation files and commit:

```powershell
git add -- src-tauri/Cargo.toml src-tauri/src/equipment_probe/memory.rs
git commit -m "fix: bound remote memory enumeration"
```

### Task 2: Re-run the actual-game capture

**Files:**
- Modify only on a verified result: `docs/testing/game-2.0.2-inventory-probe.md`

**Interfaces:**
- Consumes: the pinned running `granblue_fantasy_relink.exe`, environment variable `DJEETA_INVENTORY_PROBE=1`, and backend inventory-probe logs.
- Produces: evidence that region enumeration completes and the next terminal probe status is observable.

- [ ] **Step 1: Confirm the game is still the pinned build**

Run a read-only process lookup and SHA-256 check. Expected executable hash:

```text
63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F
```

Do not proceed if the process is absent or the hash differs.

- [ ] **Step 2: Start the debug app with the probe enabled**

Run:

```powershell
$env:DJEETA_INVENTORY_PROBE = "1"
npm run tauri dev
```

Expected: the app starts without elevation and the capture control is visible.

- [ ] **Step 3: Capture from the unfiltered in-game sigil inventory**

Have the user open the unfiltered sigil inventory and select **보유 진 캡처** once. Read the backend log and verify that it does not contain:

```text
INVENTORY PROBE status=INTERNAL stage=region-enumeration
```

Record the next terminal status and scan timing. If it is `LIMIT_EXCEEDED`, stop and open a separate scanner-performance stage instead of changing this fix.

- [ ] **Step 4: Record only verified checklist evidence**

If and only if the capture reaches `STABLE` and the user supplies the in-game count, update the baseline row in `docs/testing/game-2.0.2-inventory-probe.md` with PID, candidate records, occupied count, digest, UI count, and result. Do not include raw memory addresses or full inventory data.

If the status is not `STABLE`, leave the row unchecked and report the exact stable public status.

- [ ] **Step 5: Commit verified documentation if changed**

When the checklist changed:

```powershell
git add -- docs/testing/game-2.0.2-inventory-probe.md
git commit -m "docs: record inventory probe baseline"
```

Skip this commit when no checklist evidence was added.

### Task 3: Full regression verification

**Files:**
- No source changes expected.

**Interfaces:**
- Consumes: the Task 1 implementation and Task 2 runtime result.
- Produces: a merge-ready stage-1 branch only if all automated checks pass.

- [ ] **Step 1: Run frontend verification**

Run:

```powershell
npm run format-check
npm run lint
npm run tsc
npm test -- --run
npm run build
```

Expected: every command exits zero. Existing dependency-audit and Rollup chunk-size warnings may be reported but must not be changed in this stage.

- [ ] **Step 2: Run Rust verification**

Run:

```powershell
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
```

Expected: both commands exit zero. Existing hook dead-code warnings may remain.

- [ ] **Step 3: Confirm repository scope**

Run:

```powershell
git status --short
git diff master...HEAD --check
git log --oneline master..HEAD
```

Expected: only the stage-1 design, plan, implementation, and any verified checklist update are committed; `logs.db` remains untracked and unstaged.

- [ ] **Step 4: Request code review**

Review `master..HEAD` for correct native bounds, fail-closed behavior, unchanged access rights, and test coverage. Do not merge until no Critical or Important findings remain.

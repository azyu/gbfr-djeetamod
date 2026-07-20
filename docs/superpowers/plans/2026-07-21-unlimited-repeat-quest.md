# Unlimited Repeat Quest Toggle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a release-visible, default-OFF `무한 퀘스트 반복` switch that safely applies and restores the two verified Granblue Fantasy: Relink 2.0.2 code patches without Cheat Engine or automatic elevation.

**Architecture:** A dedicated `repeat_quest` Tauri backend module owns pure signature discovery, target-byte classification, transactional patching, the narrowly scoped Windows writable handle, and lifecycle restoration. The React settings page calls two Tauri commands through a non-persistent hook; the existing equipment and inventory probes remain read-only and the injected-hook protocol remains unchanged.

**Tech Stack:** Rust nightly-2024-05-04, Tauri 1.6, `windows` 0.52 Win32 bindings, serde, React 18, TypeScript, Mantine 7, Vitest, Rust unit tests.

## Global Constraints

- Support only `granblue_fantasy_relink.exe` with SHA-256 `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F`.
- Derive only the `Unlimited Repeat Quest` behavior from CT SHA-256 `65A3677AD62593617077B9655530FE52B24189CA02E1CEC93C16648CF5FC3072`; do not bundle or launch Cheat Engine.
- Never persist absolute addresses or the ON state; rediscover sites for every PID and start each Djeeta MOD launch OFF.
- Keep `src-tauri/manifest.xml` at `requestedExecutionLevel level="asInvoker"` and never auto-elevate.
- Keep `src-tauri/src/equipment_probe/memory.rs` and `inventory.rs` read-only; writable APIs belong only to `src-tauri/src/repeat_quest.rs`.
- Change only the verified reset bytes `45 31 C0 <-> 44 8B 01` and getter bytes `0F B6 01 <-> B0 01 90`.
- Refuse unknown target bytes, ambiguous signatures, unsupported hashes, and partial success. Roll back the first enable write if the second fails.
- Normal app exit attempts synchronous restoration. Forced process termination can leave the patch until Djeeta MOD restarts or the game exits.
- Keep the current hook DLL protocol send-only and unchanged.
- Preserve unrelated working-tree changes in `Cargo.toml`, the inventory scanner documents, `src-tauri/src/equipment_probe/inventory.rs`, and untracked `logs.db`. Stage only the paths named by each task.
- Do not claim 2.0.2 feature compatibility until the offline/private manual checklist passes.

## File Map

- Create `src-tauri/src/repeat_quest.rs`: pure patch model, Win32 integration, runtime state, Tauri commands, startup cleanup, and exit restoration.
- Modify `src-tauri/src/equipment_probe/mod.rs`: expose the already-pinned process name and executable hash inside the crate without duplicating constants.
- Modify `src-tauri/src/main.rs`: register state/commands, run startup cleanup, and route the Tauri exit event through restoration.
- Create `src/pages/useRepeatQuest.ts`: non-persistent frontend status and toggle state.
- Modify `src/pages/Settings.tsx`: render the separate game-feature fieldset and switch.
- Create `src/pages/Settings.repeatQuest.test.tsx`: command, pending, reason, and failed-toggle behavior.
- Modify `src/pages/Settings.localization.test.ts`: require Korean and English game-feature copy.
- Modify `src-tauri/lang/ko/ui.json` and `src-tauri/lang/en/ui.json`: localized labels and reason strings.
- Modify `src/securityConfiguration.test.ts`: prove the old probes remain read-only and writable APIs are isolated.
- Modify `docs/testing/game-2.0.2-smoke-test.md`: add manual ON/OFF, normal-exit restoration, and no-elevation results.
- After a successful NSIS build only, update `README.md` and the smoke-test document with the new installer and hook hashes required by the maintainer guide.

---

### Task 1: Pure signature discovery and byte-state model

**Files:**
- Create: `src-tauri/src/repeat_quest.rs`
- Modify: `src-tauri/src/main.rs`

**Interfaces:**
- Produces: `PatchOffsets { reset: usize, getter: usize }`
- Produces: `PatchSites { reset: usize, getter: usize }`
- Produces: `SiteBytes::{Original, Patched, Unknown}` and `ObservedPatchState::{Off, On, Mixed, Unknown}`
- Produces: `find_patch_offsets(text: &[u8]) -> Result<PatchOffsets, RepeatQuestError>`
- Consumes later: Task 2 uses `PatchSites` and byte classification; Task 3 converts offsets to remote addresses.

- [ ] **Step 1: Register the module and write failing pure tests**

Add `mod repeat_quest;` beside the existing backend modules in `src-tauri/src/main.rs`. Create `src-tauri/src/repeat_quest.rs` with tests that build byte fixtures from the exact CT signatures:

```rust
#[test]
fn finds_each_repeat_quest_signature_once() {
    let text = signature_fixture(1, 1);
    assert_eq!(
        find_patch_offsets(&text).unwrap(),
        PatchOffsets { reset: 0x128, getter: 0x212 }
    );
}

#[test]
fn rejects_missing_or_duplicate_signatures() {
    assert_eq!(
        find_patch_offsets(&signature_fixture(0, 1)),
        Err(RepeatQuestError::SignatureCount { site: PatchSiteName::Reset, count: 0 })
    );
    assert_eq!(
        find_patch_offsets(&signature_fixture(1, 2)),
        Err(RepeatQuestError::SignatureCount { site: PatchSiteName::Getter, count: 2 })
    );
}

#[test]
fn classifies_original_patched_mixed_and_unknown_bytes() {
    assert_eq!(classify_pair(RESET_ORIGINAL, GETTER_ORIGINAL), ObservedPatchState::Off);
    assert_eq!(classify_pair(RESET_PATCHED, GETTER_PATCHED), ObservedPatchState::On);
    assert_eq!(classify_pair(RESET_PATCHED, GETTER_ORIGINAL), ObservedPatchState::Mixed);
    assert_eq!(classify_pair([0x90; 3], GETTER_ORIGINAL), ObservedPatchState::Unknown);
}
```

The fixture must place the full reset AOB at `0x100` and full getter AOB at `0x200`, varying only the four `??` displacement bytes. The expected patch offsets are signature start plus `0x28` and `0x12` respectively.

- [ ] **Step 2: Run the focused Rust test and observe RED**

Run:

```powershell
cargo test --locked --package gbfr-logs repeat_quest::tests::finds_each_repeat_quest_signature_once
```

Expected: compilation fails because the model and discovery functions do not exist.

- [ ] **Step 3: Implement the minimal pure model and masked discovery**

Define the exact three-byte constants and a small prefix/wildcard/suffix matcher:

```rust
const RESET_ORIGINAL: [u8; 3] = [0x45, 0x31, 0xC0];
const RESET_PATCHED: [u8; 3] = [0x44, 0x8B, 0x01];
const GETTER_ORIGINAL: [u8; 3] = [0x0F, 0xB6, 0x01];
const GETTER_PATCHED: [u8; 3] = [0xB0, 0x01, 0x90];
const RESET_PATCH_OFFSET: usize = 0x28;
const GETTER_PATCH_OFFSET: usize = 0x12;

const RESET_PREFIX: &[u8] = &[
    0x48, 0x83, 0xB8, 0x08, 0xC1, 0x01, 0x00, 0x00, 0xC7, 0x80,
    0x24, 0xC1, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0F, 0x84,
];
const RESET_SUFFIX: &[u8] = &[
    0xC6, 0x87, 0x24, 0x06, 0x00, 0x00, 0x00, 0x48, 0x8D, 0x8F,
    0x28, 0x06, 0x00, 0x00, 0x31, 0xD2, 0x45, 0x31, 0xC0, 0x44,
    0x89, 0x01, 0x85, 0xDB, 0x75, 0x15,
];
const GETTER_PREFIX: &[u8] = &[
    0x48, 0x83, 0xC1, 0x15, 0xEB, 0x0C, 0xB9, 0x24, 0x06, 0x00,
    0x00, 0x48, 0x03, 0x0D,
];
const GETTER_SUFFIX: &[u8] = &[0x0F, 0xB6, 0x01, 0x48, 0x83, 0xC4, 0x20];
const SIGNATURE_WILDCARD_BYTES: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PatchSiteName { Reset, Getter }

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
enum RepeatQuestError {
    #[error("game is not running")]
    GameNotRunning,
    #[error("unsupported game executable")]
    UnsupportedGame,
    #[error("{site:?} signature count was {count}")]
    SignatureCount { site: PatchSiteName, count: usize },
    #[error("patch address overflow")]
    AddressOverflow,
    #[error("target bytes are neither original nor patched")]
    UnexpectedBytes,
    #[error("process access denied")]
    AccessDenied,
    #[error("read failed at {address:#x}: {detail}")]
    Read { address: usize, detail: String },
    #[error("write failed at {address:#x}: {detail}")]
    Write { address: usize, detail: String },
    #[error("write returned {actual} of {expected} bytes")]
    PartialWrite { expected: usize, actual: usize },
    #[error("page protection failed at {address:#x}: {detail}")]
    Protection { address: usize, detail: String },
    #[error("write and protection restore both failed at {address:#x}")]
    WriteAndProtectionRestore { address: usize },
    #[error("instruction-cache flush failed at {address:#x}: {detail}")]
    Flush { address: usize, detail: String },
    #[error("final byte read-back did not match the requested state")]
    ReadBackMismatch,
    #[error("enable failed and rollback did not restore OFF")]
    Rollback,
    #[error("pinned SHA-256 constant is invalid")]
    InvalidPinnedHash,
}

fn masked_matches(window: &[u8], prefix: &[u8], wildcard_len: usize, suffix: &[u8]) -> bool {
    window.get(..prefix.len()) == Some(prefix)
        && window.get(prefix.len() + wildcard_len..) == Some(suffix)
}

fn unique_signature_offset(
    text: &[u8],
    site: PatchSiteName,
    prefix: &[u8],
    wildcard_len: usize,
    suffix: &[u8],
) -> Result<usize, RepeatQuestError> {
    let signature_len = prefix.len() + wildcard_len + suffix.len();
    let matches = text
        .windows(signature_len)
        .enumerate()
        .filter_map(|(offset, window)| masked_matches(window, prefix, wildcard_len, suffix).then_some(offset))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [offset] => Ok(*offset),
        _ => Err(RepeatQuestError::SignatureCount { site, count: matches.len() }),
    }
}
```

Use the exact constants above with four wildcard branch/RIP-displacement bytes. `find_patch_offsets` must use checked addition for both patch offsets. Implement `classify_site` and `classify_pair` without treating any other byte sequence as writable.

- [ ] **Step 4: Run all new pure tests and observe GREEN**

Run:

```powershell
cargo test --locked --package gbfr-logs repeat_quest::tests
```

Expected: all Task 1 tests pass.

- [ ] **Step 5: Commit only the pure model**

```powershell
git add -- src-tauri/src/repeat_quest.rs src-tauri/src/main.rs
git commit -m "feat: identify repeat quest patch sites"
```

---

### Task 2: Transactional enable and safe restoration

**Files:**
- Modify: `src-tauri/src/repeat_quest.rs`

**Interfaces:**
- Produces: `trait PatchMemory { read_site; write_site }`
- Produces: `enable_patch(memory: &mut impl PatchMemory, sites: PatchSites) -> Result<ObservedPatchState, RepeatQuestError>`
- Produces: `restore_patch(memory: &mut impl PatchMemory, sites: PatchSites) -> Result<ObservedPatchState, RepeatQuestError>`
- Consumes later: the Windows adapter in Task 3 implements `PatchMemory`.

- [ ] **Step 1: Add failing fake-memory transaction tests**

Use a `FakePatchMemory` backed by `HashMap<usize, [u8; 3]>` with an optional `fail_write_at` address and a `writes: Vec<(usize, [u8; 3])>`. Add these tests:

```rust
#[test]
fn enable_writes_both_sites_and_verifies_on() {
    let (mut memory, sites) = original_memory();
    assert_eq!(enable_patch(&mut memory, sites).unwrap(), ObservedPatchState::On);
    assert_eq!(memory.writes, vec![(sites.reset, RESET_PATCHED), (sites.getter, GETTER_PATCHED)]);
}

#[test]
fn second_enable_write_failure_rolls_back_the_first() {
    let (mut memory, sites) = original_memory();
    memory.fail_write_once_at = Some(sites.getter);
    assert!(matches!(enable_patch(&mut memory, sites), Err(RepeatQuestError::Write { .. })));
    assert_eq!(memory.bytes_at(sites.reset), RESET_ORIGINAL);
    assert_eq!(memory.bytes_at(sites.getter), GETTER_ORIGINAL);
}

#[test]
fn restore_repairs_mixed_state_without_touching_original_site() {
    let (mut memory, sites) = mixed_memory();
    assert_eq!(restore_patch(&mut memory, sites).unwrap(), ObservedPatchState::Off);
    assert_eq!(memory.writes, vec![(sites.reset, RESET_ORIGINAL)]);
}

#[test]
fn unknown_bytes_are_never_overwritten() {
    let (mut memory, sites) = original_memory();
    memory.set(sites.getter, [0x90; 3]);
    assert_eq!(enable_patch(&mut memory, sites), Err(RepeatQuestError::UnexpectedBytes));
    assert_eq!(restore_patch(&mut memory, sites), Err(RepeatQuestError::UnexpectedBytes));
    assert!(memory.writes.is_empty());
}
```

Also cover enabling from `On` as an idempotent success, refusing `Mixed` enable, restoring `Off` without writes, rollback failure, and final read-back mismatch.

- [ ] **Step 2: Run the transaction tests and observe RED**

Run:

```powershell
cargo test --locked --package gbfr-logs repeat_quest::tests::enable_writes_both_sites_and_verifies_on
cargo test --locked --package gbfr-logs repeat_quest::tests::second_enable_write_failure_rolls_back_the_first
```

Expected: compilation fails because `PatchMemory`, `enable_patch`, and `restore_patch` do not exist.

- [ ] **Step 3: Implement the minimal transaction engine**

Use the narrow interface:

```rust
trait PatchMemory {
    fn read_site(&self, address: usize) -> Result<[u8; 3], RepeatQuestError>;
    fn write_site(&mut self, address: usize, bytes: [u8; 3]) -> Result<(), RepeatQuestError>;
}
```

`enable_patch` must read and classify both sites before any write. Permit only `Off` or already `On`. After writing reset then getter, read both sites again. On getter failure or non-ON read-back, attempt to restore every site containing the known patched bytes and return the original failure unless rollback also fails, in which case return `RepeatQuestError::Rollback`.

`restore_patch` must pre-read both sites. If either is unknown, return before any write. Restore only sites classified as patched, then require a final `Off` read-back.

- [ ] **Step 4: Run the complete backend unit-test module and observe GREEN**

Run:

```powershell
cargo test --locked --package gbfr-logs repeat_quest::tests
```

Expected: discovery, classification, enable, rollback, restoration, and read-back tests all pass.

- [ ] **Step 5: Commit the transaction engine**

```powershell
git add -- src-tauri/src/repeat_quest.rs
git commit -m "feat: patch repeat quest transactionally"
```

---

### Task 3: Windows process validation and isolated write boundary

**Files:**
- Modify: `src-tauri/src/repeat_quest.rs`
- Modify: `src-tauri/src/equipment_probe/mod.rs`
- Modify: `src/securityConfiguration.test.ts`

**Interfaces:**
- Consumes: `RemoteProcess::find`, `read_text_section`, and `executable_sha256` from the existing read-only probe memory module.
- Produces: `resolve_process_sites() -> Result<(RemoteProcess, PatchSites), RepeatQuestError>`
- Produces: Windows-only `WritablePatchMemory` implementing `PatchMemory`.
- Produces: `observe_current()`, `enable_current()`, and `restore_current()` for Task 4.

- [ ] **Step 1: Add failing hash/address/security tests**

In Rust, test checked conversion from a `.text` base plus `PatchOffsets` to `PatchSites`, and reject a non-pinned hash before constructing a writable adapter. Keep the hash decision in a pure helper:

```rust
#[test]
fn accepts_only_the_pinned_game_hash() {
    let pinned = parse_sha256(PINNED_GAME_SHA256).unwrap();
    assert!(verify_game_hash(&pinned).is_ok());
    assert_eq!(verify_game_hash(&[0; 32]), Err(RepeatQuestError::UnsupportedGame));
}

#[test]
fn converts_text_offsets_to_checked_remote_addresses() {
    assert_eq!(
        patch_sites(0x140001000, PatchOffsets { reset: 0x28, getter: 0x112 }).unwrap(),
        PatchSites { reset: 0x140001028, getter: 0x140001112 }
    );
}
```

Extend `src/securityConfiguration.test.ts` with a test that requires `PROCESS_VM_WRITE`, `PROCESS_VM_OPERATION`, `WriteProcessMemory`, `VirtualProtectEx`, and `FlushInstructionCache` in `repeat_quest.rs`, while rejecting `VirtualAllocEx`, `CreateRemoteThread`, and `PROCESS_CREATE_THREAD`. Retain the existing read-only tests unchanged.

- [ ] **Step 2: Run Rust and security tests and observe RED**

Run:

```powershell
cargo test --locked --package gbfr-logs repeat_quest::tests::accepts_only_the_pinned_game_hash
npm test -- --run src/securityConfiguration.test.ts
```

Expected: Rust compilation fails for missing integration helpers, and the new security test fails because the writable boundary is absent.

- [ ] **Step 3: Expose shared target constants without broadening probe writes**

Change only the visibility of the existing constants in `equipment_probe/mod.rs`:

```rust
pub(crate) const GAME_PROCESS_NAME: &str = "granblue_fantasy_relink.exe";
pub(crate) const PINNED_GAME_SHA256: &str =
    "63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F";
```

Do not move or duplicate the constants and do not change the existing probe access rights.

- [ ] **Step 4: Implement discovery followed by a just-in-time writable handle**

`resolve_process_sites` must:

1. call `RemoteProcess::find(GAME_PROCESS_NAME)`;
2. return `GameNotRunning` when absent;
3. verify `executable_sha256()` against `PINNED_GAME_SHA256`;
4. call `read_text_section()`;
5. call `find_patch_offsets` and checked-address conversion;
6. return the still-running read-only `RemoteProcess` and sites.

Implement `parse_sha256(&str) -> Result<[u8; 32], RepeatQuestError>` without a new dependency: require exactly 64 ASCII hex characters and decode each two-character pair with `u8::from_str_radix`. `verify_game_hash` compares the returned file hash with this parsed pinned value.

Only after all six checks may `WritablePatchMemory::open(&RemoteProcess)` call:

```rust
OpenProcess(
    PROCESS_QUERY_INFORMATION | PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION,
    false,
    process.pid,
)
```

Implement its `write_site` using the `windows 0.52` signatures already available under current crate features:

```rust
VirtualProtectEx(handle, address as *const c_void, 3, PAGE_EXECUTE_READWRITE, &mut old)?;
let write_result = WriteProcessMemory(
    handle,
    address as *const c_void,
    bytes.as_ptr().cast::<c_void>(),
    bytes.len(),
    Some(&mut written),
);
let restore_result = VirtualProtectEx(handle, address as *const c_void, 3, old, &mut restored);
match (write_result, restore_result) {
    (Err(_), Err(_)) => return Err(RepeatQuestError::WriteAndProtectionRestore { address }),
    (Err(error), Ok(())) => return Err(RepeatQuestError::Write { address, detail: error.to_string() }),
    (Ok(()), Err(error)) => return Err(RepeatQuestError::Protection { address, detail: error.to_string() }),
    (Ok(()), Ok(())) => {}
}
if written != bytes.len() { return Err(RepeatQuestError::PartialWrite { expected: bytes.len(), actual: written }); }
FlushInstructionCache(handle, Some(address as *const c_void), bytes.len())?;
```

Preserve and report both write and protection-restoration failures. `observe_current` uses only the existing read-only handle. `enable_current` and `restore_current` re-resolve and revalidate immediately before opening the writable handle.

- [ ] **Step 5: Run integration-focused tests and observe GREEN**

Run:

```powershell
cargo test --locked --package gbfr-logs repeat_quest::tests
npm test -- --run src/securityConfiguration.test.ts
cargo check --locked --package gbfr-logs
```

Expected: all repeat-quest and security tests pass; the Windows backend compiles without adding dependencies or changing the manifest.

- [ ] **Step 6: Commit the isolated Windows boundary**

```powershell
git add -- src-tauri/src/repeat_quest.rs src-tauri/src/equipment_probe/mod.rs src/securityConfiguration.test.ts
git commit -m "feat: control repeat quest patch safely"
```

---

### Task 4: Tauri commands, startup cleanup, and exit restoration

**Files:**
- Modify: `src-tauri/src/repeat_quest.rs`
- Modify: `src-tauri/src/main.rs`

**Interfaces:**
- Produces: serializable `RepeatQuestStatus { state, reason }` with camel-case fields and lower-case state values.
- Produces: `get_repeat_quest_status` and `set_repeat_quest_enabled(enabled)` Tauri commands.
- Produces: `restore_on_startup` and idempotent `restore_on_exit` lifecycle functions.
- Consumes later: `useRepeatQuest.ts` invokes the two command names exactly.

- [ ] **Step 1: Write failing state and lifecycle tests**

Add a private `RepeatQuestBackend: Send + Sync` interface with `observe`, `enable`, and `restore` methods. Production uses `LiveRepeatQuestBackend`; tests retain an `Arc<FakeBackend>` passed into `RepeatQuestState::with_backend`. No live process is touched:

```rust
#[test]
fn every_new_runtime_starts_with_off_as_the_desired_state() {
    let backend = Arc::new(FakeBackend::patched());
    let state = RepeatQuestState::with_backend(backend.clone());
    state.restore_on_startup();
    assert_eq!(backend.restore_calls(), 1);
    assert_eq!(state.status().state, RepeatQuestStatusKind::Off);
}

#[test]
fn normal_exit_restores_once_only_after_successful_enable() {
    let backend = Arc::new(FakeBackend::original());
    let state = RepeatQuestState::with_backend(backend.clone());
    assert_eq!(state.set_enabled(true).state, RepeatQuestStatusKind::On);
    state.restore_on_exit();
    state.restore_on_exit();
    assert_eq!(backend.restore_calls(), 1);
}

#[test]
fn a_busy_operation_returns_busy_without_a_second_write() {
    let backend = Arc::new(FakeBackend::original());
    let state = RepeatQuestState::with_backend(backend.clone());
    let _operation = state.lock_operation_for_test();
    assert_eq!(state.set_enabled(true).reason, Some(RepeatQuestReason::Busy));
    assert_eq!(backend.write_calls(), 0);
}
```

Also test error-to-reason mapping for game not running, unsupported game, absent/ambiguous signature, unexpected bytes, access denied, and patch/restoration failure.

- [ ] **Step 2: Run lifecycle tests and observe RED**

Run:

```powershell
cargo test --locked --package gbfr-logs repeat_quest::tests::normal_exit_restores_once_only_after_successful_enable
```

Expected: compilation fails because the managed state and lifecycle methods do not exist.

- [ ] **Step 3: Implement managed state and non-optimistic commands**

Use an `Arc`-backed state whose inner value contains an operation mutex, an atomic `may_be_patched` flag, and an atomic exit-cleanup guard. `status` takes the mutex normally so it observes startup cleanup or a preceding change; `set_enabled` uses `try_lock` and returns `Busy` for overlapping user changes. `restore_on_exit` waits for an active change, then restores once. Expose `lock_operation_for_test` only under `#[cfg(test)]`. The public response is:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepeatQuestStatus {
    pub state: RepeatQuestStatusKind,
    pub reason: Option<RepeatQuestReason>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RepeatQuestStatusKind { Unavailable, Off, On }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum RepeatQuestReason {
    Busy,
    GameNotRunning,
    UnsupportedGame,
    SignatureMissing,
    SignatureAmbiguous,
    UnexpectedBytes,
    AccessDenied,
    PatchFailed,
    RestoreFailed,
    Internal,
}
```

Both commands must return the observed backend status, not an optimistic boolean. Run blocking hash/scan/write work through `tauri::async_runtime::spawn_blocking` using a cloned `RepeatQuestState`:

```rust
#[tauri::command]
pub(crate) async fn get_repeat_quest_status(
    state: tauri::State<'_, RepeatQuestState>,
) -> RepeatQuestStatus {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.status())
        .await
        .unwrap_or_else(|_| RepeatQuestStatus::internal())
}
```

Implement `set_repeat_quest_enabled` the same way with an `enabled: bool` argument. Successful ON sets `may_be_patched`; successful OFF clears it. Any failure re-observes the sites where possible and returns that state with a reason.

- [ ] **Step 4: Wire startup, commands, and Tauri exit**

In `main.rs`:

- manage `repeat_quest::RepeatQuestState::default()`;
- register both commands in `generate_handler!`;
- run startup restoration synchronously from `.setup` before returning `Ok(())`, so the frontend cannot observe a stale patched state first;
- replace `Builder::run(context)` with `Builder::build(context)` followed by `App::run`;
- on `tauri::RunEvent::Exit`, synchronously call `restore_on_exit`.

Use this shape so tray `handle.exit(0)` flows through the same cleanup path:

```rust
let app = tauri::Builder::default()
    // existing plugins, state, events, commands, and setup stay unchanged
    .build(tauri::generate_context!())
    .expect("error while building tauri application");

app.run(|handle, event| {
    if matches!(event, tauri::RunEvent::Exit) {
        handle.state::<repeat_quest::RepeatQuestState>().restore_on_exit();
    }
});
```

Startup restoration may run before the game is connected and treats `GameNotRunning` as a harmless desired-OFF outcome. Later status queries still return `Unavailable(GameNotRunning)` while no game exists. Startup must not display an unsolicited dialog.

- [ ] **Step 5: Run backend lifecycle and existing main tests**

Run:

```powershell
cargo test --locked --package gbfr-logs repeat_quest::tests
cargo test --locked --package gbfr-logs tests::
cargo check --locked --package gbfr-logs
```

Expected: all lifecycle tests and existing main tests pass; Tauri builds with the new `App::run` shape.

- [ ] **Step 6: Commit the backend lifecycle**

```powershell
git add -- src-tauri/src/repeat_quest.rs src-tauri/src/main.rs
git commit -m "feat: restore repeat quest patch on exit"
```

---

### Task 5: Release settings switch and localized status

**Files:**
- Create: `src/pages/useRepeatQuest.ts`
- Create: `src/pages/Settings.repeatQuest.test.tsx`
- Modify: `src/pages/Settings.tsx`
- Modify: `src/pages/Settings.localization.test.ts`
- Modify: `src-tauri/lang/ko/ui.json`
- Modify: `src-tauri/lang/en/ui.json`

**Interfaces:**
- Consumes: `get_repeat_quest_status` and `set_repeat_quest_enabled` from Task 4.
- Produces: `useRepeatQuest(connectionState)` returning `{ status, pending, setEnabled }`.
- Produces: the non-persistent `무한 퀘스트 반복` switch and translated reason copy.

- [ ] **Step 1: Write failing frontend behavior tests**

Mock `@tauri-apps/api` and render `SettingsPage` in a `MantineProvider`. Cover:

```tsx
it("shows the release-visible repeat quest switch from backend state", async () => {
  mocks.status = { state: "off", reason: null };
  renderSettings();
  const toggle = await screen.findByRole("switch", { name: "무한 퀘스트 반복" });
  expect((toggle as HTMLInputElement).checked).toBe(false);
});

it("locks while enabling and uses the backend observed result", async () => {
  mocks.status = { state: "off", reason: null };
  mocks.setPromise = deferred<{ state: "on"; reason: null }>();
  renderSettings();
  const toggle = await screen.findByRole("switch", { name: "무한 퀘스트 반복" });
  fireEvent.click(toggle);
  expect((toggle as HTMLInputElement).disabled).toBe(true);
  await act(async () => mocks.setPromise.resolve({ state: "on", reason: null }));
  expect((toggle as HTMLInputElement).checked).toBe(true);
});

it("keeps the observed state and shows the reason after a failed toggle", async () => {
  mocks.status = { state: "off", reason: null };
  mocks.setResult = { state: "off", reason: "accessDenied" };
  renderSettings();
  fireEvent.click(await screen.findByRole("switch", { name: "무한 퀘스트 반복" }));
  expect(await screen.findByText("현재 권한으로 게임 코드를 변경할 수 없습니다.")).toBeTruthy();
});
```

Also test game-not-running and unsupported-game disabled reasons, invoke argument `{ enabled: true }`, and a connection-state change triggering a status refresh.

- [ ] **Step 2: Extend localization expectations and observe RED**

Add exact nested translation expectations to `Settings.localization.test.ts`:

```ts
const expectedKoreanGameFeatures = {
  title: "게임 기능",
  "repeat-quest": {
    label: "무한 퀘스트 반복",
    description: "퀘스트 반복 횟수 제한을 해제합니다.",
  },
};

const expectedEnglishGameFeatures = {
  title: "Game Features",
  "repeat-quest": {
    label: "Unlimited Repeat Quest",
    description: "Removes the quest repeat-count limit.",
  },
};
```

Run:

```powershell
npm test -- --run src/pages/Settings.repeatQuest.test.tsx src/pages/Settings.localization.test.ts
```

Expected: tests fail because the hook, UI, and translations do not exist.

- [ ] **Step 3: Add the non-persistent frontend hook**

Define the backend shape in `useRepeatQuest.ts`:

```ts
export type RepeatQuestStatus = {
  state: "unavailable" | "off" | "on";
  reason:
    | "busy"
    | "gameNotRunning"
    | "unsupportedGame"
    | "signatureMissing"
    | "signatureAmbiguous"
    | "unexpectedBytes"
    | "accessDenied"
    | "patchFailed"
    | "restoreFailed"
    | "internal"
    | null;
};
```

Initialize `status` as `null` so the switch stays disabled while the first query is pending. On mount and whenever `connectionState` changes, call `get_repeat_quest_status`. `setEnabled` sets local `pending`, invokes `set_repeat_quest_enabled` with `{ enabled }`, replaces local state with the returned observed status, and always clears `pending`. If invocation itself rejects, preserve the previous observed state and attach reason `internal`; do not invent an ON or OFF transition. Do not use Zustand or local storage.

- [ ] **Step 4: Add translations and the separate game-feature fieldset**

Under `ui.game-features`, add `title`, `repeat-quest.label`, `repeat-quest.description`, and reason strings in both JSON files. Korean reason copy must include:

- `gameNotRunning`: `게임이 실행 중이 아닙니다.`
- `unsupportedGame`: `지원하는 게임 2.0.2 실행 파일이 아닙니다.`
- `signatureMissing`: `무한 퀘스트 반복 코드를 찾지 못했습니다.`
- `signatureAmbiguous`: `무한 퀘스트 반복 코드 후보가 여러 개입니다.`
- `unexpectedBytes`: `게임 코드가 예상 상태와 달라 변경하지 않았습니다.`
- `accessDenied`: `현재 권한으로 게임 코드를 변경할 수 없습니다.`
- `patchFailed`: `무한 퀘스트 반복을 켜지 못했습니다.`
- `restoreFailed`: `원본 게임 코드를 복원하지 못했습니다.`
- `busy`: `이전 변경 작업이 아직 진행 중입니다.`
- `internal`: `무한 퀘스트 반복 처리 중 내부 오류가 발생했습니다.`

Import Mantine `Switch`. Wrap the existing meter fieldset and a new game-feature fieldset in a `Stack`. Bind `checked` only to `status.state === "on"`; disable for `pending` or `status.state === "unavailable"`; render the description and current reason as text. Do not place the feature behind debug mode.

- [ ] **Step 5: Run frontend tests, type checking, formatting, and lint**

Run:

```powershell
npm test -- --run src/pages/Settings.repeatQuest.test.tsx src/pages/Settings.localization.test.ts src/securityConfiguration.test.ts
npm run tsc
npm run format-check
npm run lint
```

Expected: focused tests pass, TypeScript reports no errors, Prettier reports all matched files formatted, and ESLint exits successfully.

- [ ] **Step 6: Commit the settings experience**

```powershell
git add -- src/pages/useRepeatQuest.ts src/pages/Settings.repeatQuest.test.tsx src/pages/Settings.tsx src/pages/Settings.localization.test.ts src-tauri/lang/ko/ui.json src-tauri/lang/en/ui.json
git commit -m "feat: add unlimited repeat quest switch"
```

---

### Task 6: Actual-game validation and full release regression

**Files:**
- Modify: `docs/testing/game-2.0.2-smoke-test.md`
- Modify after successful packaging only: `README.md`

**Interfaces:**
- Consumes: completed backend and frontend behavior from Tasks 1-5.
- Produces: manual 2.0.2 evidence and complete automated/release verification.

- [ ] **Step 1: Confirm the exact live target before any write**

With the game running in an offline or private session, verify:

```powershell
Get-CimInstance Win32_Process -Filter "Name='granblue_fantasy_relink.exe'" |
  Select-Object ProcessId, ExecutablePath
Get-FileHash -Algorithm SHA256 -LiteralPath 'D:\SteamLibrary\steamapps\common\Granblue Fantasy Relink\granblue_fantasy_relink.exe'
```

Expected executable hash: `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F`. Stop without toggling if it differs.

- [ ] **Step 2: Run the debug app without elevation and verify status**

Build the matching hook and start Tauri normally:

```powershell
cargo build --release --locked --package hook
Copy-Item -LiteralPath 'target\release\hook.dll' -Destination 'src-tauri\hook.dll' -Force
npm run tauri dev
```

Expected: no UAC prompt; Settings shows `게임 기능` and an enabled, initially OFF `무한 퀘스트 반복` switch. Backend logs must identify the verified PID/hash and unique sites without logging raw surrounding memory.

- [ ] **Step 3: Verify ON, OFF, and normal-exit restoration**

In the game UI:

1. Enable `무한 퀘스트 반복` and verify the switch reports ON only after read-back.
2. Repeat a quest beyond the normal repeat limit and record the observed behavior.
3. Disable the switch and verify it reports OFF after restoring both original byte sequences.
4. Enable it again, choose tray `종료`, restart Djeeta MOD while the game remains open, and verify the switch starts OFF with both sites original.
5. Restart the game and confirm the switch remains OFF until explicitly enabled.
6. With the feature OFF, close the game and refresh Settings; verify the switch becomes unavailable with the game-not-running reason and no write attempt.

If any target bytes are unknown, any signature count is not one, or restoration fails, stop and return to Task 2/3 debugging. Do not force-write and do not increase permissions automatically.

The controlled partial-state simulation is the Task 2 fake-memory rollback and mixed-state test. Do not deliberately corrupt the live game's code merely to reproduce a partial state.

- [ ] **Step 4: Record the manual result**

Add checklist rows to `docs/testing/game-2.0.2-smoke-test.md` for:

- default OFF/no UAC;
- successful ON beyond the normal repeat limit;
- explicit OFF restoration;
- tray-exit restoration;
- game restart returning to OFF;
- unsupported/missing-game reason without a write.

Fill the actual result and date only for scenarios personally observed. Leave unperformed items unchecked.

- [ ] **Step 5: Run the full required verification suite**

Load the VS 2022 developer environment if MSVC is absent, then run exactly:

```powershell
npm ci
npm run format-check
npm run lint
npm run tsc
npm test -- --run
npm run build
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
npm run tauri build -- --bundles nsis
```

Expected: every command exits 0. Do not describe the feature as complete if any command fails.

- [ ] **Step 6: Verify packaged hook equality and record release hashes**

Run:

```powershell
$builtHook = (Get-FileHash -Algorithm SHA256 -LiteralPath 'target\release\hook.dll').Hash
$bundledHook = (Get-FileHash -Algorithm SHA256 -LiteralPath 'src-tauri\hook.dll').Hash
if ($builtHook -ne $bundledHook) { throw 'hook.dll hash mismatch' }
Get-FileHash -Algorithm SHA256 -LiteralPath 'target\release\bundle\nsis\Djeeta MOD_0.1.0_x64-setup.exe'
```

Expected: hook hashes are identical. Update the installer and hook hashes in `README.md` and `docs/testing/game-2.0.2-smoke-test.md` only from these outputs.

- [ ] **Step 7: Review the final diff and commit verification evidence**

Run:

```powershell
git diff --check
git status --short
git diff -- src-tauri/src/repeat_quest.rs src-tauri/src/main.rs src/pages/useRepeatQuest.ts src/pages/Settings.tsx src/pages/Settings.repeatQuest.test.tsx src/pages/Settings.localization.test.ts src-tauri/lang/ko/ui.json src-tauri/lang/en/ui.json src/securityConfiguration.test.ts docs/testing/game-2.0.2-smoke-test.md README.md
```

Confirm no CT file, executable, raw memory dump, `logs.db`, or unrelated inventory work is staged. Then commit only verified evidence files that changed in Task 6:

```powershell
git add -- docs/testing/game-2.0.2-smoke-test.md README.md
git commit -m "docs: verify unlimited repeat quest toggle"
```

If `README.md` did not change, omit it from `git add`. If no manual/package evidence changed, do not create an empty commit.

# External Equipment Reader Probe Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 기존 훅을 정답 소스로 유지하면서, 개발 빌드의 명시적 옵트인 환경에서만 외부 읽기 전용 프로브가 로컬 캐릭터의 진 12칸을 독립적으로 찾아 훅 결과와 비교해 개발 로그에 기록한다.

**Architecture:** 장비 바이트 디코더는 새 `equipment-core` 공유 크레이트로 이동한다. Tauri 백엔드는 순수 시그니처·포인터 해석기, Windows 원격 메모리 어댑터, 안정된 이중 읽기와 비교 상태를 분리하며, 기존 훅 파이프는 변경 없이 정답 스냅샷을 비교기에 전달한다.

**Tech Stack:** Rust 2021, Tauri 1, `windows` 0.52, `sha2` 0.10, `anyhow`, 기존 `protocol` 이벤트, Vitest/TypeScript 회귀 검증, NSIS 패키징

## Global Constraints

- 대상은 `Granblue Fantasy: Relink Endless Ragnarok 2.0.2` Windows x64이고 검증 실행 파일 SHA-256은 `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F`이다.
- 외부 프로브는 `debug_assertions`가 활성화되고 `DJEETA_EXTERNAL_READER_PROBE=1`일 때만 실행한다.
- 외부 프로세스 권한은 `PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ`만 요청한다.
- 게임 메모리 쓰기, 코드 패치, 원격 스레드와 추가 DLL 주입을 구현하지 않는다.
- 훅은 주소를 외부 프로브에 전달하지 않으며 기존 `LocalEquipmentSnapshot` 정답만 제공한다.
- 외부 프로브는 사용자 UI, `HookStatus`, 미터 연결 상태와 `EquipmentStatus.connected`를 변경하지 않는다.
- 프로브 폴링 간격은 250ms, 안정성 확인용 이중 읽기 간격은 50ms, 동일 실패 로그 제한은 5초이다.
- 런타임 절대 주소를 저장하지 않고 고유 시그니처와 RIP-relative 참조로 매 프로세스 시작마다 루트를 다시 찾는다.
- `protocol::Message` variant 순서를 변경하지 않는다.
- 현재 작업 트리의 `logs.db`는 사용자 소유 파일이므로 읽기·수정·스테이징하지 않는다.
- 사용자가 지정한 기본 실행 방식에 따라 별도 worktree나 subagent 없이 현재 세션에서 인라인 실행한다.
- 실제 게임 2.0.2 호환성은 수동 체크리스트가 완료되기 전에는 주장하지 않는다.

---

## File Map

- Create `equipment-core/Cargo.toml`: 훅과 백엔드가 공유하는 장비 디코더 크레이트 정의.
- Create `equipment-core/src/lib.rs`: 진 배열 상수, 검증 오류와 `decode_snapshot` 제공.
- Modify `Cargo.toml`: `equipment-core`를 workspace member로 추가.
- Modify `src-hook/Cargo.toml`: 공유 디코더 의존성 추가.
- Modify `src-hook/src/hooks/equipment.rs`: 프로세스 읽기·발행만 유지하고 디코더를 공유 크레이트에서 사용.
- Modify `src-tauri/Cargo.toml`: `equipment-core`, `windows`, `sha2` 의존성 추가.
- Create `src-tauri/src/equipment_probe/mod.rs`: 활성화 게이트, 폴링 실행기와 외부 공개 인터페이스.
- Create `src-tauri/src/equipment_probe/locator.rs`: 시그니처, rel32, 플레이어 해시 테이블과 장비 주소 해석.
- Create `src-tauri/src/equipment_probe/memory.rs`: 테스트 가능한 `MemoryReader`, PE `.text` 파서와 Windows 원격 읽기 구현.
- Create `src-tauri/src/equipment_probe/compare.rs`: 훅 캐시, 안정된 읽기 판정, 차이와 중복 로그 억제.
- Modify `src-tauri/src/main.rs`: 프로브 상태 관리, 훅 정답 전달과 개발 전용 실행기 spawn.
- Modify `src/securityConfiguration.test.ts`: 외부 프로브가 읽기 권한만 사용한다는 소스 회귀 테스트.
- Modify `docs/testing/game-2.0.2-equipment-layout.md`: 외부 경로와 실제 게임 검증 결과 기록.
- Modify `docs/testing/game-2.0.2-smoke-test.md`: 프로브 수동 검증 항목과 결과 기록.

---

### Task 1: Extract the shared sigil snapshot decoder

**Files:**
- Create: `equipment-core/Cargo.toml`
- Create: `equipment-core/src/lib.rs`
- Modify: `Cargo.toml`
- Modify: `src-hook/Cargo.toml`
- Modify: `src-hook/src/hooks/equipment.rs`

**Interfaces:**
- Produces: `equipment_core::decode_snapshot(bytes: &[u8], character_key: u32) -> Result<LocalEquipmentSnapshotEvent, DecodeError>`
- Produces: `equipment_core::{EMPTY_HASH, SIGIL_ARRAY_BYTES, SIGIL_COUNT, SIGIL_STRIDE}`
- Consumes: existing `protocol::{EquipmentCaptureStatus, EquipmentSourceKind, EquippedTraitSource, LocalEquipmentSnapshotEvent}`

- [ ] **Step 1: Add failing decoder tests to the new crate**

Create the crate manifest with dependencies on `protocol` and `thiserror`. In `equipment-core/src/lib.rs`, first add tests that construct a `0x1B0` fixture and assert:

```rust
#[test]
fn decodes_primary_and_secondary_traits() {
    let bytes = fixture_with_sigil();
    let event = decode_snapshot(&bytes, CHARACTER_KEY).unwrap();
    assert_eq!(event.character_type, CHARACTER_KEY);
    assert_eq!(event.sources.len(), 2);
    assert_eq!(event.sources[0].kind, EquipmentSourceKind::SigilPrimary);
    assert_eq!(event.sources[1].kind, EquipmentSourceKind::SigilSecondary);
}

#[test]
fn rejects_partial_array_and_wrong_character() {
    assert!(decode_snapshot(&vec![0; SIGIL_ARRAY_BYTES - 1], CHARACTER_KEY).is_err());
    let mut bytes = fixture_with_sigil();
    put_u32(&mut bytes, 0x14, 0x079D_F0CC);
    assert!(decode_snapshot(&bytes, CHARACTER_KEY).is_err());
}
```

- [ ] **Step 2: Run the new crate test and verify RED**

Run: `cargo test --package equipment-core`

Expected: compilation fails because `equipment-core` is not yet a workspace member and `decode_snapshot` is not implemented.

- [ ] **Step 3: Move the minimal decoder implementation**

Expose the existing constants and implement a typed error without process or logging dependencies:

```rust
pub const EMPTY_HASH: u32 = 0x887A_E0B0;
pub const SIGIL_COUNT: usize = 12;
pub const SIGIL_STRIDE: usize = 0x24;
pub const SIGIL_ARRAY_BYTES: usize = SIGIL_COUNT * SIGIL_STRIDE;
const MAX_TRAIT_LEVEL: u32 = 10_000;

pub fn decode_snapshot(
    bytes: &[u8],
    character_key: u32,
) -> Result<LocalEquipmentSnapshotEvent, DecodeError>;
```

Preserve the current empty hash, character ownership and `1..=10_000` trait-level validation exactly. Register the crate in the root workspace and replace the hook-local decoder with imports from `equipment_core`.

Run `cargo test --package equipment-core` once after adding the manifests so Cargo updates `Cargo.lock`; all subsequent commands use `--locked`.

- [ ] **Step 4: Verify GREEN and hook regression**

Run:

```powershell
cargo test --locked --package equipment-core
cargo test --locked --package hook hooks::equipment
```

Expected: all moved decoder tests and existing cache/publish tests pass.

- [ ] **Step 5: Commit the shared decoder**

```powershell
git add Cargo.toml Cargo.lock equipment-core src-hook/Cargo.toml src-hook/src/hooks/equipment.rs
git commit -m "refactor: share equipment snapshot decoder"
```

---

### Task 2: Implement the pure 2.0.2 player locator

**Files:**
- Create: `src-tauri/src/equipment_probe/locator.rs`
- Create: `src-tauri/src/equipment_probe/mod.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/Cargo.toml`

**Interfaces:**
- Consumes: `MemoryReader::read_exact(address: usize, output: &mut [u8]) -> Result<(), MemoryReadError>` from Task 3; define the trait in `locator.rs` initially and move it without changing the signature in Task 3.
- Produces: `find_unique_pattern(haystack: &[u8], pattern: &[Option<u8>]) -> Result<usize, LocateError>`
- Produces: `resolve_rel32(instruction_address: usize, displacement: i32, instruction_len: usize) -> Result<usize, LocateError>`
- Produces: `locate_equipment<R: MemoryReader>(reader: &R, module_base: usize, text_rva: usize, text: &[u8]) -> Result<LocatedEquipment, LocateError>`
- Produces: `LocatedEquipment { character_key: u32, record_address: usize, snapshot_address: usize }`

- [ ] **Step 1: Write failing signature and ASLR tests**

Use a synthetic `.text` fixture containing the verified 2.0.2 lookup sequence:

```text
56 57 48 83 EC 38 48 8B 31
48 8B 0D ?? ?? ?? ??
C6 44 24 30 00 C6 44 24 28 00 C6 44 24 20 00
31 D2 45 31 C0 45 31 C9
E8 ?? ?? ?? ??
80 B8 BC 5E 00 00 00 B9 B0 E0 7A 88 74 ?? 8B 88 A8 5E 00 00
```

Add tests for unique, zero and duplicate matches, then run the same fixture at module bases `0x140000000` and `0x180000000`. Assert that resolved absolute addresses move by the ASLR delta while the matched RVA remains unchanged.

- [ ] **Step 2: Run locator tests and verify RED**

Run: `cargo test --locked --package gbfr-logs equipment_probe::locator`

Expected: compilation fails because `equipment_probe::locator` and its functions do not exist.

- [ ] **Step 3: Implement pattern parsing and rel32 resolution**

Implement exact byte-or-wildcard matching. Reject any match count other than one. Resolve the local-key global from the `mov rcx, [rip+rel32]` instruction at match offset `0x09`, using its displacement at `0x0C` and next-instruction offset `0x10`. Resolve the getter call at match offset `0x27`, using displacement offset `0x28` and next-instruction offset `0x2C`.

Validate the getter prologue:

```text
41 57 41 56 41 55 41 54 56 57 55 53 48 83 EC 68
```

Then validate `48 8B 35` at getter offset `0x44` and resolve the player-manager global using the displacement at `0x47` and next-instruction offset `0x4B`.

The pinned executable provides the following static-analysis evidence. Keep it in test comments for auditability, but do not hardcode these RVAs into runtime lookup:

```text
lookup signature match RVA: 0x002D7E60
local-key global reference RVA: 0x07C23878
player-manager global reference RVA: 0x07C24980
refresh-player function match RVA: 0x00A2B600
```

- [ ] **Step 4: Write failing player hash-table tests**

Construct a fake address space with:

```text
*local_key_global -> local_keys
u32(local_keys + 0x00) = character_key
*manager_global -> manager
u32(manager + 0xA58) = mask
*(manager + 0xA30) = sentinel
*(manager + 0xA40) = buckets
*(buckets + index * 0x10 + 0x08) = node
u32(node + 0x10) = character_key
*(node + 0x30) = record
*(record + 0x5E60) = snapshot
u32(record + 0x5EA8) = character_key
```

Assert successful location and explicit rejection of null globals, sentinel-only buckets, linked-list cycles, node-key mismatch and record-key mismatch.

- [ ] **Step 5: Run hash-table tests and verify RED**

Run: `cargo test --locked --package gbfr-logs equipment_probe::locator::tests::locates_record_in_player_hash_table`

Expected: test fails because the hash-table traversal is not implemented.

- [ ] **Step 6: Implement bounded hash-table traversal**

Use the game’s verified lookup layout:

```rust
let index = usize::from(mask & character_key);
let bucket_start = read_usize(reader, buckets + index * 0x10)?;
let mut node = read_usize(reader, buckets + index * 0x10 + 0x08)?;
for _ in 0..=1024 {
    if node == sentinel || node == bucket_start {
        return Err(LocateError::PlayerNotFound);
    }
    if read_u32(reader, node + 0x10)? == character_key {
        break;
    }
    node = read_usize(reader, node + 0x08)?;
}
```

After locating the record at `node + 0x30`, verify `record + 0x5EA8` equals the local key and read the snapshot pointer at `record + 0x5E60`. Reject null and non-canonical user pointers before every dereference.

- [ ] **Step 7: Verify locator GREEN**

Run: `cargo test --locked --package gbfr-logs equipment_probe::locator`

Expected: unique signature, ASLR, pointer validation and hash-table tests all pass.

- [ ] **Step 8: Commit the pure locator**

```powershell
git add src-tauri/Cargo.toml Cargo.lock src-tauri/src/main.rs src-tauri/src/equipment_probe/mod.rs src-tauri/src/equipment_probe/locator.rs
git commit -m "feat: locate external equipment snapshot"
```

---

### Task 3: Add the read-only Windows process adapter

**Files:**
- Create: `src-tauri/src/equipment_probe/memory.rs`
- Modify: `src-tauri/src/equipment_probe/locator.rs`
- Modify: `src-tauri/src/equipment_probe/mod.rs`
- Modify: `src-tauri/Cargo.toml`

**Interfaces:**
- Produces: `MemoryReader` in `memory.rs` with the unchanged `read_exact` signature.
- Produces: `RemoteProcess::find(name: &str) -> Result<Option<RemoteProcess>, MemoryReadError>`
- Produces: `RemoteProcess::{pid, module_base, module_size, module_path, read_text_section}`
- Produces: `parse_text_section(headers: &[u8]) -> Result<PeSection, MemoryReadError>`
- Consumes: `locator::locate_equipment` from Task 2.

- [ ] **Step 1: Write failing PE section and exact-read tests**

Add a minimal PE32+ header fixture with `.text` at RVA `0x1000`, virtual size `0x2000`. Assert that `parse_text_section` returns exactly those values and rejects DOS magic errors, PE magic errors, missing `.text`, overflowing ranges and truncated section tables.

Add a fake reader that returns fewer bytes than requested and assert `read_usize` and `read_u32` return `MemoryReadError::PartialRead` rather than accepting zero-filled output.

- [ ] **Step 2: Run memory tests and verify RED**

Run: `cargo test --locked --package gbfr-logs equipment_probe::memory`

Expected: compilation fails because `memory.rs`, `RemoteProcess` and the PE parser do not exist.

- [ ] **Step 3: Implement Windows handle and module discovery**

Add `windows = 0.52.0` features:

```toml
windows = { version = "0.52.0", features = [
  "Win32_Foundation",
  "Win32_System_Diagnostics_Debug",
  "Win32_System_Diagnostics_ToolHelp",
  "Win32_System_Threading"
] }
sha2 = "0.10"
```

Enumerate processes with `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)` and correctly inspect both `Process32FirstW` and subsequent `Process32NextW` entries. Find the main module using `TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32`. Open it only with:

```rust
OpenProcess(
    PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ,
    false,
    pid,
)
```

Wrap process and snapshot handles in RAII types that call `CloseHandle` exactly once.

- [ ] **Step 4: Implement strict remote reads and `.text` capture**

Call `ReadProcessMemory` and require `bytes_read == output.len()`. Read enough remote PE headers to parse the section table, then copy only the remote `.text` virtual range into a local `Vec<u8>`. Do not request write, operation, thread-creation or all-access rights.

Hash `module_path` with `sha2::Sha256` before attempting location and compare the uppercase digest with the pinned 2.0.2 hash.

- [ ] **Step 5: Verify memory adapter GREEN**

Run:

```powershell
cargo test --locked --package gbfr-logs equipment_probe::memory
cargo test --locked --package gbfr-logs equipment_probe::locator
```

Expected: all pure adapter/locator tests pass without a running game.

- [ ] **Step 6: Commit the read-only adapter**

```powershell
git add src-tauri/Cargo.toml Cargo.lock src-tauri/src/equipment_probe
git commit -m "feat: add read-only game memory adapter"
```

---

### Task 4: Compare stable external snapshots with hook truth

**Files:**
- Create: `src-tauri/src/equipment_probe/compare.rs`
- Modify: `src-tauri/src/equipment_probe/mod.rs`
- Modify: `src-tauri/src/main.rs`

**Interfaces:**
- Produces: `ProbeComparator::record_hook(LocalEquipmentSnapshotEvent)`
- Produces: `ProbeComparator::compare_external(character_key: u32, first: &[u8], second: &[u8], now: Instant) -> CompareDecision`
- Produces: `CompareDecision::{Match(ComparisonSummary), Mismatch(Vec<SlotDifference>), Deferred(DeferredReason), Suppressed}`
- Consumes: `equipment_core::decode_snapshot` and `SIGIL_ARRAY_BYTES`.

- [ ] **Step 1: Write failing stability and comparison tests**

Add tests asserting:

```rust
assert!(matches!(
    comparator.compare_external(KEY, &first, &changed, now),
    CompareDecision::Deferred(DeferredReason::UnstableRead)
));

comparator.record_hook(decode_snapshot(&first, KEY).unwrap());
assert!(matches!(
    comparator.compare_external(KEY, &first, &first, now),
    CompareDecision::Match(_)
));
```

Change one primary level and assert the mismatch contains only that slot and field. Repeat the same result before five seconds and assert `Suppressed`; repeat at five seconds and assert it is emitted again.

- [ ] **Step 2: Run comparator tests and verify RED**

Run: `cargo test --locked --package gbfr-logs equipment_probe::compare`

Expected: compilation fails because the comparator types do not exist.

- [ ] **Step 3: Implement minimal comparison state**

Store hook truth by `character_type`. Reject unequal external byte arrays before decoding. Use the shared decoder for stable arrays, compare normalized `LocalEquipmentSnapshotEvent.sources`, and generate slot differences for `item_id`, trait IDs, trait levels and character key only.

Calculate the logged snapshot digest from the stable `0x1B0` bytes and expose only the first 16 lowercase hexadecimal SHA-256 characters. Never include the raw bytes or display name in `Debug` or log output.

- [ ] **Step 4: Verify comparator GREEN**

Run: `cargo test --locked --package gbfr-logs equipment_probe::compare`

Expected: stability, match, mismatch, privacy and throttling tests pass.

- [ ] **Step 5: Route hook truth without changing existing behavior**

In the existing `Message::LocalEquipmentSnapshot(event)` arm, clone the event into the probe comparator before applying the original event to `EquipmentStatus`. Preserve the original React event and response exactly:

```rust
equipment_probe::record_hook_snapshot(&app, event.clone());
let response = {
    let equipment = app.state::<EquipmentStatus>();
    let mut equipment = equipment.0.lock().unwrap();
    equipment.apply(event);
    equipment.response()
};
```

- [ ] **Step 6: Run backend regression tests**

Run: `cargo test --locked --package gbfr-logs --all-targets`

Expected: comparator and all existing backend tests pass.

- [ ] **Step 7: Commit the comparator**

```powershell
git add src-tauri/src/equipment_probe src-tauri/src/main.rs
git commit -m "feat: compare external and hook equipment"
```

---

### Task 5: Add the opt-in development probe runner

**Files:**
- Modify: `src-tauri/src/equipment_probe/mod.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src/securityConfiguration.test.ts`

**Interfaces:**
- Produces: `probe_enabled(debug_build: bool, env_value: Option<&str>) -> bool`
- Produces: `run_if_enabled(app: AppHandle) -> impl Future<Output = ()>`
- Consumes: `RemoteProcess`, `locate_equipment`, `ProbeComparator`.

- [ ] **Step 1: Write failing activation tests**

```rust
#[test]
fn probe_requires_debug_build_and_exact_opt_in() {
    assert!(probe_enabled(true, Some("1")));
    assert!(!probe_enabled(true, None));
    assert!(!probe_enabled(true, Some("true")));
    assert!(!probe_enabled(false, Some("1")));
}
```

- [ ] **Step 2: Run activation test and verify RED**

Run: `cargo test --locked --package gbfr-logs probe_requires_debug_build_and_exact_opt_in`

Expected: compilation fails because `probe_enabled` does not exist.

- [ ] **Step 3: Implement the gated polling loop**

At app setup, manage a `ProbeComparator` state and spawn `run_if_enabled(app.clone())`. The function must return immediately unless both compile-time debug mode and the exact environment value pass.

When enabled:

1. Wait one second between process searches.
2. Log PID, uppercase executable SHA-256, module base and requested rights once per process.
3. Reject a hash other than the pinned 2.0.2 value before reading `.text` or following pointers.
4. Locate the snapshot from the unique signature and player hash table, then log the signature match count `1`, match RVA, local-key global RVA and player-manager global RVA once per process.
5. Read exactly `SIGIL_ARRAY_BYTES`.
6. Wait 50ms and repeat location plus exact read so a record replacement is also detected.
7. Submit both arrays to the comparator.
8. Wait until the 250ms polling deadline before the next candidate.
9. On process exit, drop the handle and restart discovery from the process-search step.

Log fixed event names `PROBE MATCH`, `PROBE MISMATCH`, `PROBE DEFERRED` and `PROBE UNAVAILABLE`. Apply the same five-second key-based throttle to repeated `DEFERRED` and `UNAVAILABLE` results. A probe error must never call connection-state or equipment-state update functions.

- [ ] **Step 4: Add source-level security regression tests**

Extend `src/securityConfiguration.test.ts` with this exact source check:

```ts
test("external equipment probe requests read-only process access", () => {
  const source = readRepositoryFile("src-tauri/src/equipment_probe/memory.rs");
  expect(source).toContain("PROCESS_VM_READ");
  expect(source).toContain("PROCESS_QUERY_LIMITED_INFORMATION");
  for (const forbidden of [
    "PROCESS_VM_WRITE",
    "PROCESS_VM_OPERATION",
    "PROCESS_CREATE_THREAD",
    "WriteProcessMemory",
    "VirtualAllocEx",
    "CreateRemoteThread",
  ]) {
    expect(source).not.toContain(forbidden);
  }
});
```

- [ ] **Step 5: Verify runner GREEN**

Run:

```powershell
cargo test --locked --package gbfr-logs --all-targets
npm test -- --run
```

Expected: activation, security regression and existing frontend tests pass.

- [ ] **Step 6: Commit the development runner**

```powershell
git add src-tauri/src/equipment_probe src-tauri/src/main.rs src/securityConfiguration.test.ts
git commit -m "feat: run opt-in external equipment probe"
```

---

### Task 6: Run automated verification before live-game probing

**Files:**
- Modify only files required to correct failures caused by Tasks 1-5.

**Interfaces:**
- Consumes: completed external probe implementation.
- Produces: a clean automated verification record before touching live-game state.

- [ ] **Step 1: Run formatting and static checks**

```powershell
npm run format-check
npm run lint
npm run tsc
cargo fmt --all -- --check
```

Expected: all commands exit `0` without warnings caused by the new code.

- [ ] **Step 2: Run all automated tests**

```powershell
npm test -- --run
cargo test --workspace --all-targets --locked
```

Expected: all frontend and Rust tests pass.

- [ ] **Step 3: Run production builds**

Load the Visual Studio developer environment if MSVC is not already available, then run:

```powershell
npm run build
cargo build --release --locked --package hook
```

Expected: frontend and release hook builds succeed.

- [ ] **Step 4: Commit only failure corrections if needed**

If verification required code changes, repeat Steps 1-3 and commit only those corrections:

```powershell
git add equipment-core Cargo.toml Cargo.lock src-hook/Cargo.toml src-hook/src/hooks/equipment.rs src-tauri/Cargo.toml src-tauri/src/equipment_probe src-tauri/src/main.rs src/securityConfiguration.test.ts
git commit -m "fix: satisfy external probe verification"
```

Do not stage `logs.db`.

---

### Task 7: Validate against the live 2.0.2 game

**Files:**
- Modify: `docs/testing/game-2.0.2-equipment-layout.md`
- Modify: `docs/testing/game-2.0.2-smoke-test.md`

**Interfaces:**
- Consumes: debug Tauri app with the opt-in probe and the existing hook truth stream.
- Produces: recorded `MATCH` evidence across three fresh processes or a precise blocking diagnosis.

- [ ] **Step 1: Start the debug app with the exact opt-in**

With the game initially closed, run:

```powershell
$env:DJEETA_EXTERNAL_READER_PROBE = '1'
npm run tauri dev
```

Expected: the app waits for the game; the probe does not report write or injection permissions.

- [ ] **Step 2: Validate process run 1**

Start the game normally. Select two different local characters, replace one sigil for each, then restore it. For every stable state, require `PROBE MATCH` with the expected character key and a changed/restored snapshot hash. Any `MISMATCH` must be diagnosed before proceeding.

- [ ] **Step 3: Validate process runs 2 and 3**

Completely exit the game between runs. Repeat the observation for two more fresh game processes. Record each PID, module base, unique signature RVA, character keys and `MATCH` counts. Confirm module bases may differ while signature RVA and decoded results remain stable.

- [ ] **Step 4: Validate disabled and release behavior**

Close the debug app, remove the environment variable and run it again. Confirm no `PROBE` startup log appears. Build or run the release binary with the environment variable set and confirm the probe still does not start.

- [ ] **Step 5: Record the evidence**

Append the exact game hash, signature match count, resolved relative-path layout, three-run results and any known limitations to both testing documents. Mark only the external equipment probe items complete; do not claim full 2.0.2 compatibility unless the entire smoke checklist is complete.

- [ ] **Step 6: Commit live verification docs**

```powershell
git add docs/testing/game-2.0.2-equipment-layout.md docs/testing/game-2.0.2-smoke-test.md
git commit -m "docs: verify external equipment probe"
```

---

### Task 8: Run final verification and package the unchanged release behavior

**Files:**
- Modify packaging hash documentation only if the generated distributable is intentionally selected to replace the current package.

**Interfaces:**
- Consumes: live-validated implementation and docs.
- Produces: final automated evidence and NSIS artifact.

- [ ] **Step 1: Run the complete required verification**

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

Expected: every command exits `0`.

- [ ] **Step 2: Verify packaged hook identity**

Calculate SHA-256 for `target/release/hook.dll` and `src-tauri/hook.dll` and require exact equality before accepting the NSIS artifact.

- [ ] **Step 3: Verify release probe exclusion**

Run the packaged release with `DJEETA_EXTERNAL_READER_PROBE=1` while the game is closed and inspect the application log. Require that no external probe startup or process-open log appears.

- [ ] **Step 4: Inspect the final diff and worktree**

```powershell
git diff --check
git status --short
git log --oneline -10
```

Expected: no unstaged implementation changes; only the pre-existing untracked `logs.db` may remain.

- [ ] **Step 5: Record package hashes only when replacing the distributable**

If this NSIS package becomes the new distributable, update `README.md` and `docs/testing/game-2.0.2-smoke-test.md` with the NSIS and hook SHA-256 values, rerun `git diff --check`, and commit:

```powershell
git add README.md docs/testing/game-2.0.2-smoke-test.md
git commit -m "docs: record external probe package hashes"
```

If the package is only a local verification artifact, leave the existing published hashes unchanged.

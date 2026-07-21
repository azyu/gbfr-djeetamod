# Full-Roster Validation Probe Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Determine, without changing product UI or game state, whether the pinned 2.0.2 player-manager table can safely and completely enumerate every character available in the active local save.

**Architecture:** Extend the existing debug-only external equipment probe with a pure, bounded manager-table inspector. Treat the current manager offsets and `mask + 1` bucket count as a development hypothesis, log only keys/counts/statuses/short digests, and require three fresh-process comparisons with the game UI before designing or enabling a production reader.

**Tech Stack:** Rust nightly-2024-05-04, Tauri 1, existing `MemoryReader`/`RemoteProcess`, `equipment-core`, Vitest security tests, Windows `PROCESS_QUERY_INFORMATION | PROCESS_VM_READ`.

## Global Constraints

- Target only game 2.0.2 with SHA-256 `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F`.
- Run only when `cfg!(debug_assertions)` and `DJEETA_EXTERNAL_READER_PROBE=1` are both true.
- Never write game memory, saves, or inputs.
- Do not change protocol, hook, production equipment response, React UI, or packaged release behavior in this milestone.
- Do not assume the manager table is the complete available-character roster or invent an availability offset/value.
- Do not log player names, raw sigil/trait values, reusable addresses, or save contents.
- Keep existing four-party probing and callback equipment analysis unchanged.
- Do not read, modify, stage, or commit `logs.db`.
- Product activation remains blocked until Task 5 passes.

## Scope Split

This plan implements only the independently testable validation milestone. A `PASS` in Task 5 is required before writing a second implementation plan for the approved design's production state model, five-second reader, `unavailable` UI, localization, and callback-first merge behavior. A `FAIL` ends this work at the debug probe and must not leave speculative production constants or inactive product code behind.

## File Structure

- Modify `src-tauri/src/parser/constants.rs`: exact equipment-character hash predicate.
- Create `src-tauri/src/equipment_probe/roster_probe.rs`: bounded inspection, stable classification, sanitized evidence.
- Modify `src-tauri/src/equipment_probe/locator.rs`: sibling-only visibility for existing checked-read helpers.
- Modify `src-tauri/src/equipment_probe/mod.rs`: five-second debug evidence pass.
- Modify `src/securityConfiguration.test.ts`: read-only and debug-gate regression.
- Modify `docs/testing/game-2.0.2-equipment-layout.md`: factual evidence after manual validation only.

---

### Task 1: Classify equipment-character hashes

**Files:**
- Modify/Test: `src-tauri/src/parser/constants.rs`

**Interfaces:**
- Produces: `pub(crate) fn is_known_equipment_character_hash(hash: u32) -> bool`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn equipment_hash_filter_rejects_actor_and_pet_hashes() {
    assert!(super::is_known_equipment_character_hash(0x2A26_B1B2));
    assert!(super::is_known_equipment_character_hash(0x74DD_4C79));
    assert!(!super::is_known_equipment_character_hash(0x26A4_848A));
    assert!(!super::is_known_equipment_character_hash(0x2AF6_78E8));
    assert!(!super::is_known_equipment_character_hash(0x8364_C8BC));
    assert!(!super::is_known_equipment_character_hash(0x887A_E0B0));
}
```

- [ ] **Step 2: Verify RED**

Run: `cargo test --locked --package gbfr-logs parser::constants::tests::equipment_hash_filter_rejects_actor_and_pet_hashes`

Expected: compile failure because the predicate does not exist.

- [ ] **Step 3: Implement the exact predicate**

```rust
const EQUIPMENT_CHARACTER_HASHES: [u32; 30] = [
    0x2A26_B1B2, 0xA4AC_BA76, 0x18E2_F9F9, 0x079D_F0CC, 0x4D0A_60C3,
    0xDD7A_151E, 0xC861_6284, 0xC3FF_D418, 0x22E4_37E5, 0x2EBE_91D5,
    0xBDEF_7181, 0x627B_CB0D, 0xFD3B_E362, 0xFC6C_DF7B, 0xE705_3919,
    0x978E_4B18, 0x0D21_B430, 0xF0EB_77EF, 0xAA66_178A, 0xA3A3_CB2F,
    0xF92C_7821, 0x718E_1A14, 0x2964_71BE, 0xBAD1_6E3B, 0x1BB3_7EF0,
    0x25D4_6F4B, 0x9A8A_F295, 0x9B15_CFB1, 0x646C_3168, 0x74DD_4C79,
];

pub(crate) fn is_known_equipment_character_hash(hash: u32) -> bool {
    EQUIPMENT_CHARACTER_HASHES.contains(&hash)
}
```

This identifies candidate equipment records only; it does not establish unlock state.

- [ ] **Step 4: Verify GREEN**

Run: `cargo test --locked --package gbfr-logs parser::constants::tests`

Expected: all constants tests pass.

- [ ] **Step 5: Commit**

```powershell
git add -- src-tauri/src/parser/constants.rs
git commit -m "test: classify equipment character hashes"
```

---

### Task 2: Inspect candidate manager records with bounded traversal

**Files:**
- Create/Test: `src-tauri/src/equipment_probe/roster_probe.rs`
- Modify: `src-tauri/src/equipment_probe/mod.rs`
- Modify: `src-tauri/src/equipment_probe/locator.rs`

**Interfaces:**

```rust
pub(crate) struct CandidateRecord {
    pub character_key: u32,
    pub snapshot_address: Option<usize>,
}

pub(crate) struct ManagerInspection {
    pub candidate_bucket_count: usize,
    pub records: Vec<CandidateRecord>,
    pub duplicate_keys: Vec<u32>,
    pub rejected_record_count: usize,
}

pub(crate) fn inspect_candidate_manager<R: MemoryReader>(
    reader: &R,
    manager_global: usize,
) -> Result<ManagerInspection, RosterProbeError>;
```

- [ ] **Step 1: Expose existing checked-read helpers**

Change only visibility in `locator.rs`:

```rust
pub(super) fn checked_address(base: usize, offset: usize) -> Result<usize, LocateError>;
pub(super) fn validate_pointer(address: usize) -> Result<usize, LocateError>;
pub(super) fn read_u32<R: MemoryReader>(reader: &R, address: usize) -> Result<u32, LocateError>;
pub(super) fn read_usize<R: MemoryReader>(reader: &R, address: usize) -> Result<usize, LocateError>;
```

Do not change bodies or party-locator call sites.

- [ ] **Step 2: Register `mod roster_probe;` and write RED tests**

Use a `BTreeMap<usize, u8>` fake `MemoryReader`. Its fixture must encode the observed candidate layout: sentinel `manager+0xA30`, buckets `manager+0xA40`, mask `manager+0xA58`, bucket stride `0x10`, node next/key/record at `+0x08/+0x10/+0x30`, record self-key/snapshot at `+0x5EA8/+0x5E60`.

```rust
#[test]
fn inspects_empty_single_and_collision_candidate_buckets() {
    let memory = manager_fixture_with_three_known_records();
    let result = inspect_candidate_manager(&memory, MANAGER_GLOBAL).unwrap();
    assert_eq!(result.candidate_bucket_count, 4);
    assert_eq!(
        result.records.iter().map(|entry| entry.character_key).collect::<Vec<_>>(),
        vec![0x74DD_4C79, 0x9B15_CFB1, 0xE705_3919]
    );
    assert!(result.duplicate_keys.is_empty());
    assert_eq!(result.rejected_record_count, 0);
}

#[test]
fn bounds_candidate_bucket_and_node_traversal() {
    assert!(matches!(
        inspect_candidate_manager(&manager_with_mask(4096), MANAGER_GLOBAL),
        Err(RosterProbeError::CandidateBucketLimit(4097))
    ));
    assert!(matches!(
        inspect_candidate_manager(&manager_with_cycle(), MANAGER_GLOBAL),
        Err(RosterProbeError::LinkedListCycle(_))
    ));
    assert!(matches!(
        inspect_candidate_manager(&manager_with_1025_nodes(), MANAGER_GLOBAL),
        Err(RosterProbeError::TraversalLimit)
    ));
}

#[test]
fn preserves_membership_without_a_valid_snapshot_and_reports_duplicates() {
    let result = inspect_candidate_manager(
        &manager_with_duplicate_and_invalid_snapshot(0xE705_3919),
        MANAGER_GLOBAL,
    ).unwrap();
    assert_eq!(result.duplicate_keys, vec![0xE705_3919]);
    assert_eq!(result.records[0].snapshot_address, None);
}

#[test]
fn rejects_a_self_key_mismatch() {
    let result = inspect_candidate_manager(
        &manager_with_self_key_mismatch(0xE705_3919, 0x9B15_CFB1),
        MANAGER_GLOBAL,
    ).unwrap();
    assert!(result.records.is_empty());
    assert_eq!(result.rejected_record_count, 1);
}
```

- [ ] **Step 3: Verify RED**

Run: `cargo test --locked --package gbfr-logs equipment_probe::roster_probe::tests`

Expected: compile failure because the module types and inspector do not exist.

- [ ] **Step 4: Implement bounded inspection**

```rust
const CANDIDATE_BUCKET_STRIDE: usize = 0x10;
const MAX_CANDIDATE_BUCKETS: usize = 4096;
const MAX_CANDIDATE_NODES: usize = 1024;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(crate) enum RosterProbeError {
    #[error(transparent)]
    Locate(#[from] LocateError),
    #[error("candidate manager bucket count {0} exceeds the development safety limit")]
    CandidateBucketLimit(usize),
    #[error("candidate manager list contains a cycle at {0:#x}")]
    LinkedListCycle(usize),
    #[error("candidate manager traversal exceeded 1024 nodes")]
    TraversalLimit,
}
```

Dereference the manager global; compute checked `mask + 1`; reject more than 4096 candidate buckets; traverse with one global visited-node set and 1024-node limit; require node key = record self-key; retain self-consistent records with invalid snapshot pointers as `None`; collapse duplicate keys, sort results, and never interpret membership as unlock state.

- [ ] **Step 5: Verify GREEN and party regression**

```powershell
cargo test --locked --package gbfr-logs equipment_probe::roster_probe::tests
cargo test --locked --package gbfr-logs equipment_probe::locator::tests
```

Expected: both suites pass.

- [ ] **Step 6: Commit**

```powershell
git add -- src-tauri/src/equipment_probe/roster_probe.rs src-tauri/src/equipment_probe/mod.rs src-tauri/src/equipment_probe/locator.rs
git commit -m "feat: inspect candidate character records"
```

---

### Task 3: Emit sanitized five-second roster evidence

**Files:**
- Modify/Test: `src-tauri/src/equipment_probe/roster_probe.rs`
- Modify: `src-tauri/src/equipment_probe/mod.rs`
- Modify/Test: `src/securityConfiguration.test.ts`

**Interfaces:**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CandidateSnapshotStatus {
    Stable { source_count: usize, digest: String },
    Unavailable,
    Unstable,
    Invalid,
}

pub(crate) fn classify_candidate_snapshot(
    character_key: u32,
    first: Option<&[u8]>,
    second: Option<&[u8]>,
) -> CandidateSnapshotStatus;
```

- [ ] **Step 1: Write RED classification tests**

```rust
#[test]
fn classifies_stable_unavailable_unstable_and_invalid_snapshots() {
    let valid = equipment_fixture(0xE705_3919);
    let mut changed = valid.clone();
    changed[4..8].copy_from_slice(&16_u32.to_le_bytes());
    let short = &valid[..valid.len() - 1];

    assert!(matches!(classify_candidate_snapshot(
        0xE705_3919, Some(&valid), Some(&valid)
    ), CandidateSnapshotStatus::Stable { .. }));
    assert_eq!(classify_candidate_snapshot(0xE705_3919, None, None), CandidateSnapshotStatus::Unavailable);
    assert_eq!(classify_candidate_snapshot(0xE705_3919, Some(&valid), Some(&changed)), CandidateSnapshotStatus::Unstable);
    assert_eq!(classify_candidate_snapshot(0xE705_3919, Some(short), Some(short)), CandidateSnapshotStatus::Invalid);
}
```

Add this Vitest case:

```ts
test("full-roster validation stays read-only and development-gated", () => {
  const runner = readRepositoryFile("src-tauri/src/equipment_probe/mod.rs");
  const roster = readRepositoryFile("src-tauri/src/equipment_probe/roster_probe.rs");
  const memory = readRepositoryFile("src-tauri/src/equipment_probe/memory.rs");
  expect(runner).toContain('std::env::var("DJEETA_EXTERNAL_READER_PROBE")');
  expect(runner).toContain("cfg!(debug_assertions)");
  expect(memory).toContain("PROCESS_QUERY_INFORMATION | PROCESS_VM_READ");
  for (const forbidden of ["PROCESS_VM_WRITE", "PROCESS_VM_OPERATION", "WriteProcessMemory", "VirtualAllocEx", "CreateRemoteThread"]) {
    expect(memory + runner + roster).not.toContain(forbidden);
  }
});
```

- [ ] **Step 2: Verify RED**

```powershell
cargo test --locked --package gbfr-logs equipment_probe::roster_probe::tests
npm.cmd test -- --run src/securityConfiguration.test.ts
```

Expected: Rust fails on missing classification; security test confirms no write API is introduced.

- [ ] **Step 3: Implement classification**

Return `Unavailable` if either read is absent, `Unstable` if bytes differ, `Invalid` if `decode_snapshot(first, character_key)` fails, otherwise `Stable` with source count and `snapshot_digest_prefix(first)`.

- [ ] **Step 4: Wire a five-second pass into the existing debug runner**

Keep the 250ms party loop. Add `let mut next_roster_inspection = Instant::now();`. At the deadline, inspect `roots.manager_global`, double-read known equipment-character candidates with the existing 50ms delay, classify them, and log only:

```text
ROSTER PROBE CANDIDATE buckets=<n> known=<n> unknown=<n> duplicates=<n> rejected=<n>
ROSTER PROBE CANDIDATE character_key=0x........ status=<stable|unavailable|unstable|invalid> sources=<n-or-0> digest=<16-hex-or-none>
```

Count unknown self-consistent keys without logging their values. Do not log pointers or raw values. Do not call `EquipmentState`, `emit_all`, or `equipment-analysis-update`. Advance the deadline by five seconds.

- [ ] **Step 5: Verify GREEN**

```powershell
cargo test --locked --package gbfr-logs equipment_probe::roster_probe::tests
cargo test --locked --package gbfr-logs equipment_probe::locator::tests
cargo test --locked --package gbfr-logs equipment_probe::compare::tests
npm.cmd test -- --run src/securityConfiguration.test.ts
```

Expected: all focused suites pass.

- [ ] **Step 6: Commit**

```powershell
git add -- src-tauri/src/equipment_probe/roster_probe.rs src-tauri/src/equipment_probe/mod.rs src/securityConfiguration.test.ts
git commit -m "feat: probe candidate character roster"
```

---

### Task 4: Run automated regression verification before the manual gate

**Files:**
- No additional files.

- [ ] **Step 1: Run focused regressions**

```powershell
cargo test --locked --package gbfr-logs equipment_probe::roster_probe::tests
cargo test --locked --package gbfr-logs equipment_probe::locator::tests
cargo test --locked --package gbfr-logs equipment_probe::compare::tests
npm.cmd test -- --run src/securityConfiguration.test.ts
```

Expected: all focused tests pass.

- [ ] **Step 2: Run required frontend verification**

```powershell
npm.cmd run format-check
npm.cmd run lint
npm.cmd run tsc
npm.cmd test -- --run
npm.cmd run build
```

Expected: every command exits 0.

- [ ] **Step 3: Run required Rust verification**

Load the Visual Studio developer environment if needed, then run:

```powershell
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
```

Expected: both commands exit 0; existing warnings may remain.

- [ ] **Step 4: Confirm scope**

```powershell
git diff --check
git status --short
git log --oneline --decorate -5
```

Expected: only intentional committed probe changes; `logs.db` remains untracked; no UI, protocol, or hook changes.

---

### Task 5: Run the three-process manual validation gate

**Files:**
- Modify only with factual evidence: `docs/testing/game-2.0.2-equipment-layout.md`

**Interfaces:**
- Consumes: sanitized `ROSTER PROBE CANDIDATE` logs and the game's visible character/equipment screens.
- Produces: `PASS` authorizing a separate production-reader/UI plan, or `FAIL` stopping at the debug probe.

- [ ] **Step 1: Start the gated debug build**

```powershell
$env:DJEETA_EXTERNAL_READER_PROBE = '1'
npm.cmd run tauri dev
```

Expected: no roster line before the pinned game exists; no roster line at all without the environment variable.

- [ ] **Step 2: Validate the first process in an offline/private session**

Record the game's available-character set; compare it with stable known keys; confirm no locked character, Id transformation, Ferry pet, or online identity appears separately. Compare all 12 equipped sigils. Change and restore one sigil on two characters outside the active party and require digest changes within five seconds.

- [ ] **Step 3: Repeat with two fresh processes**

Fully exit the game between runs and repeat Step 2 twice. ASLR and PID may change; membership and decoded results must not.

- [ ] **Step 4: Apply the hard gate**

Pass only when all three runs have an exact roster match, bounded termination, no duplicate known key/cycle/self-key ambiguity, both non-party changes within five seconds, opt-out disables probing, and existing meter/UI behavior remains unchanged. Otherwise record `FAIL` and do not plan product activation.

- [ ] **Step 5: Record and commit factual evidence**

Add `전체 로스터 후보 검증` to the layout document with executable hash, visible/probe counts per run, stable/unavailable counts, two non-party update results, `PASS`/`FAIL`, and confirmation that no addresses/raw save values were retained.

```powershell
git diff --check
git add -- docs/testing/game-2.0.2-equipment-layout.md
git commit -m "docs: record full-roster probe evidence"
```

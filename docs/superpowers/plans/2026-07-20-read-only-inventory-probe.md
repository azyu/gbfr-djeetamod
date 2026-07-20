# Read-Only Inventory Probe Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a debug-only, manually triggered, non-elevated probe that identifies one stable full-sigil-inventory candidate in Granblue Fantasy: Relink 2.0.2 without writing game memory or exposing inventory contents to React.

**Architecture:** Extend `equipment-core` with a process-independent `0x24` inventory record decoder, extend the existing minimal-rights `RemoteProcess` with readable-private-region enumeration, and add a separate inventory scanner/runner under `src-tauri/src/equipment_probe`. The React equipment page only discovers availability and triggers one scan; candidate details remain in restricted backend logs.

**Tech Stack:** Rust 2021, `windows` 0.52, Tauri 1.5 commands, Tokio `spawn_blocking`, SHA-256, React 18, Mantine 7, Vitest, existing Korean/English JSON resources.

## Global Constraints

- Target only Granblue Fantasy: Relink Endless Ragnarok 2.0.2 on Windows x64.
- Keep `requestedExecutionLevel level="asInvoker"` and NSIS `installMode: "currentUser"`.
- Request only `PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ` for the external reader.
- Do not add game-memory writes, code patches, remote threads, or another DLL injection path.
- Require both a debug build and `DJEETA_INVENTORY_PROBE=1`; every release invocation must fail before process access.
- Pin the game executable to SHA-256 `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F`.
- Scan only `MEM_COMMIT`, `MEM_PRIVATE`, readable, non-guard, non-`PAGE_NOACCESS` regions.
- Use 4MiB chunks, a 60-second deadline, and a 16GiB requested-byte limit.
- Never choose the largest candidate: accept exactly one candidate or return `UNAVAILABLE`/`AMBIGUOUS`.
- Re-read a unique candidate twice, 50ms apart, and require byte equality.
- Do not send sigil records, raw bytes, addresses, player names, or inventory JSON to React.
- Do not change `protocol::Message`, hook setup, encounter parsing, or current equipment connection state.
- Preserve existing untracked `logs.db`; never stage it.

---

### Task 1: Decode one inventory record independently of process memory

**Files:**
- Create: `equipment-core/src/inventory.rs`
- Modify: `equipment-core/src/lib.rs`
- Test: `equipment-core/src/inventory.rs`

**Interfaces:**
- Consumes: two owned `HashSet<u32>` catalogs constructed by the backend.
- Produces: `InventoryCatalog::new`, `InventorySigilRecord`, `InventoryDecodeError`, `INVENTORY_RECORD_BYTES`, and `decode_inventory_record` for Task 3.

- [ ] **Step 1: Write failing decoder tests**

Add `mod inventory; pub use inventory::*;` to `equipment-core/src/lib.rs`, create `equipment-core/src/inventory.rs`, and add these tests before defining the referenced types:

```rust
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{decode_inventory_record, InventoryCatalog, InventoryDecodeError, INVENTORY_RECORD_BYTES};

    const SIGIL: u32 = 0xEE73_2781;
    const PRIMARY: u32 = 0xDC58_4F60;
    const SECONDARY: u32 = 0x5007_9A1C;
    const CHARACTER: u32 = 0xE705_3919;

    fn catalog() -> InventoryCatalog {
        InventoryCatalog::new(HashSet::from([SIGIL]), HashSet::from([PRIMARY, SECONDARY]))
    }

    fn put(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn occupied_fixture() -> [u8; INVENTORY_RECORD_BYTES] {
        let mut bytes = [0u8; INVENTORY_RECORD_BYTES];
        put(&mut bytes, 0x00, PRIMARY);
        put(&mut bytes, 0x04, 15);
        put(&mut bytes, 0x08, SECONDARY);
        put(&mut bytes, 0x0C, 11);
        put(&mut bytes, 0x10, SIGIL);
        put(&mut bytes, 0x14, CHARACTER);
        put(&mut bytes, 0x18, 15);
        put(&mut bytes, 0x1C, 42);
        put(&mut bytes, 0x20, 1);
        bytes
    }

    #[test]
    fn decodes_every_verified_inventory_field() {
        let record = decode_inventory_record(&occupied_fixture(), &catalog()).unwrap();
        assert_eq!(record.primary_trait_id, PRIMARY);
        assert_eq!(record.primary_trait_level, 15);
        assert_eq!(record.secondary_trait_id, SECONDARY);
        assert_eq!(record.secondary_trait_level, 11);
        assert_eq!(record.sigil_id, SIGIL);
        assert_eq!(record.equipped_character_key, CHARACTER);
        assert_eq!(record.sigil_level, 15);
        assert_eq!(record.acquisition_index, 42);
        assert_eq!(record.state, 1);
        assert!(record.is_occupied());
    }

    #[test]
    fn accepts_an_exact_empty_record() {
        let bytes = [0u8; INVENTORY_RECORD_BYTES];
        assert!(!decode_inventory_record(&bytes, &catalog()).unwrap().is_occupied());
    }

    #[test]
    fn rejects_partial_unknown_out_of_range_and_contradictory_records() {
        assert!(matches!(
            decode_inventory_record(&occupied_fixture()[..INVENTORY_RECORD_BYTES - 1], &catalog()),
            Err(InventoryDecodeError::TooShort { .. })
        ));

        let mut unknown = occupied_fixture();
        put(&mut unknown, 0x10, 0x1234_5678);
        assert!(matches!(
            decode_inventory_record(&unknown, &catalog()),
            Err(InventoryDecodeError::UnknownSigil(0x1234_5678))
        ));

        let mut high_level = occupied_fixture();
        put(&mut high_level, 0x04, 31);
        assert!(matches!(
            decode_inventory_record(&high_level, &catalog()),
            Err(InventoryDecodeError::InvalidLevel { .. })
        ));

        let mut contradictory = [0u8; INVENTORY_RECORD_BYTES];
        put(&mut contradictory, 0x04, 1);
        assert_eq!(
            decode_inventory_record(&contradictory, &catalog()),
            Err(InventoryDecodeError::ContradictoryEmpty)
        );
    }
}
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```powershell
cargo test --locked -p equipment-core inventory::tests -- --nocapture
```

Expected: compilation fails because `InventoryCatalog`, `InventoryDecodeError`, and `decode_inventory_record` are not defined.

- [ ] **Step 3: Implement the minimal decoder**

Implement these exact public types and validation rules in `equipment-core/src/inventory.rs`:

```rust
use std::collections::HashSet;

use thiserror::Error;

use crate::EMPTY_HASH;

pub const INVENTORY_RECORD_BYTES: usize = 0x24;
const MAX_INVENTORY_LEVEL: u32 = 30;

#[derive(Debug, Clone)]
pub struct InventoryCatalog {
    sigil_ids: HashSet<u32>,
    trait_ids: HashSet<u32>,
}

impl InventoryCatalog {
    pub fn new(sigil_ids: HashSet<u32>, trait_ids: HashSet<u32>) -> Self {
        Self { sigil_ids, trait_ids }
    }

    fn knows_sigil(&self, value: u32) -> bool {
        is_empty_id(value) || self.sigil_ids.contains(&value)
    }

    fn knows_trait(&self, value: u32) -> bool {
        is_empty_id(value) || self.trait_ids.contains(&value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventorySigilRecord {
    pub primary_trait_id: u32,
    pub primary_trait_level: u32,
    pub secondary_trait_id: u32,
    pub secondary_trait_level: u32,
    pub sigil_id: u32,
    pub equipped_character_key: u32,
    pub sigil_level: u32,
    pub acquisition_index: u32,
    pub state: u32,
}

impl InventorySigilRecord {
    pub fn is_occupied(&self) -> bool {
        !is_empty_id(self.sigil_id)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum InventoryDecodeError {
    #[error("inventory record is {actual:#x} bytes; expected {required:#x}")]
    TooShort { actual: usize, required: usize },
    #[error("unknown sigil ID {0:#010x}")]
    UnknownSigil(u32),
    #[error("unknown trait ID {0:#010x}")]
    UnknownTrait(u32),
    #[error("invalid {field} level {level}")]
    InvalidLevel { field: &'static str, level: u32 },
    #[error("empty sigil record contains trait or level data")]
    ContradictoryEmpty,
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("validated field"))
}

fn is_empty_id(value: u32) -> bool {
    value == 0 || value == EMPTY_HASH
}

pub fn decode_inventory_record(
    bytes: &[u8],
    catalog: &InventoryCatalog,
) -> Result<InventorySigilRecord, InventoryDecodeError> {
    if bytes.len() < INVENTORY_RECORD_BYTES {
        return Err(InventoryDecodeError::TooShort {
            actual: bytes.len(),
            required: INVENTORY_RECORD_BYTES,
        });
    }
    let record = InventorySigilRecord {
        primary_trait_id: read_u32(bytes, 0x00),
        primary_trait_level: read_u32(bytes, 0x04),
        secondary_trait_id: read_u32(bytes, 0x08),
        secondary_trait_level: read_u32(bytes, 0x0C),
        sigil_id: read_u32(bytes, 0x10),
        equipped_character_key: read_u32(bytes, 0x14),
        sigil_level: read_u32(bytes, 0x18),
        acquisition_index: read_u32(bytes, 0x1C),
        state: read_u32(bytes, 0x20),
    };
    if !catalog.knows_sigil(record.sigil_id) {
        return Err(InventoryDecodeError::UnknownSigil(record.sigil_id));
    }
    for trait_id in [record.primary_trait_id, record.secondary_trait_id] {
        if !catalog.knows_trait(trait_id) {
            return Err(InventoryDecodeError::UnknownTrait(trait_id));
        }
    }
    for (field, level) in [
        ("primary trait", record.primary_trait_level),
        ("secondary trait", record.secondary_trait_level),
        ("sigil", record.sigil_level),
    ] {
        if level > MAX_INVENTORY_LEVEL {
            return Err(InventoryDecodeError::InvalidLevel { field, level });
        }
    }
    if !record.is_occupied()
        && (!is_empty_id(record.primary_trait_id)
            || !is_empty_id(record.secondary_trait_id)
            || record.primary_trait_level != 0
            || record.secondary_trait_level != 0
            || record.sigil_level != 0)
    {
        return Err(InventoryDecodeError::ContradictoryEmpty);
    }
    Ok(record)
}
```

- [ ] **Step 4: Run focused and workspace Rust tests**

Run:

```powershell
cargo test --locked -p equipment-core inventory::tests -- --nocapture
cargo test --workspace --all-targets --locked
```

Expected: all decoder tests and all existing workspace tests pass.

- [ ] **Step 5: Commit Task 1**

```powershell
git add -- equipment-core/src/inventory.rs equipment-core/src/lib.rs
git commit -m "feat: decode inventory sigil records"
```

---

### Task 2: Enumerate readable private memory with the existing minimal-rights handle

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/equipment_probe/memory.rs`
- Modify: `src/securityConfiguration.test.ts`
- Test: `src-tauri/src/equipment_probe/memory.rs`

**Interfaces:**
- Consumes: existing `RemoteProcess` and `OwnedHandle`.
- Produces: `MemoryRegion`, `is_readable_private_region`, and `RemoteProcess::readable_private_regions` for Task 3.

- [ ] **Step 1: Write failing region-filter tests**

Add the following to the existing `memory.rs` test module, using raw flag values so the decision logic is testable on every platform:

```rust
use super::{is_readable_private_region, MemoryRegion};

#[test]
fn includes_only_committed_private_readable_regions() {
    let committed = 0x1000;
    let private = 0x20000;
    let readwrite = 0x04;
    assert!(is_readable_private_region(committed, private, readwrite));
    assert!(!is_readable_private_region(0x10000, private, readwrite));
    assert!(!is_readable_private_region(committed, 0x40000, readwrite));
    assert!(!is_readable_private_region(committed, private, 0x01));
    assert!(!is_readable_private_region(committed, private, readwrite | 0x100));
}

#[test]
fn memory_region_rejects_overflowing_end_addresses() {
    assert_eq!(
        MemoryRegion { base_address: 0x1000, size: 0x2000 }.end(),
        Some(0x3000)
    );
    assert_eq!(
        MemoryRegion { base_address: usize::MAX, size: 2 }.end(),
        None
    );
}
```

Extend `src/securityConfiguration.test.ts` so its forbidden list also checks `PROCESS_CREATE_PROCESS` and asserts that `VirtualQueryEx` is the only newly expected process-memory API.

- [ ] **Step 2: Run tests and verify RED**

```powershell
cargo test --locked -p gbfr-logs equipment_probe::memory::tests -- --nocapture
npm test -- --run src/securityConfiguration.test.ts
```

Expected: Rust compilation fails because the region APIs do not exist; the security test fails until the source includes `VirtualQueryEx` while preserving the forbidden API list.

- [ ] **Step 3: Add the Windows feature and pure region filter**

Add `Win32_System_Memory` to the existing `windows` feature list in `src-tauri/Cargo.toml`. Add these cross-platform definitions to `memory.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MemoryRegion {
    pub base_address: usize,
    pub size: usize,
}

impl MemoryRegion {
    pub(crate) fn end(self) -> Option<usize> {
        self.base_address.checked_add(self.size)
    }
}

pub(crate) fn is_readable_private_region(state: u32, kind: u32, protect: u32) -> bool {
    const MEM_COMMIT_VALUE: u32 = 0x1000;
    const MEM_PRIVATE_VALUE: u32 = 0x20000;
    const PAGE_NOACCESS_VALUE: u32 = 0x01;
    const PAGE_GUARD_VALUE: u32 = 0x100;
    const READABLE: [u32; 6] = [0x02, 0x04, 0x08, 0x20, 0x40, 0x80];
    state == MEM_COMMIT_VALUE
        && kind == MEM_PRIVATE_VALUE
        && protect & PAGE_GUARD_VALUE == 0
        && protect & 0xFF != PAGE_NOACCESS_VALUE
        && READABLE.contains(&(protect & 0xFF))
}
```

- [ ] **Step 4: Implement `VirtualQueryEx` enumeration**

Import `MEMORY_BASIC_INFORMATION` and `VirtualQueryEx` from `windows::Win32::System::Memory` under `#[cfg(windows)]`, then add:

```rust
#[cfg(windows)]
impl RemoteProcess {
    pub(crate) fn readable_private_regions(&self) -> Result<Vec<MemoryRegion>, MemoryReadError> {
        let mut regions = Vec::new();
        let mut address = 0usize;
        while address < 0x0000_7FFF_FFFF_FFFF {
            let mut info = windows::Win32::System::Memory::MEMORY_BASIC_INFORMATION::default();
            let queried = unsafe {
                windows::Win32::System::Memory::VirtualQueryEx(
                    self.handle.0,
                    Some(address as *const std::ffi::c_void),
                    &mut info,
                    std::mem::size_of_val(&info),
                )
            };
            if queried == 0 || info.RegionSize == 0 {
                break;
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
}
```

Keep `RemoteProcess::find` unchanged so the same minimal access mask opens the handle.

- [ ] **Step 5: Run focused tests and security checks**

```powershell
cargo test --locked -p gbfr-logs equipment_probe::memory::tests -- --nocapture
npm test -- --run src/securityConfiguration.test.ts
```

Expected: both commands pass; the source still contains none of the forbidden write/injection APIs.

- [ ] **Step 6: Commit Task 2**

```powershell
git add -- src-tauri/Cargo.toml src-tauri/src/equipment_probe/memory.rs src/securityConfiguration.test.ts Cargo.lock
git commit -m "feat: enumerate readable game memory"
```

---

### Task 3: Scan chunks and return only one strict inventory candidate

**Files:**
- Create: `src-tauri/src/equipment_probe/inventory.rs`
- Modify: `src-tauri/src/equipment_probe/mod.rs`
- Test: `src-tauri/src/equipment_probe/inventory.rs`

**Interfaces:**
- Consumes: `MemoryReader`, `MemoryRegion`, `InventoryCatalog`, and `decode_inventory_record`.
- Produces: `InventoryCandidate`, `InventoryScanOutcome`, `ScanMetrics`, `ScanLimits`, `InventoryProbeError`, `load_inventory_catalog`, `scan_inventory`, and `read_candidate` for Task 4.

- [ ] **Step 1: Write failing scanner tests with fake memory**

Create an address-backed `FakeMemory` like `locator.rs` and add tests for these exact cases:

```rust
#[test]
fn excludes_a_twelve_record_equipment_snapshot() {
    let memory = inventory_fixture(12, 12);
    assert_eq!(scan_fixture(memory), InventoryScanOutcome::Unavailable);
}

#[test]
fn accepts_one_thirteen_record_run_with_six_occupied_records() {
    let memory = inventory_fixture(13, 6);
    let InventoryScanOutcome::Unique(candidate) = scan_fixture(memory) else { panic!() };
    assert_eq!(candidate.record_count, 13);
    assert_eq!(candidate.occupied_count, 6);
}

#[test]
fn reports_two_qualified_runs_as_ambiguous() {
    let memory = two_inventory_fixture();
    assert!(matches!(scan_fixture(memory), InventoryScanOutcome::Ambiguous { count: 2 }));
}

#[test]
fn finds_a_record_that_crosses_a_chunk_boundary_without_duplicate_candidates() {
    let memory = boundary_fixture(4 * 1024 * 1024 - 8);
    let InventoryScanOutcome::Unique(candidate) = scan_fixture(memory) else { panic!() };
    assert_eq!(candidate.occupied_count, 6);
}

#[test]
fn rejects_changed_second_read_and_enforces_limits() {
    assert!(matches!(verify_changed_fixture(), Err(InventoryProbeError::Unstable)));
    assert!(ScanLimits::new(16, Duration::from_secs(60)).exceeded(17, Duration::ZERO));
    assert!(ScanLimits::new(16, Duration::from_secs(60)).exceeded(1, Duration::from_secs(61)));
}
```

The fixtures must use catalog IDs already present in `lang/en/sigils.json` and `lang/en/traits.json`, use exact `0x24` strides, and assert that raw record values never appear in formatted summaries.

- [ ] **Step 2: Run the focused tests and verify RED**

```powershell
cargo test --locked -p gbfr-logs equipment_probe::inventory::tests -- --nocapture
```

Expected: compilation fails because the inventory module and scanner interfaces do not exist.

- [ ] **Step 3: Implement catalog loading and scan types**

Parse only the JSON object keys from the bundled English resources:

```rust
fn parse_catalog_keys(source: &str) -> Result<std::collections::HashSet<u32>, InventoryProbeError> {
    let rows: std::collections::HashMap<String, serde_json::Value> = serde_json::from_str(source)?;
    rows.into_keys()
        .map(|key| u32::from_str_radix(&key, 16).map_err(|_| InventoryProbeError::InvalidCatalogKey(key)))
        .collect()
}

pub(crate) fn load_inventory_catalog() -> Result<InventoryCatalog, InventoryProbeError> {
    Ok(InventoryCatalog::new(
        parse_catalog_keys(include_str!("../../lang/en/sigils.json"))?,
        parse_catalog_keys(include_str!("../../lang/en/traits.json"))?,
    ))
}
```

Define the result types without storing decoded records:

```rust
pub(crate) const INVENTORY_SCAN_CHUNK_BYTES: usize = 4 * 1024 * 1024;
const MIN_RECORDS: usize = 13;
const MIN_OCCUPIED: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InventoryCandidate {
    pub base_address: usize,
    pub record_count: usize,
    pub occupied_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InventoryScanOutcome {
    Unique(InventoryCandidate),
    Unavailable,
    Ambiguous { count: usize },
    LimitExceeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScanMetrics {
    pub region_count: usize,
    pub requested_bytes: u64,
    pub elapsed: Duration,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum InventoryProbeError {
    #[error(transparent)]
    Memory(#[from] super::memory::MemoryReadError),
    #[error(transparent)]
    Decode(#[from] equipment_core::InventoryDecodeError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("invalid inventory catalog key {0}")]
    InvalidCatalogKey(String),
    #[error("inventory address range overflow")]
    AddressOverflow,
    #[error("inventory changed between stable reads")]
    Unstable,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ScanLimits {
    pub max_bytes: u64,
    pub max_duration: Duration,
}

impl ScanLimits {
    pub(crate) fn new(max_bytes: u64, max_duration: Duration) -> Self {
        Self { max_bytes, max_duration }
    }

    pub(crate) fn exceeded(self, bytes: u64, elapsed: Duration) -> bool {
        bytes > self.max_bytes || elapsed > self.max_duration
    }
}
```

- [ ] **Step 4: Implement chunk scanning, overlap de-duplication, and strict selection**

For every region, read `min(4MiB + 0x23, remaining)` bytes. For offsets `(0..=len-0x24).step_by(4)`, decode records and retain `(absolute_address, occupied)` for successful decodes. Sort and deduplicate addresses after all chunks, coalesce entries whose addresses differ by exactly `0x24`, trim empty entries from both ends, then retain runs with `record_count >= 13` and `occupied_count >= 6`.

Expose this signature:

```rust
pub(crate) fn scan_inventory<R: MemoryReader>(
    reader: &R,
    regions: &[MemoryRegion],
    catalog: &InventoryCatalog,
    limits: ScanLimits,
) -> Result<(InventoryScanOutcome, ScanMetrics), InventoryProbeError>;
```

`ScanMetrics` contains only `region_count`, `requested_bytes`, and `elapsed`. Before every read, calculate the next requested-byte total and return `LimitExceeded` without reading when it exceeds the limit. Check elapsed time before each chunk. Do not retain record field values after candidate counting.

- [ ] **Step 5: Implement stable double-read verification**

Expose:

```rust
pub(crate) fn read_candidate<R: MemoryReader>(
    reader: &R,
    candidate: InventoryCandidate,
    catalog: &InventoryCatalog,
) -> Result<Vec<u8>, InventoryProbeError>;
```

The function uses `record_count.checked_mul(INVENTORY_RECORD_BYTES)`, reads exactly that range, and re-decodes every record. The async runner in Task 4 calls it once, waits 50ms, calls it again, and requires byte equality. Add `snapshot_digest_prefix(bytes: &[u8]) -> String` that returns only the first 16 lowercase SHA-256 hex characters.

- [ ] **Step 6: Run scanner and workspace tests**

```powershell
cargo test --locked -p gbfr-logs equipment_probe::inventory::tests -- --nocapture
cargo test --workspace --all-targets --locked
```

Expected: all new scanner tests and existing Rust tests pass.

- [ ] **Step 7: Commit Task 3**

```powershell
git add -- src-tauri/src/equipment_probe/inventory.rs src-tauri/src/equipment_probe/mod.rs
git commit -m "feat: locate inventory candidates read only"
```

---

### Task 4: Add guarded Tauri commands and restricted logging

**Files:**
- Modify: `src-tauri/src/equipment_probe/inventory.rs`
- Modify: `src-tauri/src/equipment_probe/mod.rs`
- Modify: `src-tauri/src/main.rs`
- Test: `src-tauri/src/equipment_probe/inventory.rs`

**Interfaces:**
- Consumes: Task 3 scanner and existing `RemoteProcess::find`, `executable_sha256`, and `readable_private_regions`.
- Produces: Tauri commands `inventory_probe_available` and `capture_inventory_probe`; Task 5 invokes only those names.

- [ ] **Step 1: Write failing gate, concurrency, and outcome-code tests**

Add tests for the pure command helpers:

```rust
#[test]
fn inventory_probe_requires_debug_build_and_exact_opt_in() {
    assert!(inventory_probe_enabled(true, Some("1")));
    assert!(!inventory_probe_enabled(true, None));
    assert!(!inventory_probe_enabled(true, Some("true")));
    assert!(!inventory_probe_enabled(true, Some("01")));
    assert!(!inventory_probe_enabled(false, Some("1")));
}

#[test]
fn run_flag_rejects_overlap_and_recovers_after_drop() {
    let state = InventoryProbeState::default();
    let first = state.try_begin().unwrap();
    assert_eq!(state.try_begin().unwrap_err(), InventoryProbeCode::AlreadyRunning);
    drop(first);
    assert!(state.try_begin().is_ok());
}

#[test]
fn public_error_codes_do_not_contain_addresses_or_record_data() {
    for code in InventoryProbeCode::ALL {
        let value = code.as_str();
        assert!(!value.contains("0x"));
        assert!(!value.contains("sigil"));
    }
}
```

- [ ] **Step 2: Run the focused tests and verify RED**

```powershell
cargo test --locked -p gbfr-logs equipment_probe::inventory::tests -- --nocapture
```

Expected: compilation fails because command state, guard, and public error codes do not exist.

- [ ] **Step 3: Implement the exact opt-in and RAII run guard**

Define `InventoryProbeState` with `Arc<AtomicBool>` so `spawn_blocking` can own the flag. `try_begin` uses `compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)`. `InventoryProbeRunGuard::drop` stores `false` with `Ordering::Release`.

Use these stable public codes:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InventoryProbeCode {
    Disabled,
    AlreadyRunning,
    GameNotRunning,
    UnsupportedGame,
    Unavailable,
    Ambiguous,
    Unstable,
    LimitExceeded,
    Internal,
}

impl InventoryProbeCode {
    pub(crate) const ALL: [Self; 9] = [
        Self::Disabled, Self::AlreadyRunning, Self::GameNotRunning,
        Self::UnsupportedGame, Self::Unavailable, Self::Ambiguous,
        Self::Unstable, Self::LimitExceeded, Self::Internal,
    ];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "DISABLED",
            Self::AlreadyRunning => "ALREADY_RUNNING",
            Self::GameNotRunning => "GAME_NOT_RUNNING",
            Self::UnsupportedGame => "UNSUPPORTED_GAME",
            Self::Unavailable => "UNAVAILABLE",
            Self::Ambiguous => "AMBIGUOUS",
            Self::Unstable => "UNSTABLE",
            Self::LimitExceeded => "LIMIT_EXCEEDED",
            Self::Internal => "INTERNAL",
        }
    }
}
```

- [ ] **Step 4: Implement one-shot capture and restricted logs**

Implement a synchronous `capture_once()` called only inside `spawn_blocking`:

1. Recheck the debug/env gate.
2. Find `granblue_fantasy_relink.exe`.
3. Hash the executable and require the pinned hash.
4. Load catalogs and enumerate readable private regions.
5. Run `scan_inventory` with 16GiB/60s limits.
6. For one candidate, read once, sleep 50ms, read again, and require equality.
7. Log only PID/hash/rights, metrics, candidate address/counts/digest, and final status.

Return only `Result<(), InventoryProbeCode>`. Convert internal errors to warning logs before returning `Internal`; never serialize internal error text to React.

- [ ] **Step 5: Register state and Tauri commands**

Add to `main.rs`:

```rust
.manage(equipment_probe::inventory::InventoryProbeState::default())
```

and append these command names to `tauri::generate_handler!` without changing existing order:

```rust
equipment_probe::inventory::inventory_probe_available,
equipment_probe::inventory::capture_inventory_probe,
```

The availability command returns `bool`. The async capture command returns `Result<(), String>` and maps `InventoryProbeCode` through `as_str()`.

- [ ] **Step 6: Run command and workspace tests**

```powershell
cargo test --locked -p gbfr-logs equipment_probe::inventory::tests -- --nocapture
cargo test --workspace --all-targets --locked
```

Expected: all tests pass and no protocol or hook files change.

- [ ] **Step 7: Commit Task 4**

```powershell
git add -- src-tauri/src/equipment_probe/inventory.rs src-tauri/src/equipment_probe/mod.rs src-tauri/src/main.rs
git commit -m "feat: trigger inventory probe manually"
```

---

### Task 5: Add the debug-only capture control to Equipment Analysis

**Files:**
- Modify: `src/pages/EquipmentAnalysis.tsx`
- Modify: `src/pages/EquipmentAnalysis.test.tsx`
- Modify: `src-tauri/lang/ko/ui.json`
- Modify: `src-tauri/lang/en/ui.json`
- Test: `src/pages/EquipmentAnalysis.test.tsx`

**Interfaces:**
- Consumes: Tauri commands `inventory_probe_available` and `capture_inventory_probe` from Task 4.
- Produces: no shared store or inventory data; only local availability/running/status state inside `EquipmentAnalysis`.

- [ ] **Step 1: Rewrite the Tauri mock to dispatch by command**

Replace the fixed invoke mock with:

```ts
const mocks = vi.hoisted(() => ({
  response: null as unknown,
  probeAvailable: false,
  captureError: null as string | null,
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
}));

vi.mock("@tauri-apps/api", () => ({
  invoke: vi.fn(async (command: string) => {
    if (command === "fetch_equipment_analysis") return mocks.response;
    if (command === "inventory_probe_available") return mocks.probeAvailable;
    if (command === "capture_inventory_probe") {
      if (mocks.captureError) throw mocks.captureError;
      return undefined;
    }
    throw new Error(`unexpected command: ${command}`);
  }),
}));
```

Reset `probeAvailable` and `captureError` in `beforeEach`.

- [ ] **Step 2: Add failing visibility, running, success, and error tests**

Import `fireEvent` from Testing Library; do not add another test dependency.

```ts
it("shows the inventory probe only when the backend enables it", async () => {
  const hidden = renderPage();
  expect(await screen.findByText("진 특성 상한 분석")).toBeTruthy();
  expect(hidden.queryByRole("button", { name: "보유 진 캡처" })).toBeNull();
  hidden.unmount();

  mocks.probeAvailable = true;
  renderPage();
  expect(await screen.findByRole("button", { name: "보유 진 캡처" })).toBeTruthy();
});

it("disables capture while running and reports completion without inventory data", async () => {
  mocks.probeAvailable = true;
  renderPage();
  const button = await screen.findByRole("button", { name: "보유 진 캡처" });
  fireEvent.click(button);
  expect((button as HTMLButtonElement).disabled).toBe(true);
  expect(await screen.findByText("캡처 완료 — 개발 로그 확인")).toBeTruthy();
  expect(screen.queryByText(/0x[0-9a-f]+/i)).toBeNull();
});

it("maps backend probe codes to limited Korean errors", async () => {
  mocks.probeAvailable = true;
  mocks.captureError = "AMBIGUOUS";
  renderPage();
  fireEvent.click(await screen.findByRole("button", { name: "보유 진 캡처" }));
  expect(await screen.findByText("보유 진 후보가 여러 개입니다.")).toBeTruthy();
});
```

- [ ] **Step 3: Run the focused UI test and verify RED**

```powershell
npm test -- --run src/pages/EquipmentAnalysis.test.tsx
```

Expected: the new tests fail because no capture control exists.

- [ ] **Step 4: Implement local probe UI state**

Import Mantine `Button` and `Alert`, add `useState`, and on mount invoke both `fetch_equipment_analysis` and `inventory_probe_available`. Keep probe state local:

```ts
const [probeAvailable, setProbeAvailable] = useState(false);
const [probeRunning, setProbeRunning] = useState(false);
const [probeMessage, setProbeMessage] = useState<string | null>(null);

const captureInventory = async () => {
  setProbeRunning(true);
  setProbeMessage(null);
  try {
    await invoke("capture_inventory_probe");
    setProbeMessage(t("ui.equipment-analysis.inventory-probe.complete"));
  } catch (error) {
    const code = typeof error === "string" ? error : "INTERNAL";
    setProbeMessage(t(`ui.equipment-analysis.inventory-probe.error.${code}`));
  } finally {
    setProbeRunning(false);
  }
};
```

Render the explanatory text, button, and status only when `probeAvailable` is true. Do not add a Zustand store, event listener, record type, or inventory result state.

- [ ] **Step 5: Add exact Korean and English translations**

Under `ui.equipment-analysis.inventory-probe`, add `button`, `hint`, `running`, `complete`, and errors for all nine backend codes. Required Korean values include:

```json
{
  "button": "보유 진 캡처",
  "hint": "게임에서 진 인벤토리 화면을 연 뒤 캡처하세요. 결과는 개발 로그에만 기록됩니다.",
  "running": "보유 진 후보를 확인하는 중입니다.",
  "complete": "캡처 완료 — 개발 로그 확인",
  "error": {
    "AMBIGUOUS": "보유 진 후보가 여러 개입니다.",
    "UNAVAILABLE": "보유 진 후보를 찾지 못했습니다.",
    "UNSTABLE": "인벤토리가 변경 중이어서 캡처하지 않았습니다."
  }
}
```

Supply explicit Korean and English strings for `DISABLED`, `ALREADY_RUNNING`, `GAME_NOT_RUNNING`, `UNSUPPORTED_GAME`, `LIMIT_EXCEEDED`, and `INTERNAL`; do not fall back to raw backend codes.

- [ ] **Step 6: Run UI, localization, type, and lint checks**

```powershell
npm test -- --run src/pages/EquipmentAnalysis.test.tsx src/pages/EquipmentAnalysis.localization.test.ts
npm run tsc
npm run lint
```

Expected: all commands pass with no inventory data added to frontend types or stores.

- [ ] **Step 7: Commit Task 5**

```powershell
git add -- src/pages/EquipmentAnalysis.tsx src/pages/EquipmentAnalysis.test.tsx src-tauri/lang/ko/ui.json src-tauri/lang/en/ui.json
git commit -m "feat: expose inventory probe control in debug"
```

---

### Task 6: Lock security invariants, document manual verification, and run the full gate

**Files:**
- Modify: `src/securityConfiguration.test.ts`
- Create: `docs/testing/game-2.0.2-inventory-probe.md`
- Modify only if packaging produces new release artifacts: `README.md`
- Modify only if packaging produces new release artifacts: `docs/testing/game-2.0.2-smoke-test.md`

**Interfaces:**
- Consumes: all prior task deliverables.
- Produces: an auditable security regression test, a manual validation checklist, and complete verification evidence.

- [ ] **Step 1: Add the final source-level security assertions**

Extend the existing test to read both `memory.rs` and `inventory.rs` and assert:

```ts
test("inventory probe stays read-only and release-gated", () => {
  const memory = readRepositoryFile("src-tauri/src/equipment_probe/memory.rs");
  const inventory = readRepositoryFile("src-tauri/src/equipment_probe/inventory.rs");
  expect(memory).toContain("PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ");
  expect(memory).toContain("VirtualQueryEx");
  expect(inventory).toContain('std::env::var("DJEETA_INVENTORY_PROBE")');
  expect(inventory).toContain("cfg!(debug_assertions)");
  for (const forbidden of [
    "PROCESS_VM_WRITE", "PROCESS_VM_OPERATION", "PROCESS_CREATE_THREAD",
    "WriteProcessMemory", "VirtualAllocEx", "CreateRemoteThread",
  ]) {
    expect(memory + inventory).not.toContain(forbidden);
  }
});
```

- [ ] **Step 2: Run the security test and repair only probe-scope failures**

```powershell
npm test -- --run src/securityConfiguration.test.ts
```

Expected: PASS. If it fails, change only the new probe code or assertion necessary to preserve the approved security boundary.

- [ ] **Step 3: Create the manual test record**

Create `docs/testing/game-2.0.2-inventory-probe.md` with:

- pinned EXE hash and `asInvoker` requirement;
- unchecked rows for baseline count, sort/filter digest stability, actual inventory mutation, three complete process restarts, existing meter regression, existing equipped-sigil regression, disabled debug run, and release rejection;
- columns for PID, candidate record count, occupied count, digest, UI count, and result;
- an explicit statement that unchecked rows do not establish full inventory compatibility;
- no addresses, raw bytes, player names, or full sigil lists.

- [ ] **Step 4: Run formatting and all automated tests**

```powershell
npm ci
npm run format
npm run format-check
npm run lint
npm run tsc
npm test -- --run
npm run build
cargo fmt --all
cargo fmt --all -- --check
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
```

Expected: every command exits 0. Review `git diff` after `npm run format` and revert no unrelated user changes.

- [ ] **Step 5: Build and verify the NSIS package**

Load the Visual Studio developer environment if MSVC is not already available, then run:

```powershell
npm run tauri build -- --bundles nsis
```

Expected: exit 0 and an NSIS installer under `target/release/bundle/nsis`.

Calculate SHA-256 for `target/release/hook.dll` and `src-tauri/hook.dll`; require exact equality. If the installer is a new release candidate, update its hash and the hook hash in `README.md` and `docs/testing/game-2.0.2-smoke-test.md`, then rerun formatting/tests affected by those files. Otherwise leave published hashes unchanged.

- [ ] **Step 6: Inspect the final diff and commit Task 6**

```powershell
git diff --check
git status --short
git diff --stat master...HEAD
git add -- src/securityConfiguration.test.ts docs/testing/game-2.0.2-inventory-probe.md
git commit -m "test: guard read-only inventory probing"
```

If release hashes were intentionally updated, include only `README.md` and `docs/testing/game-2.0.2-smoke-test.md` in this commit in addition to the two listed files. Never add `logs.db`, build outputs, captured memory, or local logs.

- [ ] **Step 7: Stop before compatibility claims**

Report the automated verification results and leave every manual game row unchecked until it is actually performed. Do not state that full inventory support works on game 2.0.2 until every row in `docs/testing/game-2.0.2-inventory-probe.md` is completed with evidence.

# Inventory Scanner Performance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace exhaustive record decoding with a complete known-sigil-ID search and candidate-only validation so the pinned game inventory probe finishes within 10 seconds.

**Architecture:** Expose the catalog's known non-empty sigil IDs, build one Aho-Corasick matcher, and scan every readable private memory region for occupied-record anchors. Fully expand and validate every distinct anchor phase through cached 64 KiB record windows, then preserve the existing zero/one/many candidate classification and stable reread.

**Tech Stack:** Rust 2021, `aho-corasick`, existing `equipment-core`, Win32 read-only `ReadProcessMemory`, Tauri debug runtime, Vitest security assertions.

## Global Constraints

- Keep the application manifest at `requestedExecutionLevel level="asInvoker"`.
- Keep process access at `PROCESS_QUERY_INFORMATION | PROCESS_VM_READ`.
- Do not add process write, memory operation, thread creation, remote allocation, code patching, or injection rights.
- Search every enumerated committed, private, readable, non-guard region; do not stop after the first or largest candidate.
- Preserve `UNAVAILABLE`, `AMBIGUOUS`, `UNSTABLE`, `LIMIT_EXCEEDED`, and `INTERNAL` fail-closed outcomes.
- Remove the 16 GiB byte limit and 60-second limit; enforce one 10-second deadline over discovery and candidate validation.
- Never accept partial results after deadline expiry or an unavailable candidate-validation read.
- Keep the existing 13-record minimum, six-occupied minimum, 12-slot equipment exclusion, and second stable read.
- Do not cache process addresses across runs or expose raw inventory contents.
- Do not change hook injection or the Equipment Analysis UI in this stage.
- Do not claim game 2.0.2 full-inventory compatibility until the manual checklist passes.
- Do not modify or stage the existing `logs.db` file.

---

### Task 1: Expose non-empty sigil search IDs

**Files:**
- Modify: `equipment-core/src/inventory.rs:13-31,91-160`

**Interfaces:**
- Consumes: `InventoryCatalog::sigil_ids` and the existing private `is_empty_id(u32) -> bool` rule.
- Produces: `pub fn known_non_empty_sigil_ids(&self) -> impl Iterator<Item = u32> + '_` for matcher construction in Task 2.

- [ ] **Step 1: Write the failing catalog iterator test**

Extend the test module imports and add:

```rust
use crate::EMPTY_HASH;

#[test]
fn exposes_only_known_non_empty_sigil_ids() {
    let catalog = InventoryCatalog::new(
        HashSet::from([SIGIL_ID, 0, EMPTY_HASH]),
        HashSet::new(),
    );

    let mut ids = catalog.known_non_empty_sigil_ids().collect::<Vec<_>>();
    ids.sort_unstable();

    assert_eq!(ids, vec![SIGIL_ID]);
}
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```powershell
cargo test --locked --package equipment-core inventory::tests::exposes_only_known_non_empty_sigil_ids
```

Expected: compilation fails because `known_non_empty_sigil_ids` does not exist.

- [ ] **Step 3: Implement the minimal read-only iterator**

Add inside `impl InventoryCatalog`:

```rust
pub fn known_non_empty_sigil_ids(&self) -> impl Iterator<Item = u32> + '_ {
    self.sigil_ids
        .iter()
        .copied()
        .filter(|value| !is_empty_id(*value))
}
```

Do not make `knows_sigil`, `knows_trait`, or `is_empty_id` public.

- [ ] **Step 4: Run the catalog tests and verify GREEN**

Run:

```powershell
cargo test --locked --package equipment-core inventory::tests
```

Expected: all inventory decoder and iterator tests pass.

- [ ] **Step 5: Format, inspect, and commit**

Run:

```powershell
rustfmt --edition 2021 equipment-core/src/inventory.rs
git diff --check
git diff -- equipment-core/src/inventory.rs
```

Stage only the catalog file and commit:

```powershell
git add -- equipment-core/src/inventory.rs
git commit -m "feat: expose inventory sigil search ids"
```

---

### Task 2: Discover aligned anchors with one multi-pattern scan

**Files:**
- Modify: `src-tauri/Cargo.toml:12-45`
- Modify: `src-tauri/src/equipment_probe/inventory.rs:1-30,237-320,529-842`

**Interfaces:**
- Consumes: `InventoryCatalog::known_non_empty_sigil_ids()` from Task 1, `MemoryRegion`, and 4 MiB remote chunks.
- Produces:
  - `fn build_sigil_matcher(catalog: &InventoryCatalog) -> Result<AhoCorasick, InventoryProbeError>`
  - `fn find_inventory_anchors(bytes: &[u8], chunk_base: usize, region: MemoryRegion, matcher: &AhoCorasick) -> Result<Vec<usize>, InventoryProbeError>`
  - `fn discover_inventory_anchors<R: MemoryReader>(...) -> Result<(Vec<usize>, u64), InventoryProbeError>`
  - `ScanDeadline::new(Duration)` and `ScanDeadline::exceeded(Duration) -> bool`

- [ ] **Step 1: Write failing matcher and boundary tests**

Add focused tests using the existing `catalog`, `put`, `occupied_record`, and `BASE` fixtures:

```rust
#[test]
fn finds_only_aligned_known_sigil_anchors() {
    let matcher = build_sigil_matcher(&catalog()).unwrap();
    let region = MemoryRegion {
        base_address: BASE,
        size: 0x200,
    };
    let mut bytes = vec![0xA5; region.size];
    let record = occupied_record();
    bytes[0x40..0x40 + INVENTORY_RECORD_BYTES].copy_from_slice(&record);
    bytes[0x81 + 0x10..0x81 + 0x14].copy_from_slice(&record[0x10..0x14]);

    assert_eq!(
        find_inventory_anchors(&bytes, BASE, region, &matcher).unwrap(),
        vec![BASE + 0x40]
    );
}

#[test]
fn finds_a_record_whose_sigil_field_starts_at_a_chunk_boundary_once() {
    let run_offset = INVENTORY_SCAN_CHUNK_BYTES - 0x10;
    let memory = boundary_fixture(run_offset);
    let matcher = build_sigil_matcher(&catalog()).unwrap();
    let deadline = ScanDeadline::new(Duration::from_secs(10));
    let started = Instant::now();
    let (anchors, requested_bytes) = discover_inventory_anchors(
        &memory,
        &memory.regions,
        &matcher,
        started,
        deadline,
    )
    .unwrap();

    assert_eq!(
        anchors
            .iter()
            .filter(|address| **address == BASE + run_offset)
            .count(),
        1
    );
    assert!(requested_bytes > 0);
}

#[test]
fn deadline_has_no_byte_limit_and_expires_at_ten_seconds() {
    let deadline = ScanDeadline::new(Duration::from_secs(10));

    assert!(!deadline.exceeded(Duration::from_secs(9)));
    assert!(!deadline.exceeded(Duration::from_secs(10)));
    assert!(deadline.exceeded(Duration::from_secs(10) + Duration::from_nanos(1)));
}
```

Import `Instant` in the test module. The boundary test deliberately places the record start at `4 MiB - 0x10`, so its known sigil field begins exactly at the next chunk and the record itself spans the boundary.

- [ ] **Step 2: Run the focused tests and verify RED**

Run:

```powershell
cargo test --locked --package gbfr-logs equipment_probe::inventory::tests::finds_only_aligned_known_sigil_anchors
```

Expected: compilation fails because the matcher and anchor-discovery helpers do not exist.

- [ ] **Step 3: Declare the direct matcher dependency and error conversion**

Add to `[dependencies]` in `src-tauri/Cargo.toml` without changing existing versions:

```toml
aho-corasick = "1.1"
```

Import the matcher:

```rust
use aho_corasick::AhoCorasick;
```

Add to `InventoryProbeError`:

```rust
#[error(transparent)]
Matcher(#[from] aho_corasick::BuildError),
#[error("inventory scan deadline exceeded")]
DeadlineExceeded,
```

- [ ] **Step 4: Add the duration-only deadline used by discovery**

Add the pattern overlap constant:

```rust
const SIGIL_PATTERN_OVERLAP: usize = std::mem::size_of::<u32>() - 1;
```

Add this type alongside the existing `ScanLimits` temporarily:

```rust
#[derive(Debug, Clone, Copy)]
pub(crate) struct ScanDeadline {
    pub max_duration: Duration,
}

impl ScanDeadline {
    pub(crate) fn new(max_duration: Duration) -> Self {
        Self { max_duration }
    }

    pub(crate) fn exceeded(self, elapsed: Duration) -> bool {
        elapsed > self.max_duration
    }
}
```

Do not change the old production scanner, `INVENTORY_MAX_BYTES`, `INVENTORY_MAX_DURATION`, or `ScanLimits` in this task. Task 3 removes them atomically when it switches production to candidate-only validation, so every intermediate commit continues to compile and pass tests.

- [ ] **Step 5: Add matcher construction and pure buffer discovery**

Implement the helpers with checked address arithmetic:

```rust
fn build_sigil_matcher(
    catalog: &InventoryCatalog,
) -> Result<AhoCorasick, InventoryProbeError> {
    let patterns = catalog
        .known_non_empty_sigil_ids()
        .map(u32::to_le_bytes)
        .collect::<Vec<_>>();
    Ok(AhoCorasick::new(
        patterns.iter().map(|pattern| pattern.as_slice()),
    )?)
}

fn find_inventory_anchors(
    bytes: &[u8],
    chunk_base: usize,
    region: MemoryRegion,
    matcher: &AhoCorasick,
) -> Result<Vec<usize>, InventoryProbeError> {
    const SIGIL_FIELD_OFFSET: usize = 0x10;
    let region_end = region.end().ok_or(InventoryProbeError::AddressOverflow)?;
    let mut anchors = Vec::new();

    for matched in matcher.find_overlapping_iter(bytes) {
        let field_address = chunk_base
            .checked_add(matched.start())
            .ok_or(InventoryProbeError::AddressOverflow)?;
        let Some(record_address) = field_address.checked_sub(SIGIL_FIELD_OFFSET) else {
            continue;
        };
        let Some(record_end) = record_address.checked_add(INVENTORY_RECORD_BYTES) else {
            return Err(InventoryProbeError::AddressOverflow);
        };
        if record_address % 4 == 0
            && record_address >= region.base_address
            && record_end <= region_end
        {
            anchors.push(record_address);
        }
    }
    anchors.sort_unstable();
    anchors.dedup();
    Ok(anchors)
}
```

- [ ] **Step 6: Add complete region discovery with 3-byte overlap**

Implement `discover_inventory_anchors` so it:

1. Checks `deadline.exceeded(started.elapsed())` before every read and returns `InventoryProbeError::DeadlineExceeded`.
2. Reads `min(remaining, INVENTORY_SCAN_CHUNK_BYTES + SIGIL_PATTERN_OVERLAP)` bytes.
3. Adds the requested length to a local `requested_bytes: u64` with checked arithmetic before `read_exact`.
4. Skips a chunk whose read becomes unavailable, matching current scan behavior.
5. Calls `find_inventory_anchors` for successful chunks.
6. Advances by exactly `INVENTORY_SCAN_CHUNK_BYTES`.
7. Checks the deadline again after each buffer search so local matcher work cannot overrun the terminal deadline unnoticed.
8. Sorts and deduplicates anchors across all chunks and regions.
9. Returns `(anchors, requested_bytes)`.

- [ ] **Step 7: Run the focused tests and verify GREEN**

Run:

```powershell
cargo test --locked --package equipment-core inventory::tests
cargo test --locked --package gbfr-logs equipment_probe::inventory::tests::finds_only_aligned_known_sigil_anchors
cargo test --locked --package gbfr-logs equipment_probe::inventory::tests::finds_a_record_whose_sigil_field_starts_at_a_chunk_boundary_once
cargo test --locked --package gbfr-logs equipment_probe::inventory::tests::deadline_has_no_byte_limit_and_expires_at_ten_seconds
```

Expected: all focused tests pass.

- [ ] **Step 8: Format, inspect, and commit**

Run:

```powershell
rustfmt --edition 2021 src-tauri/src/equipment_probe/inventory.rs
cargo check --locked --package gbfr-logs
git diff --check
git diff -- src-tauri/Cargo.toml Cargo.lock src-tauri/src/equipment_probe/inventory.rs
```

Stage only matcher discovery files and the lockfile, then commit:

```powershell
git add -- src-tauri/Cargo.toml Cargo.lock src-tauri/src/equipment_probe/inventory.rs
git commit -m "perf: discover inventory anchors by sigil id"
```

---

### Task 3: Expand and classify every distinct candidate phase

**Files:**
- Modify: `src-tauri/src/equipment_probe/inventory.rs:237-529,529-842`

**Interfaces:**
- Consumes: sorted anchor addresses from Task 2, `MemoryReader`, `MemoryRegion`, `InventoryCatalog`, and `ScanDeadline`.
- Produces:
  - `fn validate_inventory_anchor<R: MemoryReader>(...) -> Result<Option<InventoryCandidate>, InventoryProbeError>`
  - candidate-only `scan_inventory(...) -> Result<(InventoryScanOutcome, ScanMetrics), InventoryProbeError>`
  - metrics containing `anchor_count`, `validated_run_count`, and `validated_record_count`.

- [ ] **Step 1: Write failing candidate-only work tests**

Add a large decoy test and strengthen the duplicate-anchor fixture:

```rust
#[test]
fn validates_records_only_around_discovered_anchors() {
    let mut memory = inventory_fixture(13, 6);
    let mut decoy = vec![0xA5; 16 * 1024 * 1024];
    decoy.extend_from_slice(&memory.bytes[0].1);
    memory.regions[0].size = decoy.len();
    memory.bytes[0].1 = decoy;

    let (outcome, metrics) = scan_inventory(
        &memory,
        &memory.regions,
        &catalog(),
        ScanDeadline::new(Duration::from_secs(10)),
    )
    .unwrap();

    assert!(matches!(outcome, InventoryScanOutcome::Unique(_)));
    assert!(metrics.anchor_count >= 6);
    assert!(metrics.validated_record_count < 1_000);
}

#[test]
fn multiple_anchors_in_one_run_produce_one_candidate() {
    let memory = inventory_fixture(30, 20);
    let (outcome, metrics) = scan_inventory(
        &memory,
        &memory.regions,
        &catalog(),
        ScanDeadline::new(Duration::from_secs(10)),
    )
    .unwrap();

    assert!(matches!(outcome, InventoryScanOutcome::Unique(_)));
    assert_eq!(metrics.validated_run_count, 1);
}
```

Keep the existing tests for 12 records, 13 records with six occupied, two distinct runs, chunk boundaries, unavailable reads, and second-read instability.

- [ ] **Step 2: Run the work-bounding test and verify RED**

Run:

```powershell
cargo test --locked --package gbfr-logs equipment_probe::inventory::tests::validates_records_only_around_discovered_anchors
```

Expected: compilation fails because `validated_record_count` and the candidate-only scanner are not complete.

- [ ] **Step 3: Add a cached 64 KiB record window**

Add:

```rust
const INVENTORY_VALIDATION_WINDOW_BYTES: usize = 64 * 1024;

#[derive(Debug, Default)]
struct ValidationWindow {
    base_address: usize,
    bytes: Vec<u8>,
}
```

Implement a helper that maps an address to a window aligned relative to `region.base_address`, reads at most `64 KiB + INVENTORY_RECORD_BYTES - 1`, and reuses the current window while it contains the complete record. It must:

```rust
let relative = address
    .checked_sub(region.base_address)
    .ok_or(InventoryProbeError::AddressOverflow)?;
let window_base = region
    .base_address
    .checked_add((relative / INVENTORY_VALIDATION_WINDOW_BYTES) * INVENTORY_VALIDATION_WINDOW_BYTES)
    .ok_or(InventoryProbeError::AddressOverflow)?;
let read_len = (region_end - window_base).min(
    INVENTORY_VALIDATION_WINDOW_BYTES
        .checked_add(INVENTORY_RECORD_BYTES - 1)
        .ok_or(InventoryProbeError::AddressOverflow)?,
);
```

- verify the record range is fully inside the region;
- check the shared deadline before loading a new window;
- increment `requested_bytes` with checked arithmetic;
- check the shared deadline before every complete record decode, not only when loading a new window;
- increment `validated_record_count` for every complete decode attempt;
- return `Ok(None)` for a decoder rejection or region boundary;
- propagate `MemoryReadError` so the caller rejects the entire affected anchor;
- return `InventoryProbeError::DeadlineExceeded` when the deadline expires.

Use this signature:

```rust
fn decode_record_at<R: MemoryReader>(
    reader: &R,
    region: MemoryRegion,
    address: usize,
    catalog: &InventoryCatalog,
    started: Instant,
    deadline: ScanDeadline,
    metrics: &mut ScanMetrics,
    window: &mut ValidationWindow,
) -> Result<Option<equipment_core::InventorySigilRecord>, InventoryProbeError>
```

- [ ] **Step 4: Expand one anchor backward and forward**

Implement `validate_inventory_anchor` with checked `0x24` stride arithmetic:

```rust
fn validate_inventory_anchor<R: MemoryReader>(
    reader: &R,
    region: MemoryRegion,
    anchor: usize,
    catalog: &InventoryCatalog,
    started: Instant,
    deadline: ScanDeadline,
    metrics: &mut ScanMetrics,
) -> Result<Option<InventoryCandidate>, InventoryProbeError>
```

Required behavior:

1. Fully decode the anchor and require `record.is_occupied()`.
2. Walk backward until `decode_record_at` returns `None`; count valid occupied records and retain the earliest address.
3. Reset or reuse the window, then walk forward from `anchor + INVENTORY_RECORD_BYTES` until `None`.
4. Return `None` unless `record_count >= MIN_RECORDS` and `occupied_count >= MIN_OCCUPIED`.
5. Return the exact earliest address, total valid records, and occupied count in `InventoryCandidate`.
6. Propagate deadline expiry.
7. Let the caller catch a memory-read error and discard this anchor; do not turn a partial expansion into a candidate.

- [ ] **Step 5: Replace exhaustive phase maps with anchor validation**

Rewrite `scan_inventory` to:

1. Start one `Instant` and initialize `ScanMetrics::new(regions.len())`.
2. Build the matcher and call `discover_inventory_anchors`.
3. For each sorted anchor, find the containing `MemoryRegion`.
4. Skip an anchor only when it is on the same record phase inside an already accepted candidate:

```rust
fn candidate_contains_anchor(candidate: InventoryCandidate, anchor: usize) -> bool {
    let Some(size) = candidate.record_count.checked_mul(INVENTORY_RECORD_BYTES) else {
        return false;
    };
    let Some(end) = candidate.base_address.checked_add(size) else {
        return false;
    };
    anchor >= candidate.base_address
        && anchor < end
        && (anchor - candidate.base_address) % INVENTORY_RECORD_BYTES == 0
}
```

5. Validate different-phase overlapping anchors independently.
6. Deduplicate only candidates with the same base address and record count.
7. On `MemoryReadError`, discard that anchor and continue; on `DeadlineExceeded`, return `InventoryScanOutcome::LimitExceeded` with completed metrics; propagate other internal errors.
8. Set `validated_run_count` after deduplication.
9. Classify zero, one, or multiple candidates exactly as before.
10. Check the deadline once more before final classification, so the last matcher or validation operation cannot return success after 10 seconds.
11. Set `metrics.elapsed` on every terminal path.

Extend the metrics type and centralize initialization:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScanMetrics {
    pub region_count: usize,
    pub requested_bytes: u64,
    pub anchor_count: usize,
    pub validated_run_count: usize,
    pub validated_record_count: usize,
    pub elapsed: Duration,
}

impl ScanMetrics {
    fn new(region_count: usize) -> Self {
        Self {
            region_count,
            requested_bytes: 0,
            anchor_count: 0,
            validated_run_count: 0,
            validated_record_count: 0,
            elapsed: Duration::ZERO,
        }
    }
}
```

Use the `(anchors, discovery_bytes)` result from Task 2 to initialize `metrics.requested_bytes` and `metrics.anchor_count`. Replace every existing test and fixture call from `ScanLimits::new(byte_count, duration)` to `ScanDeadline::new(duration)`. Remove the old byte-limit assertion from `rejects_changed_second_read_and_enforces_limits`; the pure 10-second deadline test from Task 2 replaces it.

Delete the obsolete `RunState`, per-phase `HashMap`, `ScanLimits`, `INVENTORY_MAX_BYTES`, `INVENTORY_MAX_DURATION`, byte-limit checks, and exhaustive every-four-byte decode loop. Remove imports made unused by those deletions.

- [ ] **Step 6: Wire the production 10-second deadline and diagnostics**

Add the production constant:

```rust
const INVENTORY_SCAN_DEADLINE: Duration = Duration::from_secs(10);
```

Change `capture_once` to call:

```rust
ScanDeadline::new(INVENTORY_SCAN_DEADLINE)
```

Update the scan log without exposing new raw data:

```rust
log::warn!(
    "INVENTORY PROBE scan regions={} requested_bytes={} anchors={} validated_runs={} elapsed_ms={}",
    metrics.region_count,
    metrics.requested_bytes,
    metrics.anchor_count,
    metrics.validated_run_count,
    metrics.elapsed.as_millis()
);
```

Do not log `validated_record_count`; it exists to prove work bounds in tests.

- [ ] **Step 7: Run all inventory tests and verify GREEN**

Run:

```powershell
cargo test --locked --package gbfr-logs equipment_probe::inventory::tests
cargo test --locked --package equipment-core inventory::tests
```

Expected: all discovery, candidate, limit, stability, ambiguity, and existing regression tests pass.

- [ ] **Step 8: Run security assertions**

Run:

```powershell
npm test -- --run src/securityConfiguration.test.ts
```

Expected: all security tests pass and continue to reject process-write, memory-operation, allocation, code-patching, and remote-thread APIs.

- [ ] **Step 9: Format, inspect, and commit**

Run:

```powershell
rustfmt --edition 2021 src-tauri/src/equipment_probe/inventory.rs
git diff --check
git diff -- src-tauri/src/equipment_probe/inventory.rs src/securityConfiguration.test.ts
```

Stage only files changed for candidate validation and commit:

```powershell
git add -- src-tauri/src/equipment_probe/inventory.rs src/securityConfiguration.test.ts
git commit -m "perf: validate inventory candidates around anchors"
```

If `src/securityConfiguration.test.ts` required no change, do not stage it.

---

### Task 4: Validate the optimized scanner against the running game

**Files:**
- Modify only after a verified stable result: `docs/testing/game-2.0.2-inventory-probe.md`

**Interfaces:**
- Consumes: pinned running `granblue_fantasy_relink.exe`, `DJEETA_INVENTORY_PROBE=1`, unfiltered in-game sigil inventory, and backend diagnostic logs.
- Produces: actual-game proof that the complete scan terminates in at most 10 seconds and a checklist row only if `STABLE` counts match the UI.

- [ ] **Step 1: Confirm the pinned process without elevation**

Use a read-only process lookup and hash its executable. Require:

```text
63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F
```

Do not proceed if the process is absent or the hash differs. Do not restart or stop the game unless the user asks.

- [ ] **Step 2: Start the debug app with only the probe opt-in**

Run:

```powershell
$env:DJEETA_INVENTORY_PROBE = "1"
npm run tauri dev
```

Expected: the app starts as the current standard user and shows the capture control.

- [ ] **Step 3: Capture once from the unfiltered inventory**

Have the user open the unfiltered sigil inventory and select **Capture owned sigils** once. Record the stable public status and this diagnostic summary:

```text
regions, requested_bytes, anchors, validated_runs, elapsed_ms
```

Success for this stage requires `STABLE` with `elapsed_ms <= 10000`. `UNAVAILABLE`, `AMBIGUOUS`, `UNSTABLE`, `LIMIT_EXCEEDED`, or `INTERNAL` returns to root-cause investigation; do not raise the deadline or accept a partial result.

- [ ] **Step 4: Verify stable counts before documenting compatibility evidence**

Only if the result is `STABLE`, compare candidate record and occupied counts with the in-game unfiltered count. Do not record raw memory addresses, full lists, player names, or raw bytes.

- [ ] **Step 5: Update and commit the checklist only when verified**

If the `STABLE` counts match, update the baseline row in `docs/testing/game-2.0.2-inventory-probe.md` with PID, record count, occupied count, digest, UI count, and result, then commit:

```powershell
git add -- docs/testing/game-2.0.2-inventory-probe.md
git commit -m "docs: record optimized inventory probe baseline"
```

If the status is not `STABLE` or the UI count is unavailable, leave the checklist unchanged and skip this commit.

---

### Task 5: Run full regression verification and review

**Files:**
- No source changes expected.

**Interfaces:**
- Consumes: Tasks 1-4 implementation and runtime evidence.
- Produces: a merge-ready scanner-performance branch only when automated checks pass and no Critical or Important review findings remain.

- [ ] **Step 1: Run frontend verification**

Run:

```powershell
npm run format-check
npm run lint
npm run tsc
npm test -- --run
npm run build
```

Expected: every command exits zero. Existing dependency and Rollup chunk-size warnings may remain but must not be changed in this stage.

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
git diff --stat master...HEAD
```

Expected: only the approved design clarification, plan, catalog interface, matcher dependency, optimized scanner, tests, and any verified checklist evidence are committed. `logs.db` remains untracked and unstaged.

- [ ] **Step 4: Request code review**

Review `master..HEAD` for:

- complete full-region known-ID coverage and chunk-boundary correctness;
- little-endian ID construction and `+0x10` anchor recovery;
- different-phase overlap handling and exact candidate deduplication;
- preservation of ambiguity and stable reread;
- one 10-second deadline with no byte limit or partial success;
- unchanged read-only process rights and security assertions;
- work-bounding test coverage and actual-game timing evidence.

Fix every Critical or Important finding and rerun the affected tests before offering integration options.

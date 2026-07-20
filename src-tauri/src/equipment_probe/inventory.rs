use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use equipment_core::{
    decode_inventory_record, InventoryCatalog, InventoryDecodeError, INVENTORY_RECORD_BYTES,
};
use sha2::{Digest, Sha256};

use super::memory::{MemoryReadError, MemoryReader, MemoryRegion};
#[cfg(windows)]
use super::{memory::RemoteProcess, GAME_PROCESS_NAME, PINNED_GAME_SHA256};

pub(crate) const INVENTORY_SCAN_CHUNK_BYTES: usize = 4 * 1024 * 1024;
const MIN_RECORDS: usize = 13;
const MIN_OCCUPIED: usize = 6;
const INVENTORY_STABILITY_DELAY: Duration = Duration::from_millis(50);
const INVENTORY_MAX_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const INVENTORY_MAX_DURATION: Duration = Duration::from_secs(60);

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
        Self::Disabled,
        Self::AlreadyRunning,
        Self::GameNotRunning,
        Self::UnsupportedGame,
        Self::Unavailable,
        Self::Ambiguous,
        Self::Unstable,
        Self::LimitExceeded,
        Self::Internal,
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

#[derive(Debug, Clone, Default)]
pub(crate) struct InventoryProbeState {
    running: Arc<AtomicBool>,
}

impl InventoryProbeState {
    fn try_begin(&self) -> Result<InventoryProbeRunGuard, InventoryProbeCode> {
        self.running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| InventoryProbeCode::AlreadyRunning)?;
        Ok(InventoryProbeRunGuard {
            running: Arc::clone(&self.running),
        })
    }
}

#[derive(Debug)]
struct InventoryProbeRunGuard {
    running: Arc<AtomicBool>,
}

impl Drop for InventoryProbeRunGuard {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
    }
}

pub(crate) fn inventory_probe_enabled(debug_build: bool, env_value: Option<&str>) -> bool {
    debug_build && env_value == Some("1")
}

fn current_probe_enabled() -> bool {
    let env_value = std::env::var("DJEETA_INVENTORY_PROBE").ok();
    inventory_probe_enabled(cfg!(debug_assertions), env_value.as_deref())
}

#[tauri::command]
pub(crate) fn inventory_probe_available() -> bool {
    current_probe_enabled()
}

#[tauri::command]
pub(crate) async fn capture_inventory_probe(
    state: tauri::State<'_, InventoryProbeState>,
) -> Result<(), String> {
    if !current_probe_enabled() {
        return Err(InventoryProbeCode::Disabled.as_str().to_string());
    }
    let guard = state
        .try_begin()
        .map_err(|code| code.as_str().to_string())?;
    match tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        capture_once()
    })
    .await
    {
        Ok(result) => result.map_err(|code| code.as_str().to_string()),
        Err(_) => {
            log::warn!("INVENTORY PROBE status=INTERNAL stage=worker");
            Err(InventoryProbeCode::Internal.as_str().to_string())
        }
    }
}

#[cfg(windows)]
fn capture_once() -> Result<(), InventoryProbeCode> {
    if !current_probe_enabled() {
        return Err(InventoryProbeCode::Disabled);
    }

    let process = match RemoteProcess::find(GAME_PROCESS_NAME)
        .map_err(|_| log_internal("process-discovery"))?
    {
        Some(process) => process,
        None => {
            log_final_status(InventoryProbeCode::GameNotRunning);
            return Err(InventoryProbeCode::GameNotRunning);
        }
    };
    let executable_hash = process
        .executable_sha256()
        .map_err(|_| log_internal("executable-hash"))?
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<String>();
    log::warn!(
        "INVENTORY PROBE process pid={} sha256={} rights=PROCESS_QUERY_LIMITED_INFORMATION|PROCESS_VM_READ",
        process.pid,
        executable_hash
    );
    if executable_hash != PINNED_GAME_SHA256 {
        log_final_status(InventoryProbeCode::UnsupportedGame);
        return Err(InventoryProbeCode::UnsupportedGame);
    }

    let catalog = load_inventory_catalog().map_err(|_| log_internal("catalog"))?;
    let regions = process
        .readable_private_regions()
        .map_err(|_| log_internal("region-enumeration"))?;
    let (outcome, metrics) = scan_inventory(
        &process,
        &regions,
        &catalog,
        ScanLimits::new(INVENTORY_MAX_BYTES, INVENTORY_MAX_DURATION),
    )
    .map_err(|_| log_internal("scan"))?;
    log::warn!(
        "INVENTORY PROBE scan regions={} requested_bytes={} elapsed_ms={}",
        metrics.region_count,
        metrics.requested_bytes,
        metrics.elapsed.as_millis()
    );

    let candidate = match outcome {
        InventoryScanOutcome::Unique(candidate) => candidate,
        InventoryScanOutcome::Unavailable => {
            log_final_status(InventoryProbeCode::Unavailable);
            return Err(InventoryProbeCode::Unavailable);
        }
        InventoryScanOutcome::Ambiguous { .. } => {
            log_final_status(InventoryProbeCode::Ambiguous);
            return Err(InventoryProbeCode::Ambiguous);
        }
        InventoryScanOutcome::LimitExceeded => {
            log_final_status(InventoryProbeCode::LimitExceeded);
            return Err(InventoryProbeCode::LimitExceeded);
        }
    };

    let first = read_candidate(&process, candidate, &catalog)
        .map_err(|_| log_internal("candidate-first-read"))?;
    thread::sleep(INVENTORY_STABILITY_DELAY);
    let second = read_candidate(&process, candidate, &catalog)
        .map_err(|_| log_internal("candidate-second-read"))?;
    if first != second {
        log_final_status(InventoryProbeCode::Unstable);
        return Err(InventoryProbeCode::Unstable);
    }

    log::warn!(
        "INVENTORY PROBE candidate address={:#x} records={} occupied={} digest={}",
        candidate.base_address,
        candidate.record_count,
        candidate.occupied_count,
        snapshot_digest_prefix(&first)
    );
    log::warn!("INVENTORY PROBE status=OK");
    Ok(())
}

#[cfg(not(windows))]
fn capture_once() -> Result<(), InventoryProbeCode> {
    Err(InventoryProbeCode::Internal)
}

fn log_internal(stage: &str) -> InventoryProbeCode {
    log::warn!("INVENTORY PROBE status=INTERNAL stage={stage}");
    InventoryProbeCode::Internal
}

fn log_final_status(code: InventoryProbeCode) {
    log::warn!("INVENTORY PROBE status={}", code.as_str());
}

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
    Memory(#[from] MemoryReadError),
    #[error(transparent)]
    Decode(#[from] InventoryDecodeError),
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
        Self {
            max_bytes,
            max_duration,
        }
    }

    pub(crate) fn exceeded(self, bytes: u64, elapsed: Duration) -> bool {
        bytes > self.max_bytes || elapsed > self.max_duration
    }
}

fn parse_catalog_keys(source: &str) -> Result<HashSet<u32>, InventoryProbeError> {
    let rows: std::collections::HashMap<String, serde_json::Value> = serde_json::from_str(source)?;
    rows.into_keys()
        .map(|key| {
            u32::from_str_radix(&key, 16).map_err(|_| InventoryProbeError::InvalidCatalogKey(key))
        })
        .collect()
}

pub(crate) fn load_inventory_catalog() -> Result<InventoryCatalog, InventoryProbeError> {
    Ok(InventoryCatalog::new(
        parse_catalog_keys(include_str!("../../lang/en/sigils.json"))?,
        parse_catalog_keys(include_str!("../../lang/en/traits.json"))?,
    ))
}

#[derive(Debug, Default)]
struct RunState {
    last_tested: Option<usize>,
    first_occupied: Option<usize>,
    last_occupied: Option<usize>,
    occupied_count: usize,
}

impl RunState {
    fn finish(&mut self, candidates: &mut Vec<InventoryCandidate>) {
        if let (Some(first), Some(last)) = (self.first_occupied, self.last_occupied) {
            let record_count = (last - first) / INVENTORY_RECORD_BYTES + 1;
            if record_count >= MIN_RECORDS && self.occupied_count >= MIN_OCCUPIED {
                candidates.push(InventoryCandidate {
                    base_address: first,
                    record_count,
                    occupied_count: self.occupied_count,
                });
            }
        }
        self.first_occupied = None;
        self.last_occupied = None;
        self.occupied_count = 0;
    }

    fn observe(
        &mut self,
        address: usize,
        occupied: Option<bool>,
        candidates: &mut Vec<InventoryCandidate>,
    ) -> Result<(), InventoryProbeError> {
        if self.last_tested.is_some_and(|last| address <= last) {
            return Ok(());
        }
        if self
            .last_tested
            .is_some_and(|last| last.checked_add(INVENTORY_RECORD_BYTES) != Some(address))
        {
            self.finish(candidates);
        }
        self.last_tested = Some(address);

        match occupied {
            Some(true) => {
                self.first_occupied.get_or_insert(address);
                self.last_occupied = Some(address);
                self.occupied_count = self
                    .occupied_count
                    .checked_add(1)
                    .ok_or(InventoryProbeError::AddressOverflow)?;
            }
            Some(false) => {}
            None => self.finish(candidates),
        }
        Ok(())
    }
}

pub(crate) fn scan_inventory<R: MemoryReader>(
    reader: &R,
    regions: &[MemoryRegion],
    catalog: &InventoryCatalog,
    limits: ScanLimits,
) -> Result<(InventoryScanOutcome, ScanMetrics), InventoryProbeError> {
    let started = Instant::now();
    let mut requested_bytes = 0u64;
    let mut candidates = Vec::new();

    for region in regions {
        let mut runs = HashMap::<usize, RunState>::new();
        let end = region.end().ok_or(InventoryProbeError::AddressOverflow)?;
        let mut chunk_address = region.base_address;
        while chunk_address < end {
            let elapsed = started.elapsed();
            if limits.exceeded(requested_bytes, elapsed) {
                return Ok((
                    InventoryScanOutcome::LimitExceeded,
                    ScanMetrics {
                        region_count: regions.len(),
                        requested_bytes,
                        elapsed,
                    },
                ));
            }

            let remaining = end - chunk_address;
            let read_len = remaining.min(
                INVENTORY_SCAN_CHUNK_BYTES
                    .checked_add(INVENTORY_RECORD_BYTES - 1)
                    .ok_or(InventoryProbeError::AddressOverflow)?,
            );
            let next_requested = requested_bytes
                .checked_add(
                    u64::try_from(read_len).map_err(|_| InventoryProbeError::AddressOverflow)?,
                )
                .ok_or(InventoryProbeError::AddressOverflow)?;
            if limits.exceeded(next_requested, elapsed) {
                return Ok((
                    InventoryScanOutcome::LimitExceeded,
                    ScanMetrics {
                        region_count: regions.len(),
                        requested_bytes,
                        elapsed,
                    },
                ));
            }

            let mut bytes = vec![0u8; read_len];
            reader.read_exact(chunk_address, &mut bytes)?;
            requested_bytes = next_requested;

            if bytes.len() >= INVENTORY_RECORD_BYTES {
                for offset in (0..=bytes.len() - INVENTORY_RECORD_BYTES).step_by(4) {
                    let address = chunk_address
                        .checked_add(offset)
                        .ok_or(InventoryProbeError::AddressOverflow)?;
                    let phase = address % INVENTORY_RECORD_BYTES;
                    let occupied = decode_inventory_record(&bytes[offset..], catalog)
                        .ok()
                        .map(|record| record.is_occupied());
                    runs.entry(phase)
                        .or_default()
                        .observe(address, occupied, &mut candidates)?;
                }
            }

            chunk_address = chunk_address
                .checked_add(INVENTORY_SCAN_CHUNK_BYTES)
                .ok_or(InventoryProbeError::AddressOverflow)?;
        }
        for run in runs.values_mut() {
            run.finish(&mut candidates);
        }
    }

    let outcome = match candidates.as_slice() {
        [] => InventoryScanOutcome::Unavailable,
        [candidate] => InventoryScanOutcome::Unique(*candidate),
        _ => InventoryScanOutcome::Ambiguous {
            count: candidates.len(),
        },
    };
    Ok((
        outcome,
        ScanMetrics {
            region_count: regions.len(),
            requested_bytes,
            elapsed: started.elapsed(),
        },
    ))
}

pub(crate) fn read_candidate<R: MemoryReader>(
    reader: &R,
    candidate: InventoryCandidate,
    catalog: &InventoryCatalog,
) -> Result<Vec<u8>, InventoryProbeError> {
    let byte_count = candidate
        .record_count
        .checked_mul(INVENTORY_RECORD_BYTES)
        .ok_or(InventoryProbeError::AddressOverflow)?;
    candidate
        .base_address
        .checked_add(byte_count)
        .ok_or(InventoryProbeError::AddressOverflow)?;
    let mut bytes = vec![0u8; byte_count];
    reader.read_exact(candidate.base_address, &mut bytes)?;
    for record in bytes.chunks_exact(INVENTORY_RECORD_BYTES) {
        decode_inventory_record(record, catalog)?;
    }
    Ok(bytes)
}

pub(crate) fn snapshot_digest_prefix(bytes: &[u8]) -> String {
    Sha256::digest(bytes)[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, time::Duration};

    use equipment_core::{InventoryCatalog, INVENTORY_RECORD_BYTES};

    use super::{
        inventory_probe_enabled, load_inventory_catalog, read_candidate, scan_inventory,
        snapshot_digest_prefix, InventoryCandidate, InventoryProbeCode, InventoryProbeError,
        InventoryProbeState, InventoryScanOutcome, ScanLimits, INVENTORY_SCAN_CHUNK_BYTES,
    };
    use crate::equipment_probe::memory::{MemoryReadError, MemoryReader, MemoryRegion};

    const BASE: usize = 0x1000_0000;
    const SECOND_BASE: usize = 0x2000_0000;
    const SIGIL_ID: u32 = 0x0045_57B8;
    const TRAIT_ID: u32 = 0x0053_599E;

    struct FakeMemory {
        regions: Vec<MemoryRegion>,
        bytes: Vec<(usize, Vec<u8>)>,
    }

    impl MemoryReader for FakeMemory {
        fn read_exact(&self, address: usize, output: &mut [u8]) -> Result<(), MemoryReadError> {
            let (start, bytes) = self
                .bytes
                .iter()
                .find(|(start, bytes)| {
                    address >= *start
                        && address
                            .checked_add(output.len())
                            .is_some_and(|end| end <= *start + bytes.len())
                })
                .ok_or(MemoryReadError::Unavailable(address))?;
            let offset = address - start;
            output.copy_from_slice(&bytes[offset..offset + output.len()]);
            Ok(())
        }
    }

    struct ChangingMemory {
        address: usize,
        first: Vec<u8>,
        second: Vec<u8>,
        reads: Cell<usize>,
    }

    impl MemoryReader for ChangingMemory {
        fn read_exact(&self, address: usize, output: &mut [u8]) -> Result<(), MemoryReadError> {
            if address != self.address {
                return Err(MemoryReadError::Unavailable(address));
            }
            let read = self.reads.get();
            self.reads.set(read + 1);
            let source = if read == 0 { &self.first } else { &self.second };
            output.copy_from_slice(source);
            Ok(())
        }
    }

    fn put(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn catalog() -> InventoryCatalog {
        let catalog = load_inventory_catalog().unwrap();
        decode_fixture_catalog(&catalog).expect("bundled catalog contains fixture IDs");
        catalog
    }

    fn decode_fixture_catalog(catalog: &InventoryCatalog) -> Result<(), InventoryProbeError> {
        equipment_core::decode_inventory_record(&occupied_record(), catalog)?;
        Ok(())
    }

    fn occupied_record() -> [u8; INVENTORY_RECORD_BYTES] {
        let mut bytes = [0u8; INVENTORY_RECORD_BYTES];
        put(&mut bytes, 0x00, TRAIT_ID);
        put(&mut bytes, 0x04, 15);
        put(&mut bytes, 0x10, SIGIL_ID);
        put(&mut bytes, 0x18, 15);
        bytes
    }

    fn records(record_count: usize, occupied_count: usize) -> Vec<u8> {
        let mut bytes = vec![0u8; record_count * INVENTORY_RECORD_BYTES];
        let mut occupied_indexes = (0..occupied_count.min(record_count)).collect::<Vec<_>>();
        if occupied_count > 1 && occupied_count < record_count {
            *occupied_indexes.last_mut().expect("occupied index exists") = record_count - 1;
        }
        for index in occupied_indexes {
            let start = index * INVENTORY_RECORD_BYTES;
            bytes[start..start + INVENTORY_RECORD_BYTES].copy_from_slice(&occupied_record());
        }
        bytes
    }

    fn inventory_fixture(record_count: usize, occupied_count: usize) -> FakeMemory {
        let bytes = records(record_count, occupied_count);
        FakeMemory {
            regions: vec![MemoryRegion {
                base_address: BASE,
                size: bytes.len(),
            }],
            bytes: vec![(BASE, bytes)],
        }
    }

    fn two_inventory_fixture() -> FakeMemory {
        let first = records(13, 6);
        let second = records(14, 7);
        FakeMemory {
            regions: vec![
                MemoryRegion {
                    base_address: BASE,
                    size: first.len(),
                },
                MemoryRegion {
                    base_address: SECOND_BASE,
                    size: second.len(),
                },
            ],
            bytes: vec![(BASE, first), (SECOND_BASE, second)],
        }
    }

    fn boundary_fixture(run_offset: usize) -> FakeMemory {
        let run = records(13, 6);
        let mut bytes = vec![0xFF; run_offset + run.len()];
        bytes[run_offset..].copy_from_slice(&run);
        FakeMemory {
            regions: vec![MemoryRegion {
                base_address: BASE,
                size: bytes.len(),
            }],
            bytes: vec![(BASE, bytes)],
        }
    }

    fn scan_fixture(memory: FakeMemory) -> InventoryScanOutcome {
        scan_inventory(
            &memory,
            &memory.regions,
            &catalog(),
            ScanLimits::new(32 * 1024 * 1024, Duration::from_secs(60)),
        )
        .unwrap()
        .0
    }

    fn verify_changed_fixture() -> Result<(), InventoryProbeError> {
        let first = records(13, 6);
        let mut second = first.clone();
        put(&mut second, 0x1C, 1);
        let memory = ChangingMemory {
            address: BASE,
            first,
            second,
            reads: Cell::new(0),
        };
        let candidate = InventoryCandidate {
            base_address: BASE,
            record_count: 13,
            occupied_count: 6,
        };
        let first = read_candidate(&memory, candidate, &catalog())?;
        let second = read_candidate(&memory, candidate, &catalog())?;
        if first != second {
            return Err(InventoryProbeError::Unstable);
        }
        Ok(())
    }

    #[test]
    fn excludes_a_twelve_record_equipment_snapshot() {
        let memory = inventory_fixture(12, 12);
        assert_eq!(scan_fixture(memory), InventoryScanOutcome::Unavailable);
    }

    #[test]
    fn accepts_one_thirteen_record_run_with_six_occupied_records() {
        let memory = inventory_fixture(13, 6);
        let InventoryScanOutcome::Unique(candidate) = scan_fixture(memory) else {
            panic!()
        };
        assert_eq!(candidate.record_count, 13);
        assert_eq!(candidate.occupied_count, 6);
    }

    #[test]
    fn reports_two_qualified_runs_as_ambiguous() {
        let memory = two_inventory_fixture();
        assert!(matches!(
            scan_fixture(memory),
            InventoryScanOutcome::Ambiguous { count: 2 }
        ));
    }

    #[test]
    fn finds_a_record_that_crosses_a_chunk_boundary_without_duplicate_candidates() {
        let memory = boundary_fixture(INVENTORY_SCAN_CHUNK_BYTES - 8);
        let InventoryScanOutcome::Unique(candidate) = scan_fixture(memory) else {
            panic!()
        };
        assert_eq!(candidate.occupied_count, 6);
    }

    #[test]
    fn rejects_changed_second_read_and_enforces_limits() {
        assert!(matches!(
            verify_changed_fixture(),
            Err(InventoryProbeError::Unstable)
        ));
        assert!(ScanLimits::new(16, Duration::from_secs(60)).exceeded(17, Duration::ZERO));
        assert!(ScanLimits::new(16, Duration::from_secs(60)).exceeded(1, Duration::from_secs(61)));
    }

    #[test]
    fn formatted_results_do_not_expose_raw_record_values() {
        let outcome = scan_fixture(inventory_fixture(13, 6));
        let summary = format!("{outcome:?}");
        assert!(!summary.contains(&format!("{SIGIL_ID:08x}")));
        assert!(!summary.contains(&format!("{TRAIT_ID:08x}")));

        let digest = snapshot_digest_prefix(&records(13, 6));
        assert_eq!(digest.len(), 16);
        assert!(digest
            .chars()
            .all(|character| character.is_ascii_hexdigit()));
    }

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
        assert_eq!(
            state.try_begin().unwrap_err(),
            InventoryProbeCode::AlreadyRunning
        );
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
}

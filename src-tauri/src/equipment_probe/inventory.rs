use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, AhoCorasickKind};
use equipment_core::{
    decode_inventory_record, InventoryCatalog, InventoryDecodeError, INVENTORY_RECORD_BYTES,
};
use sha2::{Digest, Sha256};

use super::memory::{MemoryReadError, MemoryReader, MemoryRegion};
#[cfg(windows)]
use super::{memory::RemoteProcess, GAME_PROCESS_NAME, PINNED_GAME_SHA256};

pub(crate) const INVENTORY_SCAN_CHUNK_BYTES: usize = 8 * 1024 * 1024;
const SIGIL_PATTERN_OVERLAP: usize = std::mem::size_of::<u32>() - 1;
const MIN_RECORDS: usize = 13;
const MIN_OCCUPIED: usize = 6;
const INVENTORY_STABILITY_DELAY: Duration = Duration::from_millis(50);
const INVENTORY_SCAN_DEADLINE: Duration = Duration::from_secs(10);
const INVENTORY_VALIDATION_WINDOW_BYTES: usize = 64 * 1024;

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
        "INVENTORY PROBE process pid={} sha256={} rights=PROCESS_QUERY_INFORMATION|PROCESS_VM_READ",
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
        ScanDeadline::new(INVENTORY_SCAN_DEADLINE),
    )
    .map_err(|_| log_internal("scan"))?;
    log::warn!(
        "INVENTORY PROBE scan regions={} requested_bytes={} anchors={} validated_runs={} elapsed_ms={}",
        metrics.region_count,
        metrics.requested_bytes,
        metrics.anchor_count,
        metrics.validated_run_count,
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
        .map_err(|error| map_candidate_read_error(&error, "candidate-first-read"))?;
    thread::sleep(INVENTORY_STABILITY_DELAY);
    let second = read_candidate_bytes(&process, candidate)
        .map_err(|error| map_candidate_read_error(&error, "candidate-second-read"))?;
    verify_candidate_snapshots(&first, &second).map_err(|_| {
        log_final_status(InventoryProbeCode::Unstable);
        InventoryProbeCode::Unstable
    })?;

    log::warn!(
        "INVENTORY PROBE candidate address={:#x} records={} occupied={} digest={}",
        candidate.base_address,
        candidate.record_count,
        candidate.occupied_count,
        snapshot_digest_prefix(&first)
    );
    log::warn!("INVENTORY PROBE status=STABLE");
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

fn map_candidate_read_error(error: &InventoryProbeError, stage: &str) -> InventoryProbeCode {
    if matches!(error, InventoryProbeError::Memory(_)) {
        log_final_status(InventoryProbeCode::Unavailable);
        InventoryProbeCode::Unavailable
    } else {
        log_internal(stage)
    }
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

#[derive(Debug, thiserror::Error)]
pub(crate) enum InventoryProbeError {
    #[error(transparent)]
    Memory(#[from] MemoryReadError),
    #[error(transparent)]
    Decode(#[from] InventoryDecodeError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Matcher(#[from] aho_corasick::BuildError),
    #[error("inventory scan deadline exceeded")]
    DeadlineExceeded,
    #[error("invalid inventory catalog key {0}")]
    InvalidCatalogKey(String),
    #[error("inventory address range overflow")]
    AddressOverflow,
    #[error("inventory changed between stable reads")]
    Unstable,
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

fn build_sigil_matcher(catalog: &InventoryCatalog) -> Result<AhoCorasick, InventoryProbeError> {
    let patterns = catalog
        .known_non_empty_sigil_ids()
        .map(u32::to_le_bytes)
        .collect::<Vec<_>>();
    Ok(AhoCorasickBuilder::new()
        .kind(Some(AhoCorasickKind::DFA))
        .build(patterns.iter().map(|pattern| pattern.as_slice()))?)
}

fn find_inventory_anchors(
    bytes: &[u8],
    chunk_base: usize,
    region: MemoryRegion,
    matcher: &AhoCorasick,
) -> Result<Vec<usize>, InventoryProbeError> {
    const SIGIL_FIELD_OFFSET: usize = 0x10;

    let region_end = region.end().ok_or(InventoryProbeError::AddressOverflow)?;
    let mut match_starts = Vec::new();
    for matched in matcher.find_iter(bytes) {
        match_starts.push(matched.start());
        for delta in 1..std::mem::size_of::<u32>() {
            let Some(start) = matched.start().checked_add(delta) else {
                break;
            };
            let Some(end) = start.checked_add(std::mem::size_of::<u32>()) else {
                break;
            };
            if end > bytes.len() {
                break;
            }
            if matcher.is_match(&bytes[start..end]) {
                match_starts.push(start);
            }
        }
    }

    let mut anchors = Vec::new();
    for matched_start in match_starts {
        let field_address = chunk_base
            .checked_add(matched_start)
            .ok_or(InventoryProbeError::AddressOverflow)?;
        let Some(record_address) = field_address.checked_sub(SIGIL_FIELD_OFFSET) else {
            continue;
        };
        let record_end = record_address
            .checked_add(INVENTORY_RECORD_BYTES)
            .ok_or(InventoryProbeError::AddressOverflow)?;
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

fn discover_inventory_anchors<R: MemoryReader>(
    reader: &R,
    regions: &[MemoryRegion],
    matcher: &AhoCorasick,
    started: Instant,
    deadline: ScanDeadline,
    metrics: &mut ScanMetrics,
) -> Result<Vec<usize>, InventoryProbeError> {
    let mut anchors = Vec::new();
    let mut bytes = Vec::new();
    let max_read_len = INVENTORY_SCAN_CHUNK_BYTES
        .checked_add(SIGIL_PATTERN_OVERLAP)
        .ok_or(InventoryProbeError::AddressOverflow)?;

    for region in regions {
        let end = region.end().ok_or(InventoryProbeError::AddressOverflow)?;
        let mut chunk_address = region.base_address;
        while chunk_address < end {
            if deadline.exceeded(started.elapsed()) {
                return Err(InventoryProbeError::DeadlineExceeded);
            }

            let read_len = (end - chunk_address).min(max_read_len);
            metrics.requested_bytes = metrics
                .requested_bytes
                .checked_add(
                    u64::try_from(read_len).map_err(|_| InventoryProbeError::AddressOverflow)?,
                )
                .ok_or(InventoryProbeError::AddressOverflow)?;
            bytes.resize(read_len, 0);
            if reader.read_exact(chunk_address, &mut bytes).is_ok() {
                anchors.extend(find_inventory_anchors(
                    &bytes,
                    chunk_address,
                    *region,
                    matcher,
                )?);
                if deadline.exceeded(started.elapsed()) {
                    return Err(InventoryProbeError::DeadlineExceeded);
                }
            }

            chunk_address = chunk_address
                .checked_add(INVENTORY_SCAN_CHUNK_BYTES)
                .ok_or(InventoryProbeError::AddressOverflow)?;
        }
    }

    anchors.sort_unstable();
    anchors.dedup();
    metrics.anchor_count = anchors.len();
    Ok(anchors)
}

#[derive(Debug, Default)]
struct ValidationWindow {
    base_address: usize,
    bytes: Vec<u8>,
}

impl ValidationWindow {
    fn contains_record(&self, address: usize) -> bool {
        address >= self.base_address
            && address
                .checked_add(INVENTORY_RECORD_BYTES)
                .is_some_and(|end| {
                    self.base_address
                        .checked_add(self.bytes.len())
                        .is_some_and(|window_end| end <= window_end)
                })
    }
}

fn decode_record_at<R: MemoryReader>(
    reader: &R,
    region: MemoryRegion,
    address: usize,
    catalog: &InventoryCatalog,
    started: Instant,
    deadline: ScanDeadline,
    metrics: &mut ScanMetrics,
    window: &mut ValidationWindow,
) -> Result<Option<equipment_core::InventorySigilRecord>, InventoryProbeError> {
    if deadline.exceeded(started.elapsed()) {
        return Err(InventoryProbeError::DeadlineExceeded);
    }

    let region_end = region.end().ok_or(InventoryProbeError::AddressOverflow)?;
    let Some(record_end) = address.checked_add(INVENTORY_RECORD_BYTES) else {
        return Err(InventoryProbeError::AddressOverflow);
    };
    if address < region.base_address || record_end > region_end {
        return Ok(None);
    }

    if !window.contains_record(address) {
        let relative = address
            .checked_sub(region.base_address)
            .ok_or(InventoryProbeError::AddressOverflow)?;
        let window_offset = (relative / INVENTORY_VALIDATION_WINDOW_BYTES)
            .checked_mul(INVENTORY_VALIDATION_WINDOW_BYTES)
            .ok_or(InventoryProbeError::AddressOverflow)?;
        let window_base = region
            .base_address
            .checked_add(window_offset)
            .ok_or(InventoryProbeError::AddressOverflow)?;
        let max_read_len = INVENTORY_VALIDATION_WINDOW_BYTES
            .checked_add(INVENTORY_RECORD_BYTES - 1)
            .ok_or(InventoryProbeError::AddressOverflow)?;
        let read_len = (region_end - window_base).min(max_read_len);
        metrics.requested_bytes = metrics
            .requested_bytes
            .checked_add(u64::try_from(read_len).map_err(|_| InventoryProbeError::AddressOverflow)?)
            .ok_or(InventoryProbeError::AddressOverflow)?;
        let mut bytes = vec![0u8; read_len];
        reader.read_exact(window_base, &mut bytes)?;
        window.base_address = window_base;
        window.bytes = bytes;
    }

    if deadline.exceeded(started.elapsed()) {
        return Err(InventoryProbeError::DeadlineExceeded);
    }
    let offset = address
        .checked_sub(window.base_address)
        .ok_or(InventoryProbeError::AddressOverflow)?;
    metrics.validated_record_count = metrics
        .validated_record_count
        .checked_add(1)
        .ok_or(InventoryProbeError::AddressOverflow)?;
    match decode_inventory_record(&window.bytes[offset..], catalog) {
        Ok(record) => Ok(Some(record)),
        Err(_) => Ok(None),
    }
}

fn validate_inventory_anchor<R: MemoryReader>(
    reader: &R,
    region: MemoryRegion,
    anchor: usize,
    catalog: &InventoryCatalog,
    started: Instant,
    deadline: ScanDeadline,
    metrics: &mut ScanMetrics,
) -> Result<Option<InventoryCandidate>, InventoryProbeError> {
    let mut window = ValidationWindow::default();
    let Some(anchor_record) = decode_record_at(
        reader,
        region,
        anchor,
        catalog,
        started,
        deadline,
        metrics,
        &mut window,
    )?
    else {
        return Ok(None);
    };
    if !anchor_record.is_occupied() {
        return Ok(None);
    }

    let mut first_occupied = anchor;
    let mut last_occupied = anchor;
    let mut occupied_count = 1usize;
    let mut current = anchor;
    while let Some(previous) = current.checked_sub(INVENTORY_RECORD_BYTES) {
        let Some(record) = decode_record_at(
            reader,
            region,
            previous,
            catalog,
            started,
            deadline,
            metrics,
            &mut window,
        )?
        else {
            break;
        };
        current = previous;
        if record.is_occupied() {
            first_occupied = previous;
            occupied_count = occupied_count
                .checked_add(1)
                .ok_or(InventoryProbeError::AddressOverflow)?;
        }
    }

    current = anchor;
    loop {
        let next = current
            .checked_add(INVENTORY_RECORD_BYTES)
            .ok_or(InventoryProbeError::AddressOverflow)?;
        let Some(record) = decode_record_at(
            reader,
            region,
            next,
            catalog,
            started,
            deadline,
            metrics,
            &mut window,
        )?
        else {
            break;
        };
        current = next;
        if record.is_occupied() {
            last_occupied = next;
            occupied_count = occupied_count
                .checked_add(1)
                .ok_or(InventoryProbeError::AddressOverflow)?;
        }
    }

    let record_count = last_occupied
        .checked_sub(first_occupied)
        .ok_or(InventoryProbeError::AddressOverflow)?
        / INVENTORY_RECORD_BYTES
        + 1;
    if record_count < MIN_RECORDS || occupied_count < MIN_OCCUPIED {
        return Ok(None);
    }
    Ok(Some(InventoryCandidate {
        base_address: first_occupied,
        record_count,
        occupied_count,
    }))
}

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

fn region_containing_record(regions: &[MemoryRegion], address: usize) -> Option<MemoryRegion> {
    let record_end = address.checked_add(INVENTORY_RECORD_BYTES)?;
    regions.iter().copied().find(|region| {
        region
            .end()
            .is_some_and(|region_end| address >= region.base_address && record_end <= region_end)
    })
}

pub(crate) fn scan_inventory<R: MemoryReader>(
    reader: &R,
    regions: &[MemoryRegion],
    catalog: &InventoryCatalog,
    deadline: ScanDeadline,
) -> Result<(InventoryScanOutcome, ScanMetrics), InventoryProbeError> {
    let started = Instant::now();
    let mut metrics = ScanMetrics::new(regions.len());
    let mut candidates = Vec::<InventoryCandidate>::new();

    let matcher = build_sigil_matcher(catalog)?;
    let anchors = match discover_inventory_anchors(
        reader,
        regions,
        &matcher,
        started,
        deadline,
        &mut metrics,
    ) {
        Ok(result) => result,
        Err(InventoryProbeError::DeadlineExceeded) => {
            metrics.elapsed = started.elapsed();
            return Ok((InventoryScanOutcome::LimitExceeded, metrics));
        }
        Err(error) => return Err(error),
    };
    for anchor in anchors {
        if candidates
            .iter()
            .copied()
            .any(|candidate| candidate_contains_anchor(candidate, anchor))
        {
            continue;
        }
        let Some(region) = region_containing_record(regions, anchor) else {
            continue;
        };
        match validate_inventory_anchor(
            reader,
            region,
            anchor,
            catalog,
            started,
            deadline,
            &mut metrics,
        ) {
            Ok(Some(candidate)) => {
                if !candidates.iter().any(|existing| {
                    existing.base_address == candidate.base_address
                        && existing.record_count == candidate.record_count
                }) {
                    candidates.push(candidate);
                }
            }
            Ok(None) | Err(InventoryProbeError::Memory(_)) => {}
            Err(InventoryProbeError::DeadlineExceeded) => {
                metrics.elapsed = started.elapsed();
                return Ok((InventoryScanOutcome::LimitExceeded, metrics));
            }
            Err(error) => return Err(error),
        }
    }

    metrics.validated_run_count = candidates.len();
    if deadline.exceeded(started.elapsed()) {
        metrics.elapsed = started.elapsed();
        return Ok((InventoryScanOutcome::LimitExceeded, metrics));
    }

    let outcome = match candidates.as_slice() {
        [] => InventoryScanOutcome::Unavailable,
        [candidate] => InventoryScanOutcome::Unique(*candidate),
        _ => InventoryScanOutcome::Ambiguous {
            count: candidates.len(),
        },
    };
    metrics.elapsed = started.elapsed();
    Ok((outcome, metrics))
}

pub(crate) fn read_candidate<R: MemoryReader>(
    reader: &R,
    candidate: InventoryCandidate,
    catalog: &InventoryCatalog,
) -> Result<Vec<u8>, InventoryProbeError> {
    let bytes = read_candidate_bytes(reader, candidate)?;
    for record in bytes.chunks_exact(INVENTORY_RECORD_BYTES) {
        decode_inventory_record(record, catalog)?;
    }
    Ok(bytes)
}

pub(crate) fn read_candidate_bytes<R: MemoryReader>(
    reader: &R,
    candidate: InventoryCandidate,
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
    Ok(bytes)
}

pub(crate) fn verify_candidate_snapshots(
    first: &[u8],
    second: &[u8],
) -> Result<(), InventoryProbeError> {
    if first == second {
        Ok(())
    } else {
        Err(InventoryProbeError::Unstable)
    }
}

pub(crate) fn snapshot_digest_prefix(bytes: &[u8]) -> String {
    Sha256::digest(bytes)[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{
        cell::Cell,
        time::{Duration, Instant},
    };

    use equipment_core::{InventoryCatalog, INVENTORY_RECORD_BYTES};

    use super::{
        build_sigil_matcher, discover_inventory_anchors, find_inventory_anchors,
        inventory_probe_enabled, load_inventory_catalog, map_candidate_read_error, read_candidate,
        read_candidate_bytes, scan_inventory, snapshot_digest_prefix, verify_candidate_snapshots,
        InventoryCandidate, InventoryProbeCode, InventoryProbeError, InventoryProbeState,
        InventoryScanOutcome, ScanDeadline, ScanMetrics, INVENTORY_SCAN_CHUNK_BYTES,
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
            ScanDeadline::new(Duration::from_secs(10)),
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
        let second = read_candidate_bytes(&memory, candidate)?;
        verify_candidate_snapshots(&first, &second)
    }

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
    fn an_unaligned_match_does_not_hide_an_overlapping_aligned_anchor() {
        let first = u32::from_le_bytes([1, 2, 3, 4]);
        let second = u32::from_le_bytes([4, 5, 6, 7]);
        let catalog = InventoryCatalog::new(
            std::collections::HashSet::from([first, second]),
            std::collections::HashSet::new(),
        );
        let matcher = build_sigil_matcher(&catalog).unwrap();
        let region = MemoryRegion {
            base_address: BASE,
            size: 0x100,
        };
        let mut bytes = vec![0xA5; region.size];
        bytes[0x4D..0x51].copy_from_slice(&first.to_le_bytes());
        bytes[0x50..0x54].copy_from_slice(&second.to_le_bytes());

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
        let mut metrics = ScanMetrics::new(memory.regions.len());

        let anchors = discover_inventory_anchors(
            &memory,
            &memory.regions,
            &matcher,
            started,
            deadline,
            &mut metrics,
        )
        .unwrap();

        assert_eq!(
            anchors
                .iter()
                .filter(|address| **address == BASE + run_offset)
                .count(),
            1
        );
        assert!(metrics.requested_bytes > 0);
    }

    #[test]
    fn deadline_has_no_byte_limit_and_expires_at_ten_seconds() {
        let deadline = ScanDeadline::new(Duration::from_secs(10));

        assert!(!deadline.exceeded(Duration::from_secs(9)));
        assert!(!deadline.exceeded(Duration::from_secs(10)));
        assert!(deadline.exceeded(Duration::from_secs(10) + Duration::from_nanos(1)));
    }

    #[test]
    fn validates_records_only_around_discovered_anchors() {
        let mut memory = inventory_fixture(13, 6);
        let mut decoy = vec![0xA5; 144 * 1024 * 1024];
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

        assert!(
            matches!(outcome, InventoryScanOutcome::Unique(_)),
            "outcome={outcome:?} metrics={metrics:?}"
        );
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

    #[test]
    fn candidate_range_starts_and_ends_on_occupied_records() {
        let mut bytes = vec![0u8; 20 * INVENTORY_RECORD_BYTES];
        for index in [3, 4, 5, 6, 7, 8, 15] {
            let start = index * INVENTORY_RECORD_BYTES;
            bytes[start..start + INVENTORY_RECORD_BYTES].copy_from_slice(&occupied_record());
        }
        let memory = FakeMemory {
            regions: vec![MemoryRegion {
                base_address: BASE,
                size: bytes.len(),
            }],
            bytes: vec![(BASE, bytes)],
        };

        let (outcome, _) = scan_inventory(
            &memory,
            &memory.regions,
            &catalog(),
            ScanDeadline::new(Duration::from_secs(10)),
        )
        .unwrap();
        let InventoryScanOutcome::Unique(candidate) = outcome else {
            panic!("expected one candidate, got {outcome:?}");
        };

        assert_eq!(candidate.base_address, BASE + 3 * INVENTORY_RECORD_BYTES);
        assert_eq!(candidate.record_count, 13);
        assert_eq!(candidate.occupied_count, 7);
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
    fn skips_a_region_that_becomes_unreadable_during_the_scan() {
        let valid = records(13, 6);
        let memory = FakeMemory {
            regions: vec![
                MemoryRegion {
                    base_address: BASE,
                    size: INVENTORY_RECORD_BYTES * 13,
                },
                MemoryRegion {
                    base_address: SECOND_BASE,
                    size: valid.len(),
                },
            ],
            bytes: vec![(SECOND_BASE, valid)],
        };

        let InventoryScanOutcome::Unique(candidate) = scan_fixture(memory) else {
            panic!()
        };
        assert_eq!(candidate.base_address, SECOND_BASE);
    }

    #[test]
    fn rejects_changed_second_read() {
        assert!(matches!(
            verify_changed_fixture(),
            Err(InventoryProbeError::Unstable)
        ));
    }

    #[test]
    fn reports_an_invalidated_second_snapshot_as_unstable() {
        let first = records(13, 6);
        let second = vec![0xFF; first.len()];
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

        let first = read_candidate(&memory, candidate, &catalog()).unwrap();
        let second = read_candidate_bytes(&memory, candidate).unwrap();
        assert!(matches!(
            verify_candidate_snapshots(&first, &second),
            Err(InventoryProbeError::Unstable)
        ));
    }

    #[test]
    fn maps_an_unreadable_candidate_to_unavailable() {
        let error = InventoryProbeError::Memory(MemoryReadError::Unavailable(BASE));
        assert_eq!(
            map_candidate_read_error(&error, "test"),
            InventoryProbeCode::Unavailable
        );
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

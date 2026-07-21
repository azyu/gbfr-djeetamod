use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use equipment_core::SIGIL_ARRAY_BYTES;
use log::warn;
use protocol::LocalEquipmentSnapshotEvent;
use tauri::{AppHandle, Manager};
use tokio::time::sleep;

use self::{
    compare::{snapshot_digest_prefix, CompareDecision, DeferredReason},
    locator::{locate_from_globals_slot, resolve_roots, LocateError},
    memory::{MemoryReadError, MemoryReader, RemoteProcess},
    roster_probe::{
        classify_candidate_snapshot, inspect_candidate_manager, CandidateSnapshotStatus,
        RosterProbeError,
    },
};
use crate::parser::constants::is_known_equipment_character_hash;

mod compare;
pub(crate) mod inventory;
mod locator;
pub(crate) mod memory;
mod roster_probe;

#[derive(Debug, Default)]
pub(crate) struct ProbeState(Mutex<compare::ProbeComparator>);

pub(crate) fn record_hook_snapshot(app: &AppHandle, event: LocalEquipmentSnapshotEvent) {
    app.state::<ProbeState>()
        .0
        .lock()
        .expect("equipment probe comparator lock poisoned")
        .record_hook(event);
}

pub(crate) fn begin_hook_session(app: &AppHandle) {
    app.state::<ProbeState>()
        .0
        .lock()
        .expect("equipment probe comparator lock poisoned")
        .begin_hook_session();
}

pub(crate) const GAME_PROCESS_NAME: &str = "granblue_fantasy_relink.exe";
pub(crate) const PINNED_GAME_SHA256: &str =
    "63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F";
const POLL_INTERVAL: Duration = Duration::from_millis(250);
const STABILITY_DELAY: Duration = Duration::from_millis(50);
const DISCOVERY_DELAY: Duration = Duration::from_secs(1);
const LOG_REPEAT_INTERVAL: Duration = Duration::from_secs(5);
const ROSTER_INSPECTION_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug, thiserror::Error)]
enum CandidateReadError {
    #[error(transparent)]
    Locate(#[from] LocateError),
    #[error(transparent)]
    Memory(#[from] MemoryReadError),
}

impl CandidateReadError {
    fn is_expected_empty_slot(&self) -> bool {
        matches!(self, Self::Locate(LocateError::EmptyCharacterKey))
    }
}

pub(crate) fn probe_enabled(debug_build: bool, env_value: Option<&str>) -> bool {
    debug_build && env_value == Some("1")
}

#[derive(Default)]
struct ProbeLogThrottle(HashMap<String, Instant>);

impl ProbeLogThrottle {
    fn allows(&mut self, key: String, now: Instant) -> bool {
        if self
            .0
            .get(&key)
            .is_some_and(|previous| now.saturating_duration_since(*previous) < LOG_REPEAT_INTERVAL)
        {
            return false;
        }
        self.0.insert(key, now);
        true
    }
}

pub(crate) async fn run_if_enabled(app: AppHandle) {
    let env_value = std::env::var("DJEETA_EXTERNAL_READER_PROBE").ok();
    if !probe_enabled(cfg!(debug_assertions), env_value.as_deref()) {
        return;
    }

    let mut throttle = ProbeLogThrottle::default();
    let mut announced_pid = None;
    loop {
        sleep(DISCOVERY_DELAY).await;
        let process = match RemoteProcess::find(GAME_PROCESS_NAME) {
            Ok(Some(process)) => process,
            Ok(None) => {
                announced_pid = None;
                continue;
            }
            Err(error) => {
                log_unavailable(&mut throttle, "process-discovery", &error);
                continue;
            }
        };

        let hash = match process.executable_sha256() {
            Ok(hash) => hash
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<String>(),
            Err(error) => {
                log_unavailable(&mut throttle, "executable-hash", &error);
                continue;
            }
        };
        if hash != PINNED_GAME_SHA256 {
            if throttle.allows(format!("hash:{}", process.pid), Instant::now()) {
                warn!(
                    "PROBE UNAVAILABLE stage=executable-hash pid={} sha256={} module_base={:#x} rights=PROCESS_QUERY_INFORMATION|PROCESS_VM_READ expected={}",
                    process.pid, hash, process.module_base, PINNED_GAME_SHA256
                );
            }
            announced_pid = Some(process.pid);
            continue;
        }
        if announced_pid != Some(process.pid) {
            warn!(
                "PROBE MATCH process pid={} sha256={} module_base={:#x} rights=PROCESS_QUERY_INFORMATION|PROCESS_VM_READ",
                process.pid, hash, process.module_base
            );
            announced_pid = Some(process.pid);
        }

        if let Err(error) = probe_process(&app, &process, &mut throttle).await {
            log_unavailable(&mut throttle, "process-read", &error);
        }
    }
}

async fn probe_process(
    app: &AppHandle,
    process: &RemoteProcess,
    throttle: &mut ProbeLogThrottle,
) -> Result<(), String> {
    let (text_address, text) = process
        .read_text_section()
        .map_err(|error| error.to_string())?;
    let text_rva = text_address
        .checked_sub(process.module_base)
        .ok_or_else(|| "text address precedes module base".to_string())?;
    let roots = resolve_roots(process, process.module_base, text_rva, &text)
        .map_err(|error| error.to_string())?;
    let local_key_rva = roots
        .local_key_global
        .checked_sub(process.module_base)
        .ok_or_else(|| "local-key global precedes module base".to_string())?;
    let manager_rva = roots
        .manager_global
        .checked_sub(process.module_base)
        .ok_or_else(|| "player-manager global precedes module base".to_string())?;
    warn!(
        "PROBE MATCH locator signature_matches=1 match_rva={:#x} local_key_global_rva={:#x} player_manager_global_rva={:#x}",
        roots.match_rva, local_key_rva, manager_rva
    );

    let mut next_roster_inspection = Instant::now();
    loop {
        let deadline = Instant::now() + POLL_INTERVAL;
        for slot in 0..4 {
            let (first_location, first, second_location, second) =
                match read_candidate(process, roots.local_key_global, roots.manager_global, slot)
                    .await
                {
                    Ok(candidate) => candidate,
                    Err(error) => {
                        if error.is_expected_empty_slot() {
                            continue;
                        }
                        let stage = format!("equipment-read-slot-{slot}");
                        log_unavailable(throttle, &stage, &error);
                        match process.is_running() {
                            Ok(true) => continue,
                            Ok(false) => return Ok(()),
                            Err(error) => return Err(error.to_string()),
                        }
                    }
                };

            let decision = if first_location.character_key != second_location.character_key {
                CompareDecision::Deferred(DeferredReason::UnstableRead)
            } else {
                app.state::<ProbeState>()
                    .0
                    .lock()
                    .expect("equipment probe comparator lock poisoned")
                    .compare_external(
                        first_location.character_key,
                        &first,
                        &second,
                        Instant::now(),
                    )
            };
            log_decision(throttle, first_location.character_key, &first, decision);
        }

        if Instant::now() >= next_roster_inspection {
            if let Err(error) = probe_candidate_roster(process, roots.manager_global).await {
                log_unavailable(throttle, "roster-inspection", &error);
            }
            next_roster_inspection = Instant::now() + ROSTER_INSPECTION_INTERVAL;
        }

        sleep(deadline.saturating_duration_since(Instant::now())).await;
    }
}

async fn probe_candidate_roster(
    process: &RemoteProcess,
    manager_global: usize,
) -> Result<(), RosterProbeError> {
    let first_inspection = inspect_candidate_manager(process, manager_global)?;
    let known_count = first_inspection
        .records
        .iter()
        .filter(|record| is_known_equipment_character_hash(record.character_key))
        .count();
    warn!(
        "ROSTER PROBE CANDIDATE buckets={} known={} unknown={} duplicates={} rejected={}",
        first_inspection.candidate_bucket_count,
        known_count,
        first_inspection.records.len().saturating_sub(known_count),
        first_inspection.duplicate_keys.len(),
        first_inspection.rejected_record_count
    );

    let first_reads = first_inspection
        .records
        .iter()
        .filter(|record| is_known_equipment_character_hash(record.character_key))
        .map(|record| {
            (
                record.character_key,
                record.snapshot_address,
                read_roster_snapshot(process, record.snapshot_address),
            )
        })
        .collect::<Vec<_>>();

    sleep(STABILITY_DELAY).await;
    let second_inspection = inspect_candidate_manager(process, manager_global)?;
    let second_records = second_inspection
        .records
        .into_iter()
        .map(|record| (record.character_key, record))
        .collect::<HashMap<_, _>>();

    for (character_key, first_address, first) in first_reads {
        let status = match second_records.get(&character_key) {
            Some(second_record) if second_record.snapshot_address == first_address => {
                let second = read_roster_snapshot(process, second_record.snapshot_address);
                classify_candidate_snapshot(
                    character_key,
                    first.as_ref().map(|bytes| bytes.as_slice()),
                    second.as_ref().map(|bytes| bytes.as_slice()),
                )
            }
            Some(_) => CandidateSnapshotStatus::Unstable,
            None => CandidateSnapshotStatus::Unavailable,
        };
        log_roster_status(character_key, status);
    }

    Ok(())
}

fn read_roster_snapshot(
    process: &RemoteProcess,
    snapshot_address: Option<usize>,
) -> Option<[u8; SIGIL_ARRAY_BYTES]> {
    let snapshot_address = snapshot_address?;
    let mut snapshot = [0u8; SIGIL_ARRAY_BYTES];
    process.read_exact(snapshot_address, &mut snapshot).ok()?;
    Some(snapshot)
}

fn log_roster_status(character_key: u32, status: CandidateSnapshotStatus) {
    match status {
        CandidateSnapshotStatus::Stable {
            source_count,
            digest,
        } => warn!(
            "ROSTER PROBE CANDIDATE character_key={:#010x} status=stable sources={} digest={}",
            character_key, source_count, digest
        ),
        CandidateSnapshotStatus::Unavailable => warn!(
            "ROSTER PROBE CANDIDATE character_key={:#010x} status=unavailable sources=0 digest=none",
            character_key
        ),
        CandidateSnapshotStatus::Unstable => warn!(
            "ROSTER PROBE CANDIDATE character_key={:#010x} status=unstable sources=0 digest=none",
            character_key
        ),
        CandidateSnapshotStatus::Invalid => warn!(
            "ROSTER PROBE CANDIDATE character_key={:#010x} status=invalid sources=0 digest=none",
            character_key
        ),
    }
}

async fn read_candidate(
    process: &RemoteProcess,
    local_key_global: usize,
    manager_global: usize,
    slot: usize,
) -> Result<
    (
        locator::LocatedEquipment,
        [u8; SIGIL_ARRAY_BYTES],
        locator::LocatedEquipment,
        [u8; SIGIL_ARRAY_BYTES],
    ),
    CandidateReadError,
> {
    let first_location = locate_from_globals_slot(process, local_key_global, manager_global, slot)?;
    let mut first = [0u8; SIGIL_ARRAY_BYTES];
    process.read_exact(first_location.snapshot_address, &mut first)?;

    sleep(STABILITY_DELAY).await;
    let second_location =
        locate_from_globals_slot(process, local_key_global, manager_global, slot)?;
    let mut second = [0u8; SIGIL_ARRAY_BYTES];
    process.read_exact(second_location.snapshot_address, &mut second)?;
    Ok((first_location, first, second_location, second))
}

fn log_decision(
    throttle: &mut ProbeLogThrottle,
    character_key: u32,
    snapshot: &[u8],
    decision: CompareDecision,
) {
    match decision {
        CompareDecision::Match(summary) => warn!(
            "PROBE MATCH character_key={:#010x} sources={} digest={}",
            summary.character_key, summary.source_count, summary.snapshot_digest
        ),
        CompareDecision::Mismatch(differences) => warn!(
            "PROBE MISMATCH character_key={:#010x} digest={} differences={differences:?}",
            character_key,
            snapshot_digest_prefix(snapshot)
        ),
        CompareDecision::Deferred(reason) => {
            let key = format!("deferred:{character_key:08x}:{reason:?}");
            if throttle.allows(key, Instant::now()) {
                warn!(
                    "PROBE DEFERRED character_key={:#010x} reason={reason:?}",
                    character_key
                );
            }
        }
        CompareDecision::Suppressed => {}
    }
}

fn log_unavailable(throttle: &mut ProbeLogThrottle, stage: &str, error: &dyn std::fmt::Display) {
    if throttle.allows(format!("unavailable:{stage}"), Instant::now()) {
        warn!("PROBE UNAVAILABLE stage={stage} error={error}");
    }
}

#[cfg(test)]
mod tests {
    use super::{probe_enabled, CandidateReadError};
    use crate::equipment_probe::locator::LocateError;

    #[test]
    fn probe_requires_debug_build_and_exact_opt_in() {
        assert!(probe_enabled(true, Some("1")));
        assert!(!probe_enabled(true, None));
        assert!(!probe_enabled(true, Some("true")));
        assert!(!probe_enabled(false, Some("1")));
    }

    #[test]
    fn empty_party_slot_is_not_reportable() {
        let error = CandidateReadError::Locate(LocateError::EmptyCharacterKey);

        assert!(error.is_expected_empty_slot());
    }
}

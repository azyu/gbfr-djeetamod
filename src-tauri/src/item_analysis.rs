use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use equipment_core::{
    decode_item_inventory, warning_items, ItemInventoryDecodeError, OwnedItem, ITEM_MAX_QUANTITY,
    ITEM_WARNING_THRESHOLD,
};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::equipment_probe::{
    memory::{MemoryReader, MemoryRegion, RemoteProcess},
    GAME_PROCESS_NAME, PINNED_GAME_SHA256,
};

pub(crate) const INVENTORY_REGION_BYTES: usize = 243_269_632;
const STABILITY_DELAY: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItemAnalysisCode {
    AlreadyRunning,
    GameNotRunning,
    UnsupportedGame,
    Unavailable,
    Unstable,
    Internal,
}

impl ItemAnalysisCode {
    fn as_str(self) -> &'static str {
        match self {
            Self::AlreadyRunning => "ALREADY_RUNNING",
            Self::GameNotRunning => "GAME_NOT_RUNNING",
            Self::UnsupportedGame => "UNSUPPORTED_GAME",
            Self::Unavailable => "UNAVAILABLE",
            Self::Unstable => "UNSTABLE",
            Self::Internal => "INTERNAL",
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct ItemAnalysisState {
    running: Arc<AtomicBool>,
}

impl ItemAnalysisState {
    fn try_begin(&self) -> Result<ItemAnalysisGuard, ItemAnalysisCode> {
        self.running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| ItemAnalysisCode::AlreadyRunning)?;
        Ok(ItemAnalysisGuard {
            running: Arc::clone(&self.running),
        })
    }
}

#[derive(Debug)]
struct ItemAnalysisGuard {
    running: Arc<AtomicBool>,
}

impl Drop for ItemAnalysisGuard {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ItemAnalysisResponse {
    inspected_at_ms: u64,
    threshold: u32,
    maximum: u32,
    items: Vec<OwnedItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ItemInventorySnapshotResponse {
    inspected_at_ms: u64,
    items: Vec<OwnedItem>,
}

impl From<ItemInventorySnapshotResponse> for ItemAnalysisResponse {
    fn from(snapshot: ItemInventorySnapshotResponse) -> Self {
        Self {
            inspected_at_ms: snapshot.inspected_at_ms,
            threshold: ITEM_WARNING_THRESHOLD,
            maximum: ITEM_MAX_QUANTITY,
            items: warning_items(&snapshot.items),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrdinaryItemCatalog {
    game_version: String,
    game_exe_sha256: String,
    item_ids: Vec<String>,
}

pub(crate) fn bundled_ordinary_item_ids() -> Result<HashSet<u32>, ItemAnalysisCode> {
    let catalog: OrdinaryItemCatalog =
        serde_json::from_str(include_str!("../data/ordinary-items-2.0.2.json"))
            .map_err(|_| ItemAnalysisCode::Internal)?;
    if catalog.game_version != "2.0.2" || catalog.game_exe_sha256 != PINNED_GAME_SHA256 {
        return Err(ItemAnalysisCode::Internal);
    }

    let ids = catalog
        .item_ids
        .into_iter()
        .map(|item_id| u32::from_str_radix(&item_id, 16).map_err(|_| ItemAnalysisCode::Internal))
        .collect::<Result<HashSet<_>, _>>()?;
    if ids.len() != 281 {
        return Err(ItemAnalysisCode::Internal);
    }
    Ok(ids)
}

pub(crate) fn select_inventory_region(
    regions: &[MemoryRegion],
) -> Result<MemoryRegion, ItemAnalysisCode> {
    let matches = regions
        .iter()
        .copied()
        .filter(|region| region.size == INVENTORY_REGION_BYTES)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [region] => Ok(*region),
        _ => Err(ItemAnalysisCode::Unavailable),
    }
}

fn decode_snapshot(
    bytes: &[u8],
    ordinary_item_ids: &HashSet<u32>,
) -> Result<Vec<OwnedItem>, ItemAnalysisCode> {
    decode_item_inventory(bytes, ordinary_item_ids).map_err(|error| match error {
        ItemInventoryDecodeError::DuplicateItem(_) => ItemAnalysisCode::Unavailable,
    })
}

pub(crate) fn stable_inventory_snapshot(
    first: &[u8],
    second: &[u8],
    ordinary_item_ids: &HashSet<u32>,
) -> Result<ItemInventorySnapshotResponse, ItemAnalysisCode> {
    let first = decode_snapshot(first, ordinary_item_ids)?;
    let second = decode_snapshot(second, ordinary_item_ids)?;
    if first != second {
        return Err(ItemAnalysisCode::Unstable);
    }
    let inspected_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ItemAnalysisCode::Internal)?
        .as_millis()
        .try_into()
        .map_err(|_| ItemAnalysisCode::Internal)?;

    Ok(ItemInventorySnapshotResponse {
        inspected_at_ms,
        items: first,
    })
}

pub(crate) fn analyze_snapshots(
    first: &[u8],
    second: &[u8],
    ordinary_item_ids: &HashSet<u32>,
) -> Result<ItemAnalysisResponse, ItemAnalysisCode> {
    stable_inventory_snapshot(first, second, ordinary_item_ids).map(Into::into)
}

fn read_region(process: &RemoteProcess, region: MemoryRegion) -> Result<Vec<u8>, ItemAnalysisCode> {
    let mut bytes = vec![0; region.size];
    process
        .read_exact(region.base_address, &mut bytes)
        .map_err(|_| ItemAnalysisCode::Unavailable)?;
    Ok(bytes)
}

fn analyze_process() -> Result<ItemInventorySnapshotResponse, ItemAnalysisCode> {
    let started = Instant::now();
    let process = RemoteProcess::find(GAME_PROCESS_NAME)
        .map_err(|_| ItemAnalysisCode::Internal)?
        .ok_or(ItemAnalysisCode::GameNotRunning)?;
    let executable_hash = process
        .executable_sha256()
        .map_err(|_| ItemAnalysisCode::Internal)?
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<String>();
    if executable_hash != PINNED_GAME_SHA256 {
        return Err(ItemAnalysisCode::UnsupportedGame);
    }

    let ordinary_item_ids = bundled_ordinary_item_ids()?;
    let regions = process
        .readable_private_regions()
        .map_err(|_| ItemAnalysisCode::Unavailable)?;
    let region = select_inventory_region(&regions)?;
    let first = read_region(&process, region)?;
    std::thread::sleep(STABILITY_DELAY);
    let second = read_region(&process, region)?;
    let response = stable_inventory_snapshot(&first, &second, &ordinary_item_ids)?;
    log::warn!(
        "ITEM ANALYSIS MATCH pid={} elapsed_ms={} decoded_count={}",
        process.pid,
        started.elapsed().as_millis(),
        response.items.len()
    );
    Ok(response)
}

async fn fetch_snapshot(
    state: State<'_, ItemAnalysisState>,
) -> Result<ItemInventorySnapshotResponse, String> {
    let _guard = state.try_begin().map_err(|code| code.as_str().to_owned())?;
    tauri::async_runtime::spawn_blocking(analyze_process)
        .await
        .map_err(|_| ItemAnalysisCode::Internal.as_str().to_owned())?
        .map_err(|code| code.as_str().to_owned())
}

#[tauri::command]
pub(crate) async fn fetch_item_inventory_snapshot(
    state: State<'_, ItemAnalysisState>,
) -> Result<ItemInventorySnapshotResponse, String> {
    fetch_snapshot(state).await
}

#[tauri::command]
pub(crate) async fn fetch_item_analysis(
    state: State<'_, ItemAnalysisState>,
) -> Result<ItemAnalysisResponse, String> {
    fetch_snapshot(state).await.map(Into::into)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use equipment_core::{OwnedItem, ITEM_MAX_QUANTITY, ITEM_RECORD_BYTES, ITEM_WARNING_THRESHOLD};

    use super::{
        analyze_snapshots, bundled_ordinary_item_ids, select_inventory_region,
        stable_inventory_snapshot, ItemAnalysisCode, ItemAnalysisResponse, ItemAnalysisState,
        ItemInventorySnapshotResponse, INVENTORY_REGION_BYTES,
    };
    use crate::equipment_probe::memory::MemoryRegion;

    const ITEM_A: u32 = 0x6877_33c4;
    const ITEM_B: u32 = 0x2e94_d39a;

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn record(item_id: u32, quantity: u32) -> Vec<u8> {
        let mut bytes = vec![0; ITEM_RECORD_BYTES];
        put_u32(&mut bytes, 0x00, item_id);
        put_u32(&mut bytes, 0x04, quantity);
        put_u32(&mut bytes, 0x08, 0x0c);
        put_u32(&mut bytes, 0x10, 0);
        put_u32(&mut bytes, 0x14, u32::MAX);
        put_u32(&mut bytes, 0x18, u32::MAX);
        put_u32(&mut bytes, 0x1c, u32::MAX);
        bytes
    }

    #[test]
    fn returns_only_warning_items_from_two_equal_snapshots() {
        let bytes = [record(ITEM_A, 899), record(ITEM_B, 900)].concat();
        let response = analyze_snapshots(&bytes, &bytes, &HashSet::from([ITEM_A, ITEM_B])).unwrap();

        assert_eq!(
            response.items,
            vec![OwnedItem {
                item_id: ITEM_B,
                quantity: 900,
            }]
        );
        assert_eq!(response.threshold, 900);
        assert_eq!(response.maximum, 999);
    }

    #[test]
    fn full_snapshot_keeps_items_below_the_warning_threshold() {
        let bytes = [
            record(ITEM_A, ITEM_WARNING_THRESHOLD - 1),
            record(ITEM_B, ITEM_WARNING_THRESHOLD),
        ]
        .concat();

        let snapshot =
            stable_inventory_snapshot(&bytes, &bytes, &HashSet::from([ITEM_A, ITEM_B])).unwrap();

        assert_eq!(
            snapshot.items,
            vec![
                OwnedItem {
                    item_id: ITEM_B,
                    quantity: ITEM_WARNING_THRESHOLD,
                },
                OwnedItem {
                    item_id: ITEM_A,
                    quantity: ITEM_WARNING_THRESHOLD - 1,
                },
            ]
        );
    }

    #[test]
    fn warning_response_still_filters_the_complete_snapshot() {
        let snapshot = ItemInventorySnapshotResponse {
            inspected_at_ms: 123,
            items: vec![
                OwnedItem {
                    item_id: ITEM_A,
                    quantity: ITEM_WARNING_THRESHOLD - 1,
                },
                OwnedItem {
                    item_id: ITEM_B,
                    quantity: ITEM_WARNING_THRESHOLD,
                },
            ],
        };

        let response = ItemAnalysisResponse::from(snapshot);

        assert_eq!(
            response.items,
            vec![OwnedItem {
                item_id: ITEM_B,
                quantity: ITEM_WARNING_THRESHOLD,
            }]
        );
        assert_eq!(response.threshold, ITEM_WARNING_THRESHOLD);
        assert_eq!(response.maximum, ITEM_MAX_QUANTITY);
    }

    #[test]
    fn rejects_changed_second_snapshot() {
        let first = record(ITEM_B, 900);
        let second = record(ITEM_B, 901);

        assert_eq!(
            analyze_snapshots(&first, &second, &HashSet::from([ITEM_B])),
            Err(ItemAnalysisCode::Unstable)
        );
    }

    #[test]
    fn requires_one_inventory_region() {
        let expected = MemoryRegion {
            base_address: 0x2000_0000,
            size: INVENTORY_REGION_BYTES,
        };
        assert_eq!(
            select_inventory_region(&[
                MemoryRegion {
                    base_address: 0x1000_0000,
                    size: 0x1000,
                },
                expected,
            ]),
            Ok(expected)
        );
        assert_eq!(
            select_inventory_region(&[]),
            Err(ItemAnalysisCode::Unavailable)
        );
        assert_eq!(
            select_inventory_region(&[expected, expected]),
            Err(ItemAnalysisCode::Unavailable)
        );
    }

    #[test]
    fn state_rejects_overlapping_requests() {
        let state = ItemAnalysisState::default();
        let _guard = state.try_begin().unwrap();

        assert_eq!(
            state.try_begin().unwrap_err(),
            ItemAnalysisCode::AlreadyRunning
        );
    }

    #[test]
    fn bundled_catalog_is_version_pinned_and_complete() {
        let ids = bundled_ordinary_item_ids().unwrap();

        assert_eq!(ids.len(), 281);
        assert!(ids.contains(&ITEM_A));
        assert!(ids.contains(&0x0eb6_83cd));
        assert!(!ids.contains(&0xc93e_2fdd));
    }
}

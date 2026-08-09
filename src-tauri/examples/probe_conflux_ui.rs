#[path = "../src/equipment_probe/memory.rs"]
mod memory;

#[cfg(not(windows))]
compile_error!("probe_conflux_ui is Windows-only");

use std::{
    collections::HashMap,
    env,
    ops::Range,
    thread,
    time::{Duration, Instant},
};

use memory::{MemoryReader, RemoteProcess};
use sha2::{Digest, Sha256};

const GAME_PROCESS_NAME: &str = "granblue_fantasy_relink.exe";
const PINNED_GAME_SHA256: &str = "F827F3C13CAA90B290FAB2FE7E28165A80448FDE0A3F7A96D79DAC6B8343FF2A";
const PROBE_ENV: &str = "DJEETA_CONFLUX_UI_PROBE";
const TIMER_PROBE_ENV: &str = "DJEETA_CONFLUX_TIMER_PROBE";
const SCAN_CHUNK_BYTES: usize = 4 * 1024 * 1024;
const MODULE_SCAN_CHUNK_BYTES: usize = 64 * 1024;
const OBJECT_FINGERPRINT_BYTES: usize = 0x600;
const TIMER_OBJECT_BYTES: usize = 0x2000;
const TIMER_SAMPLE_DELAY: Duration = Duration::from_millis(500);
const TIMER_MANAGER_BYTES: usize = 0x347C;
const TIMER_NOTICE_THRESHOLD_OFFSET: usize = 0x2DA4;
const TIMER_DEFAULTS_OFFSET: usize = 0x2DA8;
const TIMER_DEFAULT_COUNT: usize = 11;
const TIMER_MODE_OFFSET: usize = 0x2DE0;
const TIMER_FLAGS_OFFSET: usize = 0x3468;
const TIMER_INITIAL_OFFSET: usize = 0x346C;
const TIMER_CURRENT_OFFSET: usize = 0x3470;
const TIMER_NOTICE_OFFSET: usize = 0x3474;
const ORIGINAL_TIMER_CONFIG: [f32; TIMER_DEFAULT_COUNT + 1] = [
    3.0, 60.0, 60.0, 30.0, 30.0, 30.0, 30.0, 30.0, 30.0, 60.0, 30.0, 30.0,
];
const FAST_TIMER_CONFIG: [f32; TIMER_DEFAULT_COUNT + 1] =
    [1.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0];
const MAX_FINGERPRINTS_PER_TARGET: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VtableTarget {
    label: &'static str,
    rva: usize,
}

const TARGETS: &[VtableTarget] = &[
    VtableTarget {
        label: "final_reward_controller.primary",
        rva: 0x0607_E1D8,
    },
    VtableTarget {
        label: "final_reward_controller.secondary_1",
        rva: 0x0607_E2F8,
    },
    VtableTarget {
        label: "final_reward_controller.secondary_2",
        rva: 0x0607_E308,
    },
    VtableTarget {
        label: "final_reward_controller.secondary_3",
        rva: 0x0607_E318,
    },
    VtableTarget {
        label: "final_reward_controller.secondary_4",
        rva: 0x0607_E330,
    },
    VtableTarget {
        label: "final_reward_controller.secondary_5",
        rva: 0x0607_E348,
    },
    VtableTarget {
        label: "endless_reward_menu.primary",
        rva: 0x0608_6DB0,
    },
    VtableTarget {
        label: "endless_reward_menu.secondary_1",
        rva: 0x0608_6F98,
    },
    VtableTarget {
        label: "endless_reward_menu.secondary_2",
        rva: 0x0608_6FA8,
    },
    VtableTarget {
        label: "endless_reward_menu.secondary_3",
        rva: 0x0608_6FC8,
    },
    VtableTarget {
        label: "endless_reward_menu.secondary_4",
        rva: 0x0608_6FE8,
    },
    VtableTarget {
        label: "result_reward_controller.primary",
        rva: 0x05C6_2128,
    },
    VtableTarget {
        label: "result_reward_controller.secondary_1",
        rva: 0x05C6_2248,
    },
    VtableTarget {
        label: "result_reward_controller.secondary_2",
        rva: 0x05C6_2258,
    },
    VtableTarget {
        label: "result_reward_controller.secondary_3",
        rva: 0x05C6_2268,
    },
    VtableTarget {
        label: "result_reward_controller.secondary_4",
        rva: 0x05C6_2280,
    },
    VtableTarget {
        label: "result_reward_controller.secondary_5",
        rva: 0x05C6_2298,
    },
    VtableTarget {
        label: "result_reward_menu.primary",
        rva: 0x0618_A490,
    },
    VtableTarget {
        label: "result_reward_menu.secondary_1",
        rva: 0x05F6_F8A8,
    },
    VtableTarget {
        label: "result_reward_menu.secondary_2",
        rva: 0x05F6_F8B8,
    },
    VtableTarget {
        label: "result_reward_menu.secondary_3",
        rva: 0x05F6_F8D8,
    },
    VtableTarget {
        label: "result_reward_menu.secondary_4",
        rva: 0x05F6_F8F8,
    },
    VtableTarget {
        label: "result_reward_treasure_menu.primary",
        rva: 0x0618_A210,
    },
    VtableTarget {
        label: "result_reward_treasure_menu.secondary_1",
        rva: 0x05F6_F418,
    },
    VtableTarget {
        label: "result_reward_treasure_menu.secondary_2",
        rva: 0x05F6_F428,
    },
    VtableTarget {
        label: "result_reward_treasure_menu.secondary_3",
        rva: 0x05F6_F448,
    },
    VtableTarget {
        label: "result_reward_treasure_menu.secondary_4",
        rva: 0x05F6_F468,
    },
    VtableTarget {
        label: "endless_result_info",
        rva: 0x0608_3A58,
    },
    VtableTarget {
        label: "endless_result_record",
        rva: 0x0607_DBC8,
    },
    VtableTarget {
        label: "endless_result_score",
        rva: 0x0608_34C8,
    },
    VtableTarget {
        label: "endless_difficulty",
        rva: 0x05C7_3688,
    },
    VtableTarget {
        label: "endless_top",
        rva: 0x05C7_6808,
    },
    VtableTarget {
        label: "endless_top_frame",
        rva: 0x05F9_1558,
    },
    VtableTarget {
        label: "preset_party",
        rva: 0x05F8_3148,
    },
    VtableTarget {
        label: "result_city_select_fsm",
        rva: 0x058E_0148,
    },
    VtableTarget {
        label: "endless_portal.primary",
        rva: 0x05C8_E0F0,
    },
    VtableTarget {
        label: "endless_portal.secondary_1",
        rva: 0x05C8_E488,
    },
    VtableTarget {
        label: "endless_portal.secondary_2",
        rva: 0x05C8_E568,
    },
    VtableTarget {
        label: "endless_portal.secondary_3",
        rva: 0x05C8_E5D8,
    },
    VtableTarget {
        label: "endless_portal.secondary_4",
        rva: 0x05C8_E698,
    },
    VtableTarget {
        label: "endless_portal.secondary_5",
        rva: 0x05C8_E6E8,
    },
    VtableTarget {
        label: "endless_portal.secondary_6",
        rva: 0x05C8_E728,
    },
    VtableTarget {
        label: "endless_portal.secondary_7",
        rva: 0x05C8_E778,
    },
    VtableTarget {
        label: "endless_portal.secondary_8",
        rva: 0x05C8_E7B8,
    },
    VtableTarget {
        label: "endless_portal.secondary_9",
        rva: 0x05C8_E868,
    },
    VtableTarget {
        label: "endless_portal.secondary_10",
        rva: 0x05C8_E8C0,
    },
    VtableTarget {
        label: "endless_portal.secondary_11",
        rva: 0x05C8_E998,
    },
    VtableTarget {
        label: "endless_gate_icon.primary",
        rva: 0x05A6_DC28,
    },
    VtableTarget {
        label: "endless_gate_icon.secondary_1",
        rva: 0x05A6_DD48,
    },
    VtableTarget {
        label: "endless_gate_icon.secondary_2",
        rva: 0x05A6_DD58,
    },
    VtableTarget {
        label: "endless_gate_icon.secondary_3",
        rva: 0x05A6_DD68,
    },
    VtableTarget {
        label: "endless_gate_icon.secondary_4",
        rva: 0x05A6_DD80,
    },
    VtableTarget {
        label: "endless_gate_icon.secondary_5",
        rva: 0x05A6_DD98,
    },
    VtableTarget {
        label: "endless_gate_icon.secondary_6",
        rva: 0x05A6_DDB8,
    },
    VtableTarget {
        label: "endless_event_result.primary",
        rva: 0x0608_0928,
    },
    VtableTarget {
        label: "endless_event_result.secondary_1",
        rva: 0x0608_0A48,
    },
    VtableTarget {
        label: "endless_event_result.secondary_2",
        rva: 0x0608_0A58,
    },
    VtableTarget {
        label: "endless_event_result.secondary_3",
        rva: 0x0608_0A68,
    },
    VtableTarget {
        label: "endless_event_result.secondary_4",
        rva: 0x0608_0A80,
    },
    VtableTarget {
        label: "endless_event_result.secondary_5",
        rva: 0x0608_0A98,
    },
    VtableTarget {
        label: "endless_boss_result.primary",
        rva: 0x0608_2538,
    },
    VtableTarget {
        label: "endless_boss_result.secondary_1",
        rva: 0x0608_2658,
    },
    VtableTarget {
        label: "endless_boss_result.secondary_2",
        rva: 0x0608_2668,
    },
    VtableTarget {
        label: "endless_boss_result.secondary_3",
        rva: 0x0608_2678,
    },
    VtableTarget {
        label: "endless_boss_result.secondary_4",
        rva: 0x0608_2690,
    },
    VtableTarget {
        label: "endless_boss_result.secondary_5",
        rva: 0x0608_26A8,
    },
    VtableTarget {
        label: "endless_record_info.primary",
        rva: 0x0608_4028,
    },
    VtableTarget {
        label: "endless_record_info.secondary_1",
        rva: 0x0608_4148,
    },
    VtableTarget {
        label: "endless_record_info.secondary_2",
        rva: 0x0608_4158,
    },
    VtableTarget {
        label: "endless_record_info.secondary_3",
        rva: 0x0608_4168,
    },
    VtableTarget {
        label: "endless_record_info.secondary_4",
        rva: 0x0608_4180,
    },
    VtableTarget {
        label: "endless_record_info.secondary_5",
        rva: 0x0608_4198,
    },
    VtableTarget {
        label: "result_guide.primary",
        rva: 0x0552_E828,
    },
    VtableTarget {
        label: "result_guide.secondary_1",
        rva: 0x0552_E948,
    },
    VtableTarget {
        label: "result_guide.secondary_2",
        rva: 0x0552_E958,
    },
    VtableTarget {
        label: "result_guide.secondary_3",
        rva: 0x0552_E968,
    },
    VtableTarget {
        label: "result_guide.secondary_4",
        rva: 0x0552_E980,
    },
    VtableTarget {
        label: "result_guide.secondary_5",
        rva: 0x0552_E998,
    },
    VtableTarget {
        label: "dialog_reward_result.primary",
        rva: 0x05FB_DB88,
    },
    VtableTarget {
        label: "dialog_reward_result.secondary_1",
        rva: 0x05FB_DD00,
    },
    VtableTarget {
        label: "dialog_reward_result.secondary_2",
        rva: 0x05FB_DD10,
    },
    VtableTarget {
        label: "dialog_reward_result.secondary_3",
        rva: 0x05FB_DD28,
    },
    VtableTarget {
        label: "dialog_reward_result.secondary_4",
        rva: 0x05FB_DD40,
    },
    VtableTarget {
        label: "dialog_reward_result.secondary_5",
        rva: 0x05FB_DD58,
    },
    VtableTarget {
        label: "dialog_reward_result.secondary_6",
        rva: 0x05FB_DD78,
    },
    VtableTarget {
        label: "endless_mode_shop.primary",
        rva: 0x05E4_A040,
    },
    VtableTarget {
        label: "endless_tree_dialog.primary",
        rva: 0x0608_2A68,
    },
    VtableTarget {
        label: "endless_shop_dialog.primary",
        rva: 0x0608_4578,
    },
    VtableTarget {
        label: "endless_tree_all_dialog.primary",
        rva: 0x0608_4CC8,
    },
    VtableTarget {
        label: "endless_tree_all_menu.primary",
        rva: 0x0608_7F20,
    },
    VtableTarget {
        label: "endless_shop_top.primary",
        rva: 0x0607_E968,
    },
    VtableTarget {
        label: "endless_buff_acquired.primary",
        rva: 0x0607_BFE8,
    },
];

fn count_vtable_refs(
    bytes: &[u8],
    base_address: usize,
    module_base: usize,
    targets: &[VtableTarget],
) -> Vec<usize> {
    let mut counts = vec![0; targets.len()];
    let first_aligned = (8 - (base_address & 7)) & 7;
    for offset in (first_aligned..bytes.len().saturating_sub(7)).step_by(8) {
        let pointer = usize::from_le_bytes(
            bytes[offset..offset + 8]
                .try_into()
                .expect("eight-byte pointer"),
        );
        for (index, target) in targets.iter().enumerate() {
            if module_base.checked_add(target.rva) == Some(pointer) {
                counts[index] += 1;
                break;
            }
        }
    }
    counts
}

fn find_vtable_object_locations(
    bytes: &[u8],
    base_address: usize,
    module_base: usize,
    targets: &[VtableTarget],
) -> Vec<(usize, usize)> {
    let pointers = targets
        .iter()
        .enumerate()
        .filter_map(|(index, target)| {
            module_base
                .checked_add(target.rva)
                .map(|pointer| (pointer, index))
        })
        .collect::<HashMap<_, _>>();
    let first_aligned = (8 - (base_address & 7)) & 7;
    let mut locations = Vec::new();
    for offset in (first_aligned..bytes.len().saturating_sub(7)).step_by(8) {
        let pointer = usize::from_le_bytes(
            bytes[offset..offset + 8]
                .try_into()
                .expect("eight-byte pointer"),
        );
        if let Some(index) = pointers.get(&pointer) {
            if let Some(address) = base_address.checked_add(offset) {
                locations.push((*index, address));
            }
        }
    }
    locations.sort_unstable();
    locations
}

fn fingerprint_vtable_objects(
    bytes: &[u8],
    base_address: usize,
    module_base: usize,
    targets: &[VtableTarget],
    object_bytes: usize,
) -> Vec<Vec<String>> {
    let mut fingerprints = vec![Vec::new(); targets.len()];
    let first_aligned = (8 - (base_address & 7)) & 7;
    for offset in (first_aligned..bytes.len().saturating_sub(7)).step_by(8) {
        let Some(object) = bytes.get(offset..offset.saturating_add(object_bytes)) else {
            continue;
        };
        let pointer = usize::from_le_bytes(
            object[..8]
                .try_into()
                .expect("fingerprinted object starts with a pointer"),
        );
        for (index, target) in targets.iter().enumerate() {
            if module_base.checked_add(target.rva) == Some(pointer) {
                let digest = Sha256::digest(object);
                fingerprints[index].push(
                    digest[..8]
                        .iter()
                        .map(|byte| format!("{byte:02X}"))
                        .collect(),
                );
                break;
            }
        }
    }
    fingerprints
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimerFieldKind {
    F32,
    F64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TimerField {
    offset: usize,
    kind: TimerFieldKind,
    before: f64,
    after: f64,
}

fn is_plausible_countdown(before: f64, after: f64, elapsed_seconds: f64) -> bool {
    if !before.is_finite()
        || !after.is_finite()
        || !elapsed_seconds.is_finite()
        || elapsed_seconds <= 0.0
        || !(0.0..=65.0).contains(&before)
        || !(0.0..=65.0).contains(&after)
    {
        return false;
    }
    let decrease = before - after;
    decrease >= elapsed_seconds * 0.25 && decrease <= elapsed_seconds * 4.0
}

fn find_decreasing_timer_fields(
    before: &[u8],
    after: &[u8],
    elapsed_seconds: f64,
) -> Vec<TimerField> {
    let shared_len = before.len().min(after.len());
    let mut fields = Vec::new();

    for offset in (0..shared_len.saturating_sub(3)).step_by(4) {
        let first = f32::from_le_bytes(
            before[offset..offset + 4]
                .try_into()
                .expect("bounded f32 bytes"),
        ) as f64;
        let second = f32::from_le_bytes(
            after[offset..offset + 4]
                .try_into()
                .expect("bounded f32 bytes"),
        ) as f64;
        if is_plausible_countdown(first, second, elapsed_seconds) {
            fields.push(TimerField {
                offset,
                kind: TimerFieldKind::F32,
                before: first,
                after: second,
            });
        }
    }

    for offset in (0..shared_len.saturating_sub(7)).step_by(8) {
        let first = f64::from_le_bytes(
            before[offset..offset + 8]
                .try_into()
                .expect("bounded f64 bytes"),
        );
        let second = f64::from_le_bytes(
            after[offset..offset + 8]
                .try_into()
                .expect("bounded f64 bytes"),
        );
        if is_plausible_countdown(first, second, elapsed_seconds) {
            fields.push(TimerField {
                offset,
                kind: TimerFieldKind::F64,
                before: first,
                after: second,
            });
        }
    }

    fields.sort_by_key(|field| field.offset);
    fields
}

#[derive(Debug, Clone, PartialEq)]
struct TimerManagerSnapshot {
    notice_threshold_seconds: f32,
    defaults: [f32; TIMER_DEFAULT_COUNT],
    mode: u32,
    flags: [u8; 4],
    initial_seconds: f32,
    current_seconds: f32,
    notice_seconds: f32,
}

fn parse_timer_manager(bytes: &[u8]) -> Option<TimerManagerSnapshot> {
    if bytes.len() < TIMER_MANAGER_BYTES {
        return None;
    }
    let notice_threshold_seconds = f32::from_le_bytes(
        bytes
            .get(TIMER_NOTICE_THRESHOLD_OFFSET..TIMER_NOTICE_THRESHOLD_OFFSET + 4)?
            .try_into()
            .ok()?,
    );
    let mut defaults = [0.0; TIMER_DEFAULT_COUNT];
    for (index, value) in defaults.iter_mut().enumerate() {
        let offset = TIMER_DEFAULTS_OFFSET + index * 4;
        *value = f32::from_le_bytes(bytes.get(offset..offset + 4)?.try_into().ok()?);
    }
    Some(TimerManagerSnapshot {
        notice_threshold_seconds,
        defaults,
        mode: u32::from_le_bytes(
            bytes
                .get(TIMER_MODE_OFFSET..TIMER_MODE_OFFSET + 4)?
                .try_into()
                .ok()?,
        ),
        flags: bytes
            .get(TIMER_FLAGS_OFFSET..TIMER_FLAGS_OFFSET + 4)?
            .try_into()
            .ok()?,
        initial_seconds: f32::from_le_bytes(
            bytes
                .get(TIMER_INITIAL_OFFSET..TIMER_INITIAL_OFFSET + 4)?
                .try_into()
                .ok()?,
        ),
        current_seconds: f32::from_le_bytes(
            bytes
                .get(TIMER_CURRENT_OFFSET..TIMER_CURRENT_OFFSET + 4)?
                .try_into()
                .ok()?,
        ),
        notice_seconds: f32::from_le_bytes(
            bytes
                .get(TIMER_NOTICE_OFFSET..TIMER_NOTICE_OFFSET + 4)?
                .try_into()
                .ok()?,
        ),
    })
}

fn has_original_timer_config(snapshot: &TimerManagerSnapshot) -> bool {
    snapshot.notice_threshold_seconds == ORIGINAL_TIMER_CONFIG[0]
        && snapshot.defaults == ORIGINAL_TIMER_CONFIG[1..]
}

fn has_known_timer_config(snapshot: &TimerManagerSnapshot) -> bool {
    has_original_timer_config(snapshot)
        || (snapshot.notice_threshold_seconds == FAST_TIMER_CONFIG[0]
            && snapshot.defaults == FAST_TIMER_CONFIG[1..])
}

fn encode_timer_config(
    config: [f32; TIMER_DEFAULT_COUNT + 1],
) -> [u8; (TIMER_DEFAULT_COUNT + 1) * 4] {
    let mut bytes = [0u8; (TIMER_DEFAULT_COUNT + 1) * 4];
    for (index, value) in config.into_iter().enumerate() {
        bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn encode_original_timer_config() -> [u8; (TIMER_DEFAULT_COUNT + 1) * 4] {
    encode_timer_config(ORIGINAL_TIMER_CONFIG)
}

fn find_pattern_offsets(bytes: &[u8], pattern: &[u8]) -> Vec<usize> {
    if pattern.is_empty() || bytes.len() < pattern.len() {
        return Vec::new();
    }
    bytes
        .windows(pattern.len())
        .enumerate()
        .filter_map(|(offset, window)| (window == pattern).then_some(offset))
        .collect()
}

fn read_timer_manager_at(
    process: &RemoteProcess,
    manager_address: usize,
) -> anyhow::Result<TimerManagerSnapshot> {
    let mut manager_bytes = vec![0u8; TIMER_MANAGER_BYTES];
    process.read_exact(manager_address, &mut manager_bytes)?;
    parse_timer_manager(&manager_bytes)
        .ok_or_else(|| anyhow::anyhow!("timer manager snapshot is truncated"))
}

fn locate_timer_manager(
    process: &RemoteProcess,
    regions: &[memory::MemoryRegion],
) -> anyhow::Result<(usize, Vec<usize>)> {
    let configs = [
        encode_original_timer_config(),
        encode_timer_config(FAST_TIMER_CONFIG),
    ];
    let overlap = configs[0].len() - 1;
    let mut managers = Vec::new();

    for region in regions {
        let mut region_offset = 0usize;
        while region_offset < region.size {
            let remaining = region.size - region_offset;
            let advance = remaining.min(SCAN_CHUNK_BYTES);
            let read_len = remaining.min(SCAN_CHUNK_BYTES + overlap);
            let Some(address) = region.base_address.checked_add(region_offset) else {
                break;
            };
            let mut bytes = vec![0u8; read_len];
            if process.read_exact(address, &mut bytes).is_ok() {
                for config in &configs {
                    for offset in find_pattern_offsets(&bytes, config) {
                        let Some(config_address) = address.checked_add(offset) else {
                            continue;
                        };
                        let Some(manager_address) =
                            config_address.checked_sub(TIMER_NOTICE_THRESHOLD_OFFSET)
                        else {
                            continue;
                        };
                        if read_timer_manager_at(process, manager_address)
                            .is_ok_and(|snapshot| has_known_timer_config(&snapshot))
                        {
                            managers.push(manager_address);
                        }
                    }
                }
            }
            region_offset += advance;
        }
    }
    managers.sort_unstable();
    managers.dedup();
    if managers.len() != 1 {
        anyhow::bail!(
            "timer manager candidate count was {}; expected one",
            managers.len()
        );
    }

    let manager_address = managers[0];
    let pointer = manager_address.to_le_bytes();
    let mut pointer_rvas = Vec::new();
    let mut module_offset = 0usize;
    while module_offset < process.module_size {
        let remaining = process.module_size - module_offset;
        let advance = remaining.min(MODULE_SCAN_CHUNK_BYTES);
        let read_len = remaining.min(MODULE_SCAN_CHUNK_BYTES + pointer.len() - 1);
        let Some(address) = process.module_base.checked_add(module_offset) else {
            break;
        };
        let mut bytes = vec![0u8; read_len];
        if process.read_exact(address, &mut bytes).is_ok() {
            for offset in find_pattern_offsets(&bytes, &pointer) {
                let Some(rva) = module_offset.checked_add(offset) else {
                    continue;
                };
                if rva % std::mem::size_of::<usize>() == 0 {
                    pointer_rvas.push(rva);
                }
            }
        }
        module_offset += advance;
    }
    pointer_rvas.sort_unstable();
    pointer_rvas.dedup();
    Ok((manager_address, pointer_rvas))
}

fn format_sha256(hash: &[u8; 32]) -> String {
    hash.iter().map(|byte| format!("{byte:02X}")).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FunctionMetadataError {
    UnreadableSlot,
    OutsideImage,
    OutsideText,
    SignatureCount(usize),
}

fn validate_function_metadata(
    image_len: usize,
    text_rvas: Range<usize>,
    function_rva: Option<usize>,
    signature_matches: usize,
) -> Result<usize, FunctionMetadataError> {
    let function_rva = function_rva.ok_or(FunctionMetadataError::UnreadableSlot)?;
    if function_rva >= image_len {
        return Err(FunctionMetadataError::OutsideImage);
    }
    if !text_rvas.contains(&function_rva) {
        return Err(FunctionMetadataError::OutsideText);
    }
    if signature_matches != 1 {
        return Err(FunctionMetadataError::SignatureCount(signature_matches));
    }
    Ok(function_rva)
}

fn run_timer_probe(
    process: &RemoteProcess,
    regions: &[memory::MemoryRegion],
) -> anyhow::Result<()> {
    let (manager_address, pointer_rvas) = locate_timer_manager(process, regions)?;
    let formatted_pointer_rvas = pointer_rvas
        .iter()
        .map(|rva| format!("0x{rva:08X}"))
        .collect::<Vec<_>>()
        .join(",");
    println!(
        "CONFLUX TIMER PROBE locator manager_candidates=1 pointer_rvas=[{formatted_pointer_rvas}]"
    );

    let manager_before = read_timer_manager_at(process, manager_address)?;
    let manager_started = Instant::now();
    thread::sleep(TIMER_SAMPLE_DELAY);
    let manager_elapsed = manager_started.elapsed().as_secs_f32();
    let manager_after = read_timer_manager_at(process, manager_address)?;
    println!(
        "CONFLUX TIMER PROBE manager mode={} threshold={:.3} flags={:02X?}->{:02X?} initial={:.3}->{:.3} current={:.3}->{:.3} notice={:.3}->{:.3} elapsed={manager_elapsed:.3}s",
        manager_before.mode,
        manager_before.notice_threshold_seconds,
        manager_before.flags,
        manager_after.flags,
        manager_before.initial_seconds,
        manager_after.initial_seconds,
        manager_before.current_seconds,
        manager_after.current_seconds,
        manager_before.notice_seconds,
        manager_after.notice_seconds,
    );
    println!("CONFLUX TIMER PROBE defaults {:?}", manager_before.defaults);

    let mut locations = Vec::new();
    let mut skipped_chunks = 0usize;
    for region in regions {
        let mut region_offset = 0usize;
        while region_offset < region.size {
            let read_len = (region.size - region_offset).min(SCAN_CHUNK_BYTES);
            let Some(address) = region.base_address.checked_add(region_offset) else {
                skipped_chunks += 1;
                break;
            };
            let mut bytes = vec![0u8; read_len];
            match process.read_exact(address, &mut bytes) {
                Ok(()) => locations.extend(find_vtable_object_locations(
                    &bytes,
                    address,
                    process.module_base,
                    TARGETS,
                )),
                Err(_) => skipped_chunks += 1,
            }
            region_offset += read_len;
        }
    }
    locations.sort_unstable();
    locations.dedup();

    let mut before = Vec::new();
    for (target_index, address) in locations {
        let mut bytes = vec![0u8; TIMER_OBJECT_BYTES];
        if process.read_exact(address, &mut bytes).is_ok() {
            before.push((target_index, address, bytes));
        }
    }

    let started = Instant::now();
    thread::sleep(TIMER_SAMPLE_DELAY);
    let elapsed = started.elapsed().as_secs_f64();
    let mut ordinals = vec![0usize; TARGETS.len()];
    let mut candidate_count = 0usize;
    for (target_index, address, first) in before {
        let ordinal = ordinals[target_index];
        ordinals[target_index] += 1;
        let mut second = vec![0u8; TIMER_OBJECT_BYTES];
        if process.read_exact(address, &mut second).is_err() {
            continue;
        }
        for field in find_decreasing_timer_fields(&first, &second, elapsed) {
            candidate_count += 1;
            println!(
                "CONFLUX TIMER PROBE candidate target={} ordinal={} offset=0x{:X} kind={:?} before={:.3} after={:.3}",
                TARGETS[target_index].label,
                ordinal,
                field.offset,
                field.kind,
                field.before,
                field.after
            );
        }
    }
    println!(
        "CONFLUX TIMER PROBE summary sampled_objects={} candidates={} elapsed={elapsed:.3}s skipped_chunks={skipped_chunks}",
        ordinals.iter().sum::<usize>(),
        candidate_count
    );
    Ok(())
}


fn main() -> anyhow::Result<()> {
    if !cfg!(debug_assertions) {
        anyhow::bail!("conflux UI probe is available only in debug builds");
    }
    let ui_probe = env::var(PROBE_ENV).ok().as_deref() == Some("1");
    let timer_probe = env::var(TIMER_PROBE_ENV).ok().as_deref() == Some("1");
    if !ui_probe && !timer_probe {
        anyhow::bail!(
            "set {PROBE_ENV}=1 or {TIMER_PROBE_ENV}=1 to opt in to a read-only debug probe"
        );
    }

    let process = RemoteProcess::find(GAME_PROCESS_NAME)?
        .ok_or_else(|| anyhow::anyhow!("game not running"))?;
    let executable_hash = format_sha256(&process.executable_sha256()?);
    if executable_hash != PINNED_GAME_SHA256 {
        anyhow::bail!(
            "unsupported executable hash: expected {PINNED_GAME_SHA256}, got {executable_hash}"
        );
    }

    let regions = process.readable_private_regions()?;
    if timer_probe {
        println!(
            "CONFLUX TIMER PROBE process pid={} sha256={} rights=PROCESS_QUERY_INFORMATION|PROCESS_VM_READ",
            process.pid, executable_hash
        );
        return run_timer_probe(&process, &regions);
    }
    let mut counts = vec![0usize; TARGETS.len()];
    let mut fingerprints = vec![Vec::<String>::new(); TARGETS.len()];
    let mut scanned_bytes = 0usize;
    let mut skipped_chunks = 0usize;

    for region in &regions {
        let mut region_offset = 0usize;
        while region_offset < region.size {
            let read_len = (region.size - region_offset).min(SCAN_CHUNK_BYTES);
            let Some(address) = region.base_address.checked_add(region_offset) else {
                skipped_chunks += 1;
                break;
            };
            let mut bytes = vec![0u8; read_len];
            match process.read_exact(address, &mut bytes) {
                Ok(()) => {
                    let chunk_counts =
                        count_vtable_refs(&bytes, address, process.module_base, TARGETS);
                    for (total, chunk) in counts.iter_mut().zip(chunk_counts) {
                        *total += chunk;
                    }
                    let chunk_fingerprints = fingerprint_vtable_objects(
                        &bytes,
                        address,
                        process.module_base,
                        TARGETS,
                        OBJECT_FINGERPRINT_BYTES,
                    );
                    for (total, chunk) in fingerprints.iter_mut().zip(chunk_fingerprints) {
                        let remaining = MAX_FINGERPRINTS_PER_TARGET.saturating_sub(total.len());
                        total.extend(chunk.into_iter().take(remaining));
                    }
                    scanned_bytes += read_len;
                }
                Err(_) => skipped_chunks += 1,
            }
            region_offset += read_len;
        }
    }

    println!(
        "CONFLUX UI PROBE process pid={} sha256={} rights=PROCESS_QUERY_INFORMATION|PROCESS_VM_READ",
        process.pid, executable_hash
    );
    println!(
        "CONFLUX UI PROBE snapshot readable_regions={} scanned_bytes={} skipped_chunks={}",
        regions.len(),
        scanned_bytes,
        skipped_chunks
    );
    for ((target, count), fingerprints) in TARGETS.iter().zip(counts).zip(fingerprints) {
        let fingerprints = if fingerprints.is_empty() {
            "-".to_owned()
        } else {
            fingerprints.join(",")
        };
        println!(
            "CONFLUX UI PROBE count {}={count} fingerprints={fingerprints}",
            target.label
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        count_vtable_refs, encode_original_timer_config, encode_timer_config,
        find_decreasing_timer_fields, find_pattern_offsets, find_vtable_object_locations,
        fingerprint_vtable_objects, has_known_timer_config, has_original_timer_config,
        parse_timer_manager, validate_function_metadata, FunctionMetadataError, TimerFieldKind,
        VtableTarget, FAST_TIMER_CONFIG, TIMER_CURRENT_OFFSET, TIMER_DEFAULTS_OFFSET,
        TIMER_INITIAL_OFFSET, TIMER_MANAGER_BYTES, TIMER_MODE_OFFSET, TIMER_NOTICE_OFFSET,
        TIMER_NOTICE_THRESHOLD_OFFSET,
    };

    const TARGETS: [VtableTarget; 2] = [
        VtableTarget {
            label: "reward",
            rva: 0x100,
        },
        VtableTarget {
            label: "difficulty",
            rva: 0x200,
        },
    ];

    #[test]
    fn counts_only_aligned_known_vtable_pointers() {
        let module_base = 0x1400_0000_0usize;
        let mut bytes = vec![0u8; 40];
        bytes[0..8].copy_from_slice(&(module_base + 0x100).to_le_bytes());
        bytes[8..16].copy_from_slice(&(module_base + 0x200).to_le_bytes());
        bytes[16..24].copy_from_slice(&(module_base + 0x100).to_le_bytes());
        bytes[24..32].copy_from_slice(&(module_base + 0x999).to_le_bytes());
        bytes[33..40].copy_from_slice(&(module_base + 0x100).to_le_bytes()[..7]);

        assert_eq!(
            count_vtable_refs(&bytes, 0, module_base, &TARGETS),
            vec![2, 1]
        );
    }

    #[test]
    fn preserves_alignment_across_chunk_offsets() {
        let module_base = 0x1400_0000_0usize;
        let pointer = (module_base + 0x100).to_le_bytes();
        let mut bytes = vec![0u8; 17];
        bytes[7..15].copy_from_slice(&pointer);

        assert_eq!(
            count_vtable_refs(&bytes, 1, module_base, &TARGETS),
            vec![1, 0]
        );
        assert_eq!(
            count_vtable_refs(&bytes, 0, module_base, &TARGETS),
            vec![0, 0]
        );
    }

    #[test]
    fn locates_known_vtables_in_one_aligned_pass() {
        let module_base = 0x1400_0000_0usize;
        let mut bytes = vec![0u8; 32];
        bytes[8..16].copy_from_slice(&(module_base + 0x200).to_le_bytes());
        bytes[24..32].copy_from_slice(&(module_base + 0x100).to_le_bytes());

        assert_eq!(
            find_vtable_object_locations(&bytes, 0x1008, module_base, &TARGETS),
            vec![(0, 0x1020), (1, 0x1010)]
        );
    }

    #[test]
    fn fingerprints_bounded_object_bytes_without_exposing_them() {
        let module_base = 0x1400_0000_0usize;
        let mut bytes = (0u8..64).collect::<Vec<_>>();
        bytes[0..8].copy_from_slice(&(module_base + 0x100).to_le_bytes());

        assert_eq!(
            fingerprint_vtable_objects(&bytes, 0, module_base, &TARGETS, 64),
            vec![vec!["41F935D6E064DF11".to_owned()], Vec::new()]
        );
        assert_eq!(
            fingerprint_vtable_objects(&bytes[..63], 0, module_base, &TARGETS, 64),
            vec![Vec::<String>::new(), Vec::new()]
        );
    }

    #[test]
    fn accepts_one_readable_function_inside_text() {
        assert_eq!(
            validate_function_metadata(0x6000, 0x1000..0x5000, Some(0x2340), 1),
            Ok(0x2340)
        );
    }

    #[test]
    fn rejects_unreadable_vtable_slot() {
        assert_eq!(
            validate_function_metadata(0x6000, 0x1000..0x5000, None, 1),
            Err(FunctionMetadataError::UnreadableSlot)
        );
    }

    #[test]
    fn rejects_function_outside_text_or_image() {
        assert_eq!(
            validate_function_metadata(0x6000, 0x1000..0x5000, Some(0x5000), 1),
            Err(FunctionMetadataError::OutsideText)
        );
        assert_eq!(
            validate_function_metadata(0x4000, 0x1000..0x5000, Some(0x4500), 1),
            Err(FunctionMetadataError::OutsideImage)
        );
    }

    #[test]
    fn rejects_missing_or_duplicate_function_signatures() {
        assert_eq!(
            validate_function_metadata(0x6000, 0x1000..0x5000, Some(0x2340), 0),
            Err(FunctionMetadataError::SignatureCount(0))
        );
        assert_eq!(
            validate_function_metadata(0x6000, 0x1000..0x5000, Some(0x2340), 2),
            Err(FunctionMetadataError::SignatureCount(2))
        );
    }

    #[test]
    fn finds_aligned_f32_and_f64_countdowns_without_exposing_object_addresses() {
        let mut before = vec![0u8; 40];
        let mut after = before.clone();
        before[4..8].copy_from_slice(&60.0f32.to_le_bytes());
        after[4..8].copy_from_slice(&59.5f32.to_le_bytes());
        before[16..24].copy_from_slice(&60.0f64.to_le_bytes());
        after[16..24].copy_from_slice(&59.5f64.to_le_bytes());

        let fields = find_decreasing_timer_fields(&before, &after, 0.5);

        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].offset, 4);
        assert_eq!(fields[0].kind, TimerFieldKind::F32);
        assert_eq!(fields[1].offset, 16);
        assert_eq!(fields[1].kind, TimerFieldKind::F64);
    }

    #[test]
    fn timer_filter_rejects_growth_non_finite_values_and_implausible_rates() {
        let mut before = vec![0u8; 32];
        let mut after = before.clone();
        before[0..4].copy_from_slice(&10.0f32.to_le_bytes());
        after[0..4].copy_from_slice(&10.5f32.to_le_bytes());
        before[4..8].copy_from_slice(&f32::NAN.to_le_bytes());
        after[4..8].copy_from_slice(&9.5f32.to_le_bytes());
        before[8..12].copy_from_slice(&60.0f32.to_le_bytes());
        after[8..12].copy_from_slice(&40.0f32.to_le_bytes());

        assert!(find_decreasing_timer_fields(&before, &after, 0.5).is_empty());
    }

    #[test]
    fn parses_version_pinned_timer_manager_fields() {
        let mut bytes = vec![0u8; TIMER_MANAGER_BYTES];
        for index in 0..11 {
            bytes[TIMER_DEFAULTS_OFFSET + index * 4..TIMER_DEFAULTS_OFFSET + index * 4 + 4]
                .copy_from_slice(&(60.0 + index as f32).to_le_bytes());
        }
        bytes[TIMER_NOTICE_THRESHOLD_OFFSET..TIMER_NOTICE_THRESHOLD_OFFSET + 4]
            .copy_from_slice(&10.0f32.to_le_bytes());
        bytes[TIMER_MODE_OFFSET..TIMER_MODE_OFFSET + 4].copy_from_slice(&1u32.to_le_bytes());
        bytes[TIMER_INITIAL_OFFSET..TIMER_INITIAL_OFFSET + 4]
            .copy_from_slice(&60.0f32.to_le_bytes());
        bytes[TIMER_CURRENT_OFFSET..TIMER_CURRENT_OFFSET + 4]
            .copy_from_slice(&59.5f32.to_le_bytes());
        bytes[TIMER_NOTICE_OFFSET..TIMER_NOTICE_OFFSET + 4].copy_from_slice(&1.0f32.to_le_bytes());

        let snapshot = parse_timer_manager(&bytes).expect("complete manager");

        assert_eq!(snapshot.defaults[0], 60.0);
        assert_eq!(snapshot.defaults[10], 70.0);
        assert_eq!(snapshot.notice_threshold_seconds, 10.0);
        assert_eq!(snapshot.mode, 1);
        assert_eq!(snapshot.initial_seconds, 60.0);
        assert_eq!(snapshot.current_seconds, 59.5);
        assert_eq!(snapshot.notice_seconds, 1.0);
        assert!(parse_timer_manager(&bytes[..TIMER_MANAGER_BYTES - 1]).is_none());
    }

    #[test]
    fn finds_original_timer_config_without_exposing_manager_addresses() {
        let pattern = encode_original_timer_config();
        let mut bytes = vec![0xCC; pattern.len() * 2 + 3];
        bytes[1..1 + pattern.len()].copy_from_slice(&pattern);
        let second = pattern.len() + 3;
        bytes[second..second + pattern.len()].copy_from_slice(&pattern);

        assert_eq!(find_pattern_offsets(&bytes, &pattern), vec![1, second]);

        let mut manager = vec![0u8; TIMER_MANAGER_BYTES];
        manager[TIMER_NOTICE_THRESHOLD_OFFSET
            ..TIMER_NOTICE_THRESHOLD_OFFSET + pattern.len()]
            .copy_from_slice(&pattern);
        let snapshot = parse_timer_manager(&manager).expect("complete manager");
        assert!(has_original_timer_config(&snapshot));

        let fast = encode_timer_config(FAST_TIMER_CONFIG);
        manager[TIMER_NOTICE_THRESHOLD_OFFSET
            ..TIMER_NOTICE_THRESHOLD_OFFSET + fast.len()]
            .copy_from_slice(&fast);
        let fast_snapshot = parse_timer_manager(&manager).expect("complete patched manager");
        assert!(has_known_timer_config(&fast_snapshot));
    }
}

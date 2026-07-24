#[path = "../src/equipment_probe/locator.rs"]
mod locator;
#[path = "../src/equipment_probe/memory.rs"]
mod memory;

#[cfg(test)]
mod equipment_probe {
    pub(crate) mod memory {
        pub(crate) use crate::memory::*;
    }
}

#[cfg(not(windows))]
compile_error!("probe_item_inventory is Windows-only");

use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use aho_corasick::{AhoCorasickBuilder, AhoCorasickKind};
use equipment_core::decode_item_inventory;
use locator::{locate_from_globals, resolve_roots};
use memory::{MemoryReader, MemoryRegion, RemoteProcess};

const GAME_PROCESS_NAME: &str = "granblue_fantasy_relink.exe";
const PINNED_GAME_SHA256: &str = "63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F";
const SCAN_CHUNK_BYTES: usize = 8 * 1024 * 1024;
const OVERLAP_BYTES: usize = 0x80;
const MAX_RELATIVE_OFFSET: isize = 0x40;
const MAX_REPORTED_GROUPS: usize = 20;
const MAX_REPORTED_SAMPLES: usize = 3;
const ITEM_RECORD_STRIDE: usize = 0x30;
const MAX_CONTROL_SPAN_RECORDS: usize = 400;
const CONTROL_ITEMS: &[(u32, u32)] = &[
    (0x687733c4, 3),
    (0x24df54ee, 21),
    (0x8816adc5, 709),
    (0x4325ab79, 627),
    (0x7cfa41fc, 419),
    (0x23f2e797, 754),
    (0x9738c87a, 567),
    (0x6f1fabe6, 580),
    (0x2e94d39a, 918),
    (0x541d3800, 6),
    (0x99d6247e, 665),
    (0x24725515, 441),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GroupKey {
    region_index: usize,
    quantity_offset: isize,
}

#[derive(Debug, Default)]
struct CandidateGroup {
    item_ids: HashSet<u32>,
    quantities: HashSet<u32>,
    samples: Vec<(u32, u32)>,
    high_samples: Vec<(u32, u32)>,
    records: Vec<(usize, u32, u32)>,
    occurrences: usize,
}

impl CandidateGroup {
    fn record(&mut self, item_id: u32, quantity: u32, item_address: usize) {
        self.item_ids.insert(item_id);
        self.quantities.insert(quantity);
        self.records.push((item_address, item_id, quantity));
        self.occurrences += 1;
        if self.samples.len() < MAX_REPORTED_SAMPLES
            && !self.samples.iter().any(|sample| sample.0 == item_id)
        {
            self.samples.push((item_id, quantity));
        }
        if quantity >= 900
            && self.high_samples.len() < MAX_REPORTED_SAMPLES
            && !self.high_samples.iter().any(|sample| sample.0 == item_id)
        {
            self.high_samples.push((item_id, quantity));
        }
    }

    fn score(&self) -> usize {
        self.item_ids.len() * 4 + self.quantities.len()
    }

    fn common_stride(&self) -> Option<(usize, usize)> {
        let mut positions = self
            .records
            .iter()
            .map(|record| record.0)
            .collect::<Vec<_>>();
        positions.sort_unstable();
        positions.dedup();
        let mut strides = HashMap::<usize, usize>::new();
        for pair in positions.windows(2) {
            let stride = pair[1] - pair[0];
            if stride <= 0x200 {
                *strides.entry(stride).or_default() += 1;
            }
        }
        strides
            .into_iter()
            .max_by_key(|(stride, count)| (*count, std::cmp::Reverse(*stride)))
    }

    fn longest_run(&self, stride: usize) -> (usize, Vec<(u32, u32)>) {
        let mut records = self.records.clone();
        records.sort_unstable_by_key(|record| record.0);
        records.dedup_by_key(|record| record.0);
        let mut best_start = 0usize;
        let mut best_len = usize::from(!records.is_empty());
        let mut current_start = 0usize;
        for index in 1..records.len() {
            if records[index].0 - records[index - 1].0 != stride {
                current_start = index;
            }
            let current_len = index - current_start + 1;
            if current_len > best_len {
                best_start = current_start;
                best_len = current_len;
            }
        }
        let samples = records
            .get(best_start..best_start.saturating_add(best_len))
            .unwrap_or_default()
            .iter()
            .take(MAX_REPORTED_SAMPLES)
            .map(|(_, item_id, quantity)| (*item_id, *quantity))
            .collect();
        (best_len, samples)
    }

    fn best_control_cluster(&self) -> (usize, usize, Option<usize>, Vec<(u32, u32)>) {
        let mut matches = self
            .records
            .iter()
            .filter(|(_, item_id, quantity)| CONTROL_ITEMS.contains(&(*item_id, *quantity)))
            .copied()
            .collect::<Vec<_>>();
        matches.sort_unstable_by_key(|record| record.0);
        matches.dedup();

        let mut best = Vec::new();
        let max_span = ITEM_RECORD_STRIDE * MAX_CONTROL_SPAN_RECORDS;
        for (start_index, start) in matches.iter().enumerate() {
            let cluster = matches[start_index..]
                .iter()
                .take_while(|candidate| candidate.0 - start.0 <= max_span)
                .filter(|candidate| (candidate.0 - start.0) % ITEM_RECORD_STRIDE == 0)
                .copied()
                .collect::<Vec<_>>();
            let distinct_items = cluster
                .iter()
                .map(|(_, item_id, _)| *item_id)
                .collect::<HashSet<_>>()
                .len();
            let best_distinct_items = best
                .iter()
                .map(|(_, item_id, _)| *item_id)
                .collect::<HashSet<_>>()
                .len();
            if distinct_items > best_distinct_items {
                best = cluster;
            }
        }

        let distinct_items = best
            .iter()
            .map(|(_, item_id, _)| *item_id)
            .collect::<HashSet<_>>()
            .len();
        let span_records = best
            .first()
            .zip(best.last())
            .map(|(first, last)| (last.0 - first.0) / ITEM_RECORD_STRIDE + 1)
            .unwrap_or_default();
        let samples = best
            .iter()
            .map(|(_, item_id, quantity)| (*item_id, *quantity))
            .collect();
        let start_address = best.first().map(|record| record.0);
        (distinct_items, span_records, start_address, samples)
    }

    fn controls_near_anchor(&self) -> Vec<(u32, u32)> {
        let anchors = self
            .records
            .iter()
            .filter(|(_, item_id, quantity)| (*item_id, *quantity) == (0x2e94d39a, 918))
            .collect::<Vec<_>>();
        let max_span = ITEM_RECORD_STRIDE * MAX_CONTROL_SPAN_RECORDS;
        let mut best = Vec::new();

        for anchor in anchors {
            let mut controls = Vec::new();
            for (control_id, _) in CONTROL_ITEMS {
                let nearest = self
                    .records
                    .iter()
                    .filter(|candidate| {
                        candidate.1 == *control_id
                            && candidate.0.abs_diff(anchor.0) <= max_span
                            && candidate.0.abs_diff(anchor.0) % ITEM_RECORD_STRIDE == 0
                    })
                    .min_by_key(|candidate| candidate.0.abs_diff(anchor.0));
                if let Some((_, item_id, quantity)) = nearest {
                    controls.push((*item_id, *quantity));
                }
            }
            if controls.len() > best.len() {
                best = controls;
            }
        }
        best
    }

    fn exact_control_counts(&self) -> Vec<(u32, usize)> {
        CONTROL_ITEMS
            .iter()
            .map(|(control_id, expected_quantity)| {
                let count = self
                    .records
                    .iter()
                    .filter(|(_, item_id, quantity)| {
                        item_id == control_id && quantity == expected_quantity
                    })
                    .count();
                (*control_id, count)
            })
            .collect()
    }
}

fn summarize_control_layout(
    process: &RemoteProcess,
    group: &CandidateGroup,
) -> anyhow::Result<String> {
    let records = group
        .records
        .iter()
        .filter(|(_, item_id, quantity)| CONTROL_ITEMS.contains(&(*item_id, *quantity)))
        .collect::<Vec<_>>();
    let mut fields = (0..ITEM_RECORD_STRIDE)
        .step_by(4)
        .map(|offset| (offset, Vec::new()))
        .collect::<Vec<_>>();
    for (address, _, _) in records {
        let mut bytes = [0u8; ITEM_RECORD_STRIDE];
        process.read_exact(*address, &mut bytes)?;
        for (offset, values) in &mut fields {
            values.push(u32::from_le_bytes(
                bytes[*offset..*offset + 4]
                    .try_into()
                    .expect("four-byte record field"),
            ));
        }
    }
    Ok(fields
        .into_iter()
        .map(|(offset, values)| {
            let distinct = values.iter().copied().collect::<HashSet<_>>().len();
            let zeros = values.iter().filter(|value| **value == 0).count();
            let constant = (distinct == 1)
                .then(|| format!(",constant={:#x}", values[0]))
                .unwrap_or_default();
            format!("+{offset:#x}:distinct={distinct},zeros={zeros}{constant}")
        })
        .collect::<Vec<_>>()
        .join(";"))
}

fn scan_structural_records(
    process: &RemoteProcess,
    region: MemoryRegion,
    known_item_ids: &HashSet<u32>,
) -> anyhow::Result<Vec<(u32, u32, bool)>> {
    let mut records = Vec::new();
    let mut offset = 0usize;
    while offset < region.size {
        let payload_len = (region.size - offset).min(SCAN_CHUNK_BYTES);
        let read_len = (region.size - offset).min(payload_len.saturating_add(ITEM_RECORD_STRIDE));
        let mut bytes = vec![0; read_len];
        process.read_exact(region.base_address + offset, &mut bytes)?;
        for record_offset in (0..payload_len).step_by(4) {
            let Some(record) = bytes.get(record_offset..record_offset + ITEM_RECORD_STRIDE) else {
                continue;
            };
            if u32::from_le_bytes(record[0x08..0x0c].try_into()?) != 0x0c
                || u32::from_le_bytes(record[0x10..0x14].try_into()?) != 0
                || [0x14, 0x18, 0x1c].into_iter().any(|field_offset| {
                    u32::from_le_bytes(
                        record[field_offset..field_offset + 4]
                            .try_into()
                            .expect("four-byte structural field"),
                    ) != u32::MAX
                })
            {
                continue;
            }
            let item_id = u32::from_le_bytes(record[0x00..0x04].try_into()?);
            let quantity = u32::from_le_bytes(record[0x04..0x08].try_into()?);
            if quantity <= 999 {
                records.push((item_id, quantity, known_item_ids.contains(&item_id)));
            }
        }
        offset += payload_len;
    }
    Ok(records)
}

fn main() -> anyhow::Result<()> {
    let started = Instant::now();
    let process = RemoteProcess::find(GAME_PROCESS_NAME)?
        .ok_or_else(|| anyhow::anyhow!("game not running"))?;
    let executable_hash = process
        .executable_sha256()?
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<String>();
    anyhow::ensure!(
        executable_hash == PINNED_GAME_SHA256,
        "unsupported game executable"
    );
    let (text_address, text) = process.read_text_section()?;
    let text_rva = text_address
        .checked_sub(process.module_base)
        .ok_or_else(|| anyhow::anyhow!("text address precedes module base"))?;
    let roots = resolve_roots(&process, process.module_base, text_rva, &text)?;
    let player = locate_from_globals(&process, roots.local_key_global, roots.manager_global)?;

    let item_ids = parse_item_ids(include_str!("../lang/en/items.json"))?;
    let known_item_ids = item_ids.iter().copied().collect::<HashSet<_>>();
    let patterns = item_ids
        .iter()
        .copied()
        .map(u32::to_le_bytes)
        .collect::<Vec<_>>();
    let matcher = AhoCorasickBuilder::new()
        .kind(Some(AhoCorasickKind::DFA))
        .build(patterns.iter().map(|pattern| pattern.as_slice()))?;
    let pattern_ids = patterns
        .iter()
        .map(|pattern| u32::from_le_bytes(*pattern))
        .collect::<Vec<_>>();

    let regions = process.readable_private_regions()?;
    let mut groups = HashMap::<GroupKey, CandidateGroup>::new();
    let mut requested_bytes = 0u64;
    let mut matched_ids = 0usize;

    for (region_index, region) in regions.iter().copied().enumerate() {
        let mut offset = 0usize;
        let mut carry = Vec::new();
        while offset < region.size {
            let read_len = (region.size - offset).min(SCAN_CHUNK_BYTES);
            let mut chunk = vec![0; read_len];
            let address = region
                .base_address
                .checked_add(offset)
                .ok_or_else(|| anyhow::anyhow!("region address overflow"))?;
            if process.read_exact(address, &mut chunk).is_err() {
                break;
            }
            requested_bytes += read_len as u64;

            let carry_len = carry.len();
            let mut bytes = carry;
            bytes.extend_from_slice(&chunk);
            let combined_base = address.saturating_sub(carry_len);

            for matched in matcher.find_iter(&bytes) {
                let item_address = combined_base
                    .checked_add(matched.start())
                    .ok_or_else(|| anyhow::anyhow!("match address overflow"))?;
                if item_address % std::mem::size_of::<u32>() != 0 {
                    continue;
                }
                let item_id = pattern_ids[matched.pattern().as_usize()];
                matched_ids += 1;

                for quantity_offset in (-MAX_RELATIVE_OFFSET..=MAX_RELATIVE_OFFSET).step_by(4) {
                    if quantity_offset == 0 {
                        continue;
                    }
                    let quantity_start = isize::try_from(matched.start())
                        .ok()
                        .and_then(|start| start.checked_add(quantity_offset))
                        .and_then(|start| usize::try_from(start).ok());
                    let Some(quantity_start) = quantity_start else {
                        continue;
                    };
                    let Some(quantity_end) = quantity_start.checked_add(4) else {
                        continue;
                    };
                    if quantity_end > bytes.len() {
                        continue;
                    }
                    let quantity = u32::from_le_bytes(
                        bytes[quantity_start..quantity_end]
                            .try_into()
                            .expect("four-byte quantity"),
                    );
                    if !(1..=999).contains(&quantity) {
                        continue;
                    }
                    groups
                        .entry(GroupKey {
                            region_index,
                            quantity_offset,
                        })
                        .or_default()
                        .record(item_id, quantity, item_address);
                }
            }

            let carry_start = bytes.len().saturating_sub(OVERLAP_BYTES);
            carry = bytes[carry_start..].to_vec();
            offset += read_len;
        }
    }

    let mut ranked = groups
        .into_iter()
        .filter(|(_, group)| group.item_ids.len() >= 3)
        .collect::<Vec<_>>();
    ranked.sort_unstable_by(|left, right| {
        right
            .1
            .score()
            .cmp(&left.1.score())
            .then_with(|| left.0.region_index.cmp(&right.0.region_index))
            .then_with(|| left.0.quantity_offset.cmp(&right.0.quantity_offset))
    });
    if let Some((_, group)) = ranked.iter().find(|(key, group)| {
        key.quantity_offset == 4
            && regions[key.region_index].size == 243_269_632
            && group
                .exact_control_counts()
                .iter()
                .all(|(_, count)| *count == 1)
    }) {
        println!(
            "ITEM PROBE verified_control_layout records={} fields={}",
            CONTROL_ITEMS.len(),
            summarize_control_layout(&process, group)?
        );
    }
    let structural_regions = regions
        .iter()
        .copied()
        .filter(|region| region.size == 243_269_632)
        .collect::<Vec<_>>();
    if let [region] = structural_regions.as_slice() {
        let mut whole_region = vec![0; region.size];
        let whole_read_status = process
            .read_exact(region.base_address, &mut whole_region)
            .map(|_| "ok".to_string())
            .unwrap_or_else(|error| format!("error:{error}"));
        let ordinary_catalog: serde_json::Value =
            serde_json::from_str(include_str!("../data/ordinary-items-2.0.2.json"))?;
        let ordinary_ids = ordinary_catalog["itemIds"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("ordinary item catalog has no itemIds"))?
            .iter()
            .map(|value| {
                u32::from_str_radix(
                    value
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("ordinary item id is not text"))?,
                    16,
                )
                .map_err(anyhow::Error::from)
            })
            .collect::<Result<HashSet<_>, _>>()?;
        let decoder_status = decode_item_inventory(&whole_region, &ordinary_ids)
            .map(|items| format!("ok:{}", items.len()))
            .unwrap_or_else(|error| format!("error:{error}"));
        drop(whole_region);
        let records = scan_structural_records(&process, *region, &known_item_ids)?;
        let distinct_ids = records
            .iter()
            .map(|(item_id, _, _)| *item_id)
            .collect::<HashSet<_>>();
        let known_count = records.iter().filter(|(_, _, known)| *known).count();
        let unknown_count = records.len() - known_count;
        let duplicate_ids = records.len().saturating_sub(distinct_ids.len());
        let warning_samples = records
            .iter()
            .filter(|(_, quantity, known)| *known && *quantity >= 900)
            .take(MAX_REPORTED_SAMPLES)
            .map(|(item_id, quantity, _)| format!("{item_id:08X}:{quantity}"))
            .collect::<Vec<_>>()
            .join(",");
        let unknown_samples = records
            .iter()
            .filter(|(_, _, known)| !known)
            .take(MAX_REPORTED_SAMPLES)
            .map(|(item_id, quantity, _)| format!("{item_id:08X}:{quantity}"))
            .collect::<Vec<_>>()
            .join(",");
        let mut unknown_ids = records
            .iter()
            .filter(|(_, _, known)| !known)
            .map(|(item_id, _, _)| format!("{item_id:08X}"))
            .collect::<Vec<_>>();
        unknown_ids.sort_unstable();
        println!(
            "ITEM PROBE structural_records whole_read={} decoder={} total={} distinct_ids={} duplicate_ids={} known={} unknown={} warning_samples={} unknown_samples={} unknown_ids={}",
            whole_read_status,
            decoder_status,
            records.len(),
            distinct_ids.len(),
            duplicate_ids,
            known_count,
            unknown_count,
            warning_samples,
            unknown_samples,
            unknown_ids.join(",")
        );
    }

    println!(
        "ITEM PROBE status=STABLE pid={} regions={} requested_bytes={} id_matches={} elapsed_ms={}",
        process.pid,
        regions.len(),
        requested_bytes,
        matched_ids,
        started.elapsed().as_millis()
    );
    for (rank, (key, group)) in ranked.into_iter().take(MAX_REPORTED_GROUPS).enumerate() {
        let samples = group
            .samples
            .iter()
            .map(|(item_id, quantity)| format!("{item_id:08X}:{quantity}"))
            .collect::<Vec<_>>()
            .join(",");
        let high_samples = group
            .high_samples
            .iter()
            .map(|(item_id, quantity)| format!("{item_id:08X}:{quantity}"))
            .collect::<Vec<_>>()
            .join(",");
        let (common_stride, stride_count) = group.common_stride().unwrap_or_default();
        let (longest_run, run_samples) = group.longest_run(common_stride);
        let run_samples = run_samples
            .iter()
            .map(|(item_id, quantity)| format!("{item_id:08X}:{quantity}"))
            .collect::<Vec<_>>()
            .join(",");
        let (control_matches, control_span_records, control_start, control_samples) =
            group.best_control_cluster();
        let control_samples = control_samples
            .iter()
            .map(|(item_id, quantity)| format!("{item_id:08X}:{quantity}"))
            .collect::<Vec<_>>()
            .join(",");
        let anchored_controls = group
            .controls_near_anchor()
            .iter()
            .map(|(item_id, quantity)| format!("{item_id:08X}:{quantity}"))
            .collect::<Vec<_>>()
            .join(",");
        let exact_control_counts = group
            .exact_control_counts()
            .iter()
            .map(|(item_id, count)| format!("{item_id:08X}:{count}"))
            .collect::<Vec<_>>()
            .join(",");
        let control_record_delta =
            control_start.and_then(|address| address.checked_sub(player.record_address));
        let control_snapshot_delta =
            control_start.and_then(|address| address.checked_sub(player.snapshot_address));
        println!(
            "ITEM PROBE candidate rank={} region={} region_size={} quantity_offset={:+#x} occurrences={} distinct_items={} distinct_quantities={} common_stride={:#x} stride_count={} longest_run={} run_samples={} control_matches={}/{} control_span_records={} control_record_delta={} control_snapshot_delta={} control_samples={} anchored_controls={} exact_control_counts={} samples={} high_samples={}",
            rank + 1,
            key.region_index,
            regions[key.region_index].size,
            key.quantity_offset,
            group.occurrences,
            group.item_ids.len(),
            group.quantities.len(),
            common_stride,
            stride_count,
            longest_run,
            run_samples,
            control_matches,
            CONTROL_ITEMS.len(),
            control_span_records,
            control_record_delta
                .map(|delta| format!("{delta:#x}"))
                .unwrap_or_else(|| "none".to_string()),
            control_snapshot_delta
                .map(|delta| format!("{delta:#x}"))
                .unwrap_or_else(|| "none".to_string()),
            control_samples,
            anchored_controls,
            exact_control_counts,
            samples,
            high_samples
        );
    }
    Ok(())
}

fn parse_item_ids(source: &str) -> anyhow::Result<Vec<u32>> {
    let rows: HashMap<String, serde_json::Value> = serde_json::from_str(source)?;
    let mut ids = rows
        .into_keys()
        .map(|key| {
            u32::from_str_radix(&key, 16)
                .map_err(|_| anyhow::anyhow!("invalid item catalog key {key}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

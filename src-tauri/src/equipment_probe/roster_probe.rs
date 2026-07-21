use std::collections::{BTreeMap, BTreeSet, HashSet};

use super::{
    locator::{checked_address, read_u32, read_usize, validate_pointer, LocateError},
    memory::MemoryReader,
};

const CANDIDATE_BUCKET_STRIDE: usize = 0x10;
const MAX_CANDIDATE_BUCKETS: usize = 4096;
const MAX_CANDIDATE_NODES: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CandidateRecord {
    pub character_key: u32,
    pub snapshot_address: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagerInspection {
    pub candidate_bucket_count: usize,
    pub records: Vec<CandidateRecord>,
    pub duplicate_keys: Vec<u32>,
    pub rejected_record_count: usize,
}

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

pub(crate) fn inspect_candidate_manager<R: MemoryReader>(
    reader: &R,
    manager_global: usize,
) -> Result<ManagerInspection, RosterProbeError> {
    let manager = validate_pointer(read_usize(reader, manager_global)?)?;
    let sentinel = validate_pointer(read_usize(reader, checked_address(manager, 0xA30)?)?)?;
    let buckets = validate_pointer(read_usize(reader, checked_address(manager, 0xA40)?)?)?;
    let mask = read_u32(reader, checked_address(manager, 0xA58)?)?;
    let candidate_bucket_count = usize::try_from(mask)
        .expect("u32 mask fits in usize")
        .checked_add(1)
        .ok_or(LocateError::AddressOverflow)?;
    if candidate_bucket_count > MAX_CANDIDATE_BUCKETS {
        return Err(RosterProbeError::CandidateBucketLimit(
            candidate_bucket_count,
        ));
    }

    let mut records = BTreeMap::new();
    let mut duplicate_keys = BTreeSet::new();
    let mut rejected_record_count = 0;
    let mut visited = HashSet::new();

    for index in 0..candidate_bucket_count {
        let offset = index
            .checked_mul(CANDIDATE_BUCKET_STRIDE)
            .ok_or(LocateError::AddressOverflow)?;
        let bucket = checked_address(buckets, offset)?;
        let bucket_end = validate_pointer(read_usize(reader, bucket)?)?;
        let mut node = validate_pointer(read_usize(reader, checked_address(bucket, 0x08)?)?)?;

        while node != sentinel {
            if !visited.insert(node) {
                return Err(RosterProbeError::LinkedListCycle(node));
            }
            if visited.len() > MAX_CANDIDATE_NODES {
                return Err(RosterProbeError::TraversalLimit);
            }

            let character_key = read_u32(reader, checked_address(node, 0x10)?)?;
            let record_address = read_usize(reader, checked_address(node, 0x30)?)?;
            if let Ok(record_address) = validate_pointer(record_address) {
                let record_key = read_u32(reader, checked_address(record_address, 0x5EA8)?)?;
                if record_key == character_key {
                    let snapshot_address =
                        read_usize(reader, checked_address(record_address, 0x5E60)?)
                            .ok()
                            .and_then(|address| validate_pointer(address).ok());
                    let candidate = CandidateRecord {
                        character_key,
                        snapshot_address,
                    };
                    if records.insert(character_key, candidate).is_some() {
                        duplicate_keys.insert(character_key);
                    }
                } else {
                    rejected_record_count += 1;
                }
            } else {
                rejected_record_count += 1;
            }

            if node == bucket_end {
                break;
            }
            node = validate_pointer(read_usize(reader, checked_address(node, 0x08)?)?)?;
        }
    }

    Ok(ManagerInspection {
        candidate_bucket_count,
        records: records.into_values().collect(),
        duplicate_keys: duplicate_keys.into_iter().collect(),
        rejected_record_count,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{inspect_candidate_manager, RosterProbeError};
    use crate::equipment_probe::memory::{MemoryReadError, MemoryReader};

    const MANAGER_GLOBAL: usize = 0x2000_0000;
    const MANAGER: usize = 0x2100_0000;
    const BUCKETS: usize = 0x2200_0000;
    const SENTINEL: usize = 0x2300_0000;
    const NODE_BASE: usize = 0x2400_0000;
    const RECORD_BASE: usize = 0x3000_0000;
    const SNAPSHOT_BASE: usize = 0x4000_0000;

    #[derive(Default)]
    struct FakeMemory(BTreeMap<usize, u8>);

    impl FakeMemory {
        fn write(&mut self, address: usize, bytes: &[u8]) {
            for (offset, byte) in bytes.iter().copied().enumerate() {
                self.0.insert(address + offset, byte);
            }
        }

        fn write_u32(&mut self, address: usize, value: u32) {
            self.write(address, &value.to_le_bytes());
        }

        fn write_usize(&mut self, address: usize, value: usize) {
            self.write(address, &value.to_le_bytes());
        }
    }

    impl MemoryReader for FakeMemory {
        fn read_exact(&self, address: usize, output: &mut [u8]) -> Result<(), MemoryReadError> {
            for (offset, output_byte) in output.iter_mut().enumerate() {
                *output_byte = *self
                    .0
                    .get(&(address + offset))
                    .ok_or(MemoryReadError::Unavailable(address + offset))?;
            }
            Ok(())
        }
    }

    fn manager_with_mask(mask: u32) -> FakeMemory {
        let mut memory = FakeMemory::default();
        memory.write_usize(MANAGER_GLOBAL, MANAGER);
        memory.write_usize(MANAGER + 0xA30, SENTINEL);
        memory.write_usize(MANAGER + 0xA40, BUCKETS);
        memory.write_u32(MANAGER + 0xA58, mask);
        memory
    }

    fn write_bucket(memory: &mut FakeMemory, index: usize, first: usize, end: usize) {
        let bucket = BUCKETS + index * 0x10;
        memory.write_usize(bucket, end);
        memory.write_usize(bucket + 0x08, first);
    }

    fn write_node(memory: &mut FakeMemory, index: usize, next: usize, key: u32, record: usize) {
        let node = NODE_BASE + index * 0x100;
        memory.write_usize(node + 0x08, next);
        memory.write_u32(node + 0x10, key);
        memory.write_usize(node + 0x30, record);
    }

    fn write_record(memory: &mut FakeMemory, record: usize, key: u32, snapshot: usize) {
        memory.write_usize(record + 0x5E60, snapshot);
        memory.write_u32(record + 0x5EA8, key);
    }

    fn manager_fixture_with_three_known_records() -> FakeMemory {
        let mut memory = manager_with_mask(3);
        for index in 0..4 {
            write_bucket(&mut memory, index, SENTINEL, SENTINEL);
        }

        let keys = [0xE705_3919, 0x74DD_4C79, 0x9B15_CFB1];
        let nodes = [NODE_BASE, NODE_BASE + 0x100, NODE_BASE + 0x200];
        write_bucket(&mut memory, 1, nodes[0], nodes[2]);
        for (index, key) in keys.into_iter().enumerate() {
            let next = nodes.get(index + 1).copied().unwrap_or(SENTINEL);
            let record = RECORD_BASE + index * 0x1_0000;
            write_node(&mut memory, index, next, key, record);
            write_record(&mut memory, record, key, SNAPSHOT_BASE + index * 0x1000);
        }
        memory
    }

    fn manager_with_cycle() -> FakeMemory {
        let mut memory = manager_with_mask(0);
        let first = NODE_BASE;
        let second = NODE_BASE + 0x100;
        write_bucket(&mut memory, 0, first, NODE_BASE + 0x200);
        write_node(&mut memory, 0, second, 0xE705_3919, RECORD_BASE);
        write_node(&mut memory, 1, first, 0x9B15_CFB1, RECORD_BASE + 0x1_0000);
        write_record(&mut memory, RECORD_BASE, 0xE705_3919, SNAPSHOT_BASE);
        write_record(
            &mut memory,
            RECORD_BASE + 0x1_0000,
            0x9B15_CFB1,
            SNAPSHOT_BASE + 0x1000,
        );
        memory
    }

    fn manager_with_1025_nodes() -> FakeMemory {
        let mut memory = manager_with_mask(0);
        write_bucket(&mut memory, 0, NODE_BASE, NODE_BASE + 1024 * 0x100);
        for index in 0..1025 {
            let node = NODE_BASE + index * 0x100;
            let next = if index == 1024 {
                SENTINEL
            } else {
                node + 0x100
            };
            let record = RECORD_BASE + index * 0x1_0000;
            let key = 0x1000_0000_u32.wrapping_add(index as u32);
            write_node(&mut memory, index, next, key, record);
            write_record(&mut memory, record, key, SNAPSHOT_BASE);
        }
        memory
    }

    fn manager_with_duplicate_and_invalid_snapshot(key: u32) -> FakeMemory {
        let mut memory = manager_with_mask(0);
        write_bucket(&mut memory, 0, NODE_BASE, NODE_BASE + 0x100);
        write_node(&mut memory, 0, NODE_BASE + 0x100, key, RECORD_BASE);
        write_node(&mut memory, 1, SENTINEL, key, RECORD_BASE + 0x1_0000);
        write_record(&mut memory, RECORD_BASE, key, 0);
        write_record(&mut memory, RECORD_BASE + 0x1_0000, key, 0);
        memory
    }

    fn manager_with_self_key_mismatch(node_key: u32, record_key: u32) -> FakeMemory {
        let mut memory = manager_with_mask(0);
        write_bucket(&mut memory, 0, NODE_BASE, NODE_BASE);
        write_node(&mut memory, 0, SENTINEL, node_key, RECORD_BASE);
        write_record(&mut memory, RECORD_BASE, record_key, SNAPSHOT_BASE);
        memory
    }

    #[test]
    fn inspects_empty_single_and_collision_candidate_buckets() {
        let result =
            inspect_candidate_manager(&manager_fixture_with_three_known_records(), MANAGER_GLOBAL)
                .unwrap();

        assert_eq!(result.candidate_bucket_count, 4);
        assert_eq!(
            result
                .records
                .iter()
                .map(|entry| entry.character_key)
                .collect::<Vec<_>>(),
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
        )
        .unwrap();

        assert_eq!(result.duplicate_keys, vec![0xE705_3919]);
        assert_eq!(result.records.len(), 1);
        assert_eq!(result.records[0].snapshot_address, None);
    }

    #[test]
    fn rejects_a_self_key_mismatch() {
        let result = inspect_candidate_manager(
            &manager_with_self_key_mismatch(0xE705_3919, 0x9B15_CFB1),
            MANAGER_GLOBAL,
        )
        .unwrap();

        assert!(result.records.is_empty());
        assert_eq!(result.rejected_record_count, 1);
    }
}

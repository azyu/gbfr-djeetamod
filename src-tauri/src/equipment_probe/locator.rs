use std::collections::HashSet;

use equipment_core::EMPTY_HASH;
use thiserror::Error;

use super::memory::{MemoryReadError, MemoryReader};

const MIN_USER_ADDRESS: usize = 0x1_0000;
const MAX_USER_ADDRESS: usize = 0x0000_7FFF_FFFF_FFFF;
const MAX_PLAYER_NODES: usize = 1024;
const LOOKUP_PATTERN: &str = concat!(
    "56 57 48 83 EC 38 48 8B 31 48 8B 0D ?? ?? ?? ?? ",
    "C6 44 24 30 00 C6 44 24 28 00 C6 44 24 20 00 ",
    "31 D2 45 31 C0 45 31 C9 E8 ?? ?? ?? ?? ",
    "80 B8 BC 5E 00 00 00 B9 B0 E0 7A 88 74 ?? 8B 88 A8 5E 00 00"
);
const GETTER_PROLOGUE: [u8; 16] = [
    0x41, 0x57, 0x41, 0x56, 0x41, 0x55, 0x41, 0x54, 0x56, 0x57, 0x55, 0x53, 0x48, 0x83, 0xEC, 0x68,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LocatedEquipment {
    pub character_key: u32,
    pub record_address: usize,
    pub snapshot_address: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResolvedRoots {
    pub match_rva: usize,
    pub local_key_global: usize,
    pub manager_global: usize,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum LocateError {
    #[error("equipment lookup signature matched {0} times")]
    PatternMatchCount(usize),
    #[error("relative address overflow")]
    AddressOverflow,
    #[error(transparent)]
    Memory(#[from] MemoryReadError),
    #[error("invalid user pointer {0:#x}")]
    InvalidPointer(usize),
    #[error("local equipment character key is empty")]
    EmptyCharacterKey,
    #[error("local party slot {0} is outside 0..4")]
    InvalidLocalSlot(usize),
    #[error("local player key {character_key:#010x} was not found in the player table")]
    PlayerNotFound { character_key: u32 },
    #[error("player hash-table list contains a cycle at {0:#x}")]
    LinkedListCycle(usize),
    #[error("player hash-table traversal exceeded {MAX_PLAYER_NODES} nodes")]
    TraversalLimit,
    #[error("player record key {actual:#010x} does not match local key {expected:#010x}")]
    RecordCharacterMismatch { actual: u32, expected: u32 },
    #[error("equipment lookup instruction layout is invalid")]
    InvalidLookupLayout,
    #[error("player getter instruction layout is invalid")]
    InvalidGetterLayout,
}

fn checked_address(base: usize, offset: usize) -> Result<usize, LocateError> {
    base.checked_add(offset).ok_or(LocateError::AddressOverflow)
}

fn validate_pointer(address: usize) -> Result<usize, LocateError> {
    if (MIN_USER_ADDRESS..=MAX_USER_ADDRESS).contains(&address) {
        Ok(address)
    } else {
        Err(LocateError::InvalidPointer(address))
    }
}

fn read_u32<R: MemoryReader>(reader: &R, address: usize) -> Result<u32, LocateError> {
    validate_pointer(address)?;
    let mut bytes = [0u8; 4];
    reader.read_exact(address, &mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_usize<R: MemoryReader>(reader: &R, address: usize) -> Result<usize, LocateError> {
    validate_pointer(address)?;
    let mut bytes = [0u8; std::mem::size_of::<usize>()];
    reader.read_exact(address, &mut bytes)?;
    Ok(usize::from_le_bytes(bytes))
}

pub(crate) fn locate_from_globals<R: MemoryReader>(
    reader: &R,
    local_key_global: usize,
    manager_global: usize,
) -> Result<LocatedEquipment, LocateError> {
    locate_from_globals_slot(reader, local_key_global, manager_global, 0)
}

pub(crate) fn locate_from_globals_slot<R: MemoryReader>(
    reader: &R,
    local_key_global: usize,
    manager_global: usize,
    slot: usize,
) -> Result<LocatedEquipment, LocateError> {
    if slot >= 4 {
        return Err(LocateError::InvalidLocalSlot(slot));
    }
    let local_keys = validate_pointer(read_usize(reader, local_key_global)?)?;
    let key_offset = slot.checked_mul(0x10).ok_or(LocateError::AddressOverflow)?;
    let character_key = read_u32(reader, checked_address(local_keys, key_offset)?)?;
    if character_key == 0 || character_key == EMPTY_HASH {
        return Err(LocateError::EmptyCharacterKey);
    }

    let manager = validate_pointer(read_usize(reader, manager_global)?)?;
    let sentinel = validate_pointer(read_usize(reader, checked_address(manager, 0xA30)?)?)?;
    let buckets = validate_pointer(read_usize(reader, checked_address(manager, 0xA40)?)?)?;
    let mask = read_u32(reader, checked_address(manager, 0xA58)?)?;
    let index = usize::try_from(mask & character_key).expect("u32 index fits in usize");
    let bucket_offset = index
        .checked_mul(0x10)
        .ok_or(LocateError::AddressOverflow)?;
    let bucket = checked_address(buckets, bucket_offset)?;
    let bucket_end = read_usize(reader, bucket)?;
    let mut node = validate_pointer(read_usize(reader, checked_address(bucket, 0x08)?)?)?;
    let mut visited = HashSet::new();
    let found = loop {
        if node == sentinel {
            return Err(LocateError::PlayerNotFound { character_key });
        }
        if !visited.insert(node) {
            return Err(LocateError::LinkedListCycle(node));
        }
        if visited.len() > MAX_PLAYER_NODES {
            return Err(LocateError::TraversalLimit);
        }
        if read_u32(reader, checked_address(node, 0x10)?)? == character_key {
            break node;
        }
        if node == bucket_end {
            return Err(LocateError::PlayerNotFound { character_key });
        }
        node = validate_pointer(read_usize(reader, checked_address(node, 0x08)?)?)?;
    };

    let record = validate_pointer(read_usize(reader, checked_address(found, 0x30)?)?)?;

    let record_key = read_u32(reader, checked_address(record, 0x5EA8)?)?;
    if record_key != character_key {
        return Err(LocateError::RecordCharacterMismatch {
            actual: record_key,
            expected: character_key,
        });
    }
    let snapshot = validate_pointer(read_usize(reader, checked_address(record, 0x5E60)?)?)?;

    Ok(LocatedEquipment {
        character_key,
        record_address: record,
        snapshot_address: snapshot,
    })
}

pub(crate) fn find_unique_pattern(
    haystack: &[u8],
    pattern: &[Option<u8>],
) -> Result<usize, LocateError> {
    let matches = haystack
        .windows(pattern.len())
        .enumerate()
        .filter_map(|(offset, candidate)| {
            candidate
                .iter()
                .zip(pattern)
                .all(|(actual, expected)| match expected {
                    Some(expected) => actual == expected,
                    None => true,
                })
                .then_some(offset)
        })
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [offset] => Ok(*offset),
        _ => Err(LocateError::PatternMatchCount(matches.len())),
    }
}

pub(crate) fn resolve_rel32(
    instruction_address: usize,
    displacement: i32,
    instruction_len: usize,
) -> Result<usize, LocateError> {
    let next_instruction = instruction_address
        .checked_add(instruction_len)
        .ok_or(LocateError::AddressOverflow)?;
    if displacement >= 0 {
        next_instruction
            .checked_add(displacement as usize)
            .ok_or(LocateError::AddressOverflow)
    } else {
        next_instruction
            .checked_sub(displacement.unsigned_abs() as usize)
            .ok_or(LocateError::AddressOverflow)
    }
}

fn parse_pattern(pattern: &str) -> Vec<Option<u8>> {
    pattern
        .split_ascii_whitespace()
        .map(|value| {
            (value != "??").then(|| {
                u8::from_str_radix(value, 16).expect("internal equipment pattern is valid hex")
            })
        })
        .collect()
}

fn read_i32(bytes: &[u8], offset: usize) -> Result<i32, LocateError> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or(LocateError::InvalidLookupLayout)?;
    Ok(i32::from_le_bytes(
        value.try_into().expect("four-byte displacement"),
    ))
}

pub(crate) fn resolve_roots<R: MemoryReader>(
    reader: &R,
    module_base: usize,
    text_rva: usize,
    text: &[u8],
) -> Result<ResolvedRoots, LocateError> {
    let pattern = parse_pattern(LOOKUP_PATTERN);
    let match_offset = find_unique_pattern(text, &pattern)?;
    let match_rva = text_rva
        .checked_add(match_offset)
        .ok_or(LocateError::AddressOverflow)?;
    let match_address = module_base
        .checked_add(match_rva)
        .ok_or(LocateError::AddressOverflow)?;

    let local_displacement = read_i32(text, match_offset + 0x0C)?;
    let local_key_global =
        resolve_rel32(checked_address(match_address, 0x09)?, local_displacement, 7)?;
    let getter_displacement = read_i32(text, match_offset + 0x28)?;
    let getter = resolve_rel32(
        checked_address(match_address, 0x27)?,
        getter_displacement,
        5,
    )?;

    let mut getter_bytes = [0u8; 0x4B];
    reader.read_exact(getter, &mut getter_bytes)?;
    if getter_bytes[..GETTER_PROLOGUE.len()] != GETTER_PROLOGUE
        || getter_bytes[0x44..0x47] != [0x48, 0x8B, 0x35]
    {
        return Err(LocateError::InvalidGetterLayout);
    }
    let manager_displacement =
        read_i32(&getter_bytes, 0x47).map_err(|_| LocateError::InvalidGetterLayout)?;
    let manager_global = resolve_rel32(checked_address(getter, 0x44)?, manager_displacement, 7)?;

    Ok(ResolvedRoots {
        match_rva,
        local_key_global,
        manager_global,
    })
}

pub(crate) fn locate_equipment<R: MemoryReader>(
    reader: &R,
    module_base: usize,
    text_rva: usize,
    text: &[u8],
) -> Result<LocatedEquipment, LocateError> {
    let roots = resolve_roots(reader, module_base, text_rva, text)?;
    locate_from_globals(reader, roots.local_key_global, roots.manager_global)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        find_unique_pattern, locate_equipment, locate_from_globals, locate_from_globals_slot,
        resolve_rel32, LocateError,
    };
    use crate::equipment_probe::memory::{MemoryReadError, MemoryReader};

    #[derive(Default)]
    struct FakeMemory {
        regions: BTreeMap<usize, Vec<u8>>,
    }

    impl FakeMemory {
        fn insert(&mut self, address: usize, bytes: Vec<u8>) {
            self.regions.insert(address, bytes);
        }

        fn insert_u32(&mut self, address: usize, value: u32) {
            self.insert(address, value.to_le_bytes().to_vec());
        }

        fn insert_usize(&mut self, address: usize, value: usize) {
            self.insert(address, value.to_le_bytes().to_vec());
        }
    }

    impl MemoryReader for FakeMemory {
        fn read_exact(&self, address: usize, output: &mut [u8]) -> Result<(), MemoryReadError> {
            let (start, bytes) = self
                .regions
                .range(..=address)
                .next_back()
                .ok_or(MemoryReadError::Unavailable(address))?;
            let offset = address - start;
            let source = bytes
                .get(offset..offset + output.len())
                .ok_or(MemoryReadError::Unavailable(address))?;
            output.copy_from_slice(source);
            Ok(())
        }
    }

    fn player_fixture(module_base: usize) -> (FakeMemory, usize, usize, u32, usize, usize) {
        const LOCAL_KEYS: usize = 0x2000_0000;
        const MANAGER: usize = 0x2100_0000;
        const BUCKETS: usize = 0x2200_0000;
        const SENTINEL: usize = 0x2300_0000;
        const NODE: usize = 0x2400_0000;
        const RECORD: usize = 0x2500_0000;
        const SNAPSHOT: usize = 0x2600_0000;
        const KEY: u32 = 0xE705_3919;
        let local_key_global = module_base + 0x8_0000;
        let manager_global = module_base + 0x8_0010;

        let mut memory = FakeMemory::default();
        memory.insert_usize(local_key_global, LOCAL_KEYS);
        memory.insert_u32(LOCAL_KEYS, KEY);
        memory.insert_usize(manager_global, MANAGER);

        let mut manager = vec![0u8; 0xA60];
        manager[0xA30..0xA38].copy_from_slice(&SENTINEL.to_le_bytes());
        manager[0xA40..0xA48].copy_from_slice(&BUCKETS.to_le_bytes());
        manager[0xA58..0xA5C].copy_from_slice(&0_u32.to_le_bytes());
        memory.insert(MANAGER, manager);

        let mut buckets = vec![0u8; 0x10];
        buckets[0x00..0x08].copy_from_slice(&NODE.to_le_bytes());
        buckets[0x08..0x10].copy_from_slice(&NODE.to_le_bytes());
        memory.insert(BUCKETS, buckets);

        let mut node = vec![0u8; 0x38];
        node[0x08..0x10].copy_from_slice(&SENTINEL.to_le_bytes());
        node[0x10..0x14].copy_from_slice(&KEY.to_le_bytes());
        node[0x30..0x38].copy_from_slice(&RECORD.to_le_bytes());
        memory.insert(NODE, node);

        let mut record = vec![0u8; 0x5EB0];
        record[0x5E60..0x5E68].copy_from_slice(&SNAPSHOT.to_le_bytes());
        record[0x5EA8..0x5EAC].copy_from_slice(&KEY.to_le_bytes());
        memory.insert(RECORD, record);

        (
            memory,
            local_key_global,
            manager_global,
            KEY,
            RECORD,
            SNAPSHOT,
        )
    }

    fn put_rel32(bytes: &mut [u8], offset: usize, next_address: usize, target: usize) {
        let displacement = i32::try_from(target as isize - next_address as isize).unwrap();
        bytes[offset..offset + 4].copy_from_slice(&displacement.to_le_bytes());
    }

    #[test]
    fn unique_pattern_is_required() {
        let pattern = [Some(0x48), None, Some(0x8B)];

        assert_eq!(find_unique_pattern(&[0x48, 0x11, 0x8B], &pattern), Ok(0));
        assert_eq!(
            find_unique_pattern(&[0x90, 0x90, 0x90], &pattern),
            Err(LocateError::PatternMatchCount(0))
        );
        assert_eq!(
            find_unique_pattern(&[0x48, 0x11, 0x8B, 0x48, 0x22, 0x8B], &pattern),
            Err(LocateError::PatternMatchCount(2))
        );
    }

    #[test]
    fn rel32_resolution_tracks_aslr() {
        let first = resolve_rel32(0x14000_1000, 0x1234, 7).unwrap();
        let second = resolve_rel32(0x18000_1000, 0x1234, 7).unwrap();

        assert_eq!(first, 0x14000_223B);
        assert_eq!(second - first, 0x4000_0000);
    }

    #[test]
    fn locates_record_in_player_hash_table() {
        let (memory, local_global, manager_global, key, record, snapshot) =
            player_fixture(0x14000_0000);

        let located = locate_from_globals(&memory, local_global, manager_global).unwrap();

        assert_eq!(located.character_key, key);
        assert_eq!(located.record_address, record);
        assert_eq!(located.snapshot_address, snapshot);
    }

    #[test]
    fn locates_a_nonzero_local_party_slot() {
        const LOCAL_KEYS: usize = 0x2000_0000;
        const BUCKETS: usize = 0x2200_0000;
        const SENTINEL: usize = 0x2300_0000;
        const FIRST_NODE: usize = 0x2400_0000;
        const SECOND_NODE: usize = 0x2410_0000;
        const SECOND_RECORD: usize = 0x2510_0000;
        const SECOND_SNAPSHOT: usize = 0x2610_0000;
        const SECOND_KEY: u32 = 0xDD7A_151E;
        let (mut memory, local_global, manager_global, first_key, _, _) =
            player_fixture(0x14000_0000);
        memory.insert_u32(LOCAL_KEYS + 0x10, SECOND_KEY);
        memory.insert(
            BUCKETS,
            [SECOND_NODE.to_le_bytes(), FIRST_NODE.to_le_bytes()].concat(),
        );
        let mut first_node = vec![0u8; 0x38];
        first_node[0x08..0x10].copy_from_slice(&SECOND_NODE.to_le_bytes());
        first_node[0x10..0x14].copy_from_slice(&first_key.to_le_bytes());
        memory.insert(FIRST_NODE, first_node);
        let mut second_node = vec![0u8; 0x38];
        second_node[0x08..0x10].copy_from_slice(&SENTINEL.to_le_bytes());
        second_node[0x10..0x14].copy_from_slice(&SECOND_KEY.to_le_bytes());
        second_node[0x30..0x38].copy_from_slice(&SECOND_RECORD.to_le_bytes());
        memory.insert(SECOND_NODE, second_node);
        let mut second_record = vec![0u8; 0x5EB0];
        second_record[0x5E60..0x5E68].copy_from_slice(&SECOND_SNAPSHOT.to_le_bytes());
        second_record[0x5EA8..0x5EAC].copy_from_slice(&SECOND_KEY.to_le_bytes());
        memory.insert(SECOND_RECORD, second_record);

        let located = locate_from_globals_slot(&memory, local_global, manager_global, 1).unwrap();
        assert_eq!(located.character_key, SECOND_KEY);
        assert_eq!(located.record_address, SECOND_RECORD);
        assert_eq!(located.snapshot_address, SECOND_SNAPSHOT);
    }

    #[test]
    fn rejects_record_for_a_different_character() {
        let (mut memory, local_global, manager_global, _, record, _) = player_fixture(0x14000_0000);
        memory.insert_u32(record + 0x5EA8, 0x079D_F0CC);

        assert!(matches!(
            locate_from_globals(&memory, local_global, manager_global),
            Err(LocateError::RecordCharacterMismatch { .. })
        ));
    }

    #[test]
    fn locates_equipment_from_the_202_pattern_at_different_module_bases() {
        for module_base in [0x14000_0000, 0x18000_0000] {
            let (mut memory, local_global, manager_global, key, record, snapshot) =
                player_fixture(module_base);
            let text_rva = 0x1000;
            let match_offset = 0x20;
            let match_address = module_base + text_rva + match_offset;
            let getter = module_base + 0x5000;
            let mut text = vec![0x90; 0x100];
            let lookup = [
                0x56, 0x57, 0x48, 0x83, 0xEC, 0x38, 0x48, 0x8B, 0x31, 0x48, 0x8B, 0x0D, 0, 0, 0, 0,
                0xC6, 0x44, 0x24, 0x30, 0, 0xC6, 0x44, 0x24, 0x28, 0, 0xC6, 0x44, 0x24, 0x20, 0,
                0x31, 0xD2, 0x45, 0x31, 0xC0, 0x45, 0x31, 0xC9, 0xE8, 0, 0, 0, 0, 0x80, 0xB8, 0xBC,
                0x5E, 0, 0, 0, 0xB9, 0xB0, 0xE0, 0x7A, 0x88, 0x74, 0x06, 0x8B, 0x88, 0xA8, 0x5E, 0,
                0,
            ];
            text[match_offset..match_offset + lookup.len()].copy_from_slice(&lookup);
            put_rel32(
                &mut text,
                match_offset + 0x0C,
                match_address + 0x10,
                local_global,
            );
            put_rel32(&mut text, match_offset + 0x28, match_address + 0x2C, getter);

            let mut getter_bytes = vec![0u8; 0x4B];
            getter_bytes[..16].copy_from_slice(&[
                0x41, 0x57, 0x41, 0x56, 0x41, 0x55, 0x41, 0x54, 0x56, 0x57, 0x55, 0x53, 0x48, 0x83,
                0xEC, 0x68,
            ]);
            getter_bytes[0x44..0x47].copy_from_slice(&[0x48, 0x8B, 0x35]);
            put_rel32(&mut getter_bytes, 0x47, getter + 0x4B, manager_global);
            memory.insert(getter, getter_bytes);

            let located = locate_equipment(&memory, module_base, text_rva, &text).expect("located");
            assert_eq!(located.character_key, key);
            assert_eq!(located.record_address, record);
            assert_eq!(located.snapshot_address, snapshot);
        }
    }
}

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
        Self {
            sigil_ids,
            trait_ids,
        }
    }

    pub fn known_non_empty_sigil_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.sigil_ids
            .iter()
            .copied()
            .filter(|value| !is_empty_id(*value))
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
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("validated inventory field"),
    )
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
            || record.primary_trait_level != 0
            || !is_empty_id(record.secondary_trait_id)
            || record.secondary_trait_level != 0
            || record.sigil_level != 0)
    {
        return Err(InventoryDecodeError::ContradictoryEmpty);
    }

    Ok(record)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::EMPTY_HASH;

    use super::{
        decode_inventory_record, InventoryCatalog, InventoryDecodeError, INVENTORY_RECORD_BYTES,
    };

    const SIGIL_ID: u32 = 0xEE73_2781;
    const PRIMARY_TRAIT_ID: u32 = 0xDC58_4F60;
    const SECONDARY_TRAIT_ID: u32 = 0x5007_9A1C;
    const CHARACTER_KEY: u32 = 0xE705_3919;

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn catalog() -> InventoryCatalog {
        InventoryCatalog::new(
            HashSet::from([SIGIL_ID]),
            HashSet::from([PRIMARY_TRAIT_ID, SECONDARY_TRAIT_ID]),
        )
    }

    fn occupied_fixture() -> [u8; INVENTORY_RECORD_BYTES] {
        let mut bytes = [0u8; INVENTORY_RECORD_BYTES];
        put_u32(&mut bytes, 0x00, PRIMARY_TRAIT_ID);
        put_u32(&mut bytes, 0x04, 15);
        put_u32(&mut bytes, 0x08, SECONDARY_TRAIT_ID);
        put_u32(&mut bytes, 0x0C, 11);
        put_u32(&mut bytes, 0x10, SIGIL_ID);
        put_u32(&mut bytes, 0x14, CHARACTER_KEY);
        put_u32(&mut bytes, 0x18, 15);
        put_u32(&mut bytes, 0x1C, 42);
        put_u32(&mut bytes, 0x20, 1);
        bytes
    }

    #[test]
    fn exposes_only_known_non_empty_sigil_ids() {
        let catalog =
            InventoryCatalog::new(HashSet::from([SIGIL_ID, 0, EMPTY_HASH]), HashSet::new());

        let mut ids = catalog.known_non_empty_sigil_ids().collect::<Vec<_>>();
        ids.sort_unstable();

        assert_eq!(ids, vec![SIGIL_ID]);
    }

    #[test]
    fn decodes_every_verified_inventory_field() {
        let record = decode_inventory_record(&occupied_fixture(), &catalog()).unwrap();

        assert_eq!(record.primary_trait_id, PRIMARY_TRAIT_ID);
        assert_eq!(record.primary_trait_level, 15);
        assert_eq!(record.secondary_trait_id, SECONDARY_TRAIT_ID);
        assert_eq!(record.secondary_trait_level, 11);
        assert_eq!(record.sigil_id, SIGIL_ID);
        assert_eq!(record.equipped_character_key, CHARACTER_KEY);
        assert_eq!(record.sigil_level, 15);
        assert_eq!(record.acquisition_index, 42);
        assert_eq!(record.state, 1);
        assert!(record.is_occupied());
    }

    #[test]
    fn accepts_an_exactly_zero_empty_record() {
        let record = decode_inventory_record(&[0u8; INVENTORY_RECORD_BYTES], &catalog()).unwrap();

        assert!(!record.is_occupied());
    }

    #[test]
    fn rejects_partial_or_unknown_records() {
        assert!(matches!(
            decode_inventory_record(&[0u8; 0x23], &catalog()),
            Err(InventoryDecodeError::TooShort { .. })
        ));

        let mut unknown_sigil = occupied_fixture();
        put_u32(&mut unknown_sigil, 0x10, 0x1234_5678);
        assert!(matches!(
            decode_inventory_record(&unknown_sigil, &catalog()),
            Err(InventoryDecodeError::UnknownSigil { .. })
        ));

        let mut unknown_trait = occupied_fixture();
        put_u32(&mut unknown_trait, 0x08, 0x1234_5678);
        assert!(matches!(
            decode_inventory_record(&unknown_trait, &catalog()),
            Err(InventoryDecodeError::UnknownTrait { .. })
        ));
    }

    #[test]
    fn rejects_invalid_levels_and_contradictory_empty_records() {
        let mut high_level = occupied_fixture();
        put_u32(&mut high_level, 0x04, 31);
        assert!(matches!(
            decode_inventory_record(&high_level, &catalog()),
            Err(InventoryDecodeError::InvalidLevel { .. })
        ));

        let mut contradictory_empty = [0u8; 0x24];
        put_u32(&mut contradictory_empty, 0x04, 1);
        assert!(matches!(
            decode_inventory_record(&contradictory_empty, &catalog()),
            Err(InventoryDecodeError::ContradictoryEmpty)
        ));
    }
}

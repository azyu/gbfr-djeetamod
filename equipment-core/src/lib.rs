use protocol::{
    EquipmentCaptureStatus, EquipmentSourceKind, EquippedTraitSource, LocalEquipmentSnapshotEvent,
};
use thiserror::Error;

pub const EMPTY_HASH: u32 = 0x887A_E0B0;
pub const SIGIL_COUNT: usize = 12;
pub const SIGIL_STRIDE: usize = 0x24;
pub const SIGIL_ARRAY_BYTES: usize = SIGIL_COUNT * SIGIL_STRIDE;
const MAX_TRAIT_LEVEL: u32 = 10_000;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DecodeError {
    #[error("sigil snapshot is {actual:#x} bytes; expected at least {required:#x}")]
    SnapshotTooShort { actual: usize, required: usize },
    #[error("equipment character key is empty")]
    EmptyCharacterKey,
    #[error("sigil slot {slot} belongs to {actual:#010x}, expected {expected:#010x}")]
    WrongCharacter {
        slot: usize,
        actual: u32,
        expected: u32,
    },
    #[error("sigil slot {slot} has invalid trait level {level}")]
    InvalidTraitLevel { slot: usize, level: u32 },
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("validated sigil field"),
    )
}

fn is_empty_hash(value: u32) -> bool {
    value == 0 || value == EMPTY_HASH
}

fn push_trait(
    sources: &mut Vec<EquippedTraitSource>,
    kind: EquipmentSourceKind,
    slot: usize,
    item_id: u32,
    trait_id: u32,
    trait_level: u32,
) -> Result<(), DecodeError> {
    if is_empty_hash(trait_id) {
        return Ok(());
    }

    if !(1..=MAX_TRAIT_LEVEL).contains(&trait_level) {
        return Err(DecodeError::InvalidTraitLevel {
            slot,
            level: trait_level,
        });
    }

    sources.push(EquippedTraitSource {
        kind,
        slot: u8::try_from(slot).expect("twelve slots fit in u8"),
        item_id,
        trait_id,
        trait_level,
    });
    Ok(())
}

pub fn decode_snapshot(
    bytes: &[u8],
    character_key: u32,
) -> Result<LocalEquipmentSnapshotEvent, DecodeError> {
    if bytes.len() < SIGIL_ARRAY_BYTES {
        return Err(DecodeError::SnapshotTooShort {
            actual: bytes.len(),
            required: SIGIL_ARRAY_BYTES,
        });
    }
    if is_empty_hash(character_key) {
        return Err(DecodeError::EmptyCharacterKey);
    }

    let mut sources = Vec::new();
    for slot in 0..SIGIL_COUNT {
        let base = slot * SIGIL_STRIDE;
        let primary_trait = read_u32(bytes, base);
        let primary_level = read_u32(bytes, base + 0x04);
        let secondary_trait = read_u32(bytes, base + 0x08);
        let secondary_level = read_u32(bytes, base + 0x0C);
        let sigil_id = read_u32(bytes, base + 0x10);
        let equipped_character = read_u32(bytes, base + 0x14);

        if is_empty_hash(sigil_id) {
            continue;
        }
        if equipped_character != character_key {
            return Err(DecodeError::WrongCharacter {
                slot,
                actual: equipped_character,
                expected: character_key,
            });
        }

        push_trait(
            &mut sources,
            EquipmentSourceKind::SigilPrimary,
            slot,
            sigil_id,
            primary_trait,
            primary_level,
        )?;
        push_trait(
            &mut sources,
            EquipmentSourceKind::SigilSecondary,
            slot,
            sigil_id,
            secondary_trait,
            secondary_level,
        )?;
    }

    Ok(LocalEquipmentSnapshotEvent {
        character_type: character_key,
        status: EquipmentCaptureStatus::Complete,
        sources,
    })
}

#[cfg(test)]
mod tests {
    use protocol::EquipmentSourceKind;

    use super::{decode_snapshot, EMPTY_HASH, SIGIL_ARRAY_BYTES};

    const CHARACTER_KEY: u32 = 0xE705_3919;

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn empty_fixture() -> Vec<u8> {
        let mut bytes = vec![0u8; SIGIL_ARRAY_BYTES];
        for slot in 0..12 {
            let base = slot * 0x24;
            put_u32(&mut bytes, base, EMPTY_HASH);
            put_u32(&mut bytes, base + 0x08, EMPTY_HASH);
            put_u32(&mut bytes, base + 0x10, EMPTY_HASH);
            put_u32(&mut bytes, base + 0x14, EMPTY_HASH);
        }
        bytes
    }

    fn fixture_with_sigil() -> Vec<u8> {
        let mut bytes = empty_fixture();
        put_u32(&mut bytes, 0x00, 0xDC58_4F60);
        put_u32(&mut bytes, 0x04, 15);
        put_u32(&mut bytes, 0x08, 0x5007_9A1C);
        put_u32(&mut bytes, 0x0C, 11);
        put_u32(&mut bytes, 0x10, 0xEE73_2781);
        put_u32(&mut bytes, 0x14, CHARACTER_KEY);
        bytes
    }

    #[test]
    fn decodes_primary_and_secondary_traits() {
        let event = decode_snapshot(&fixture_with_sigil(), CHARACTER_KEY).unwrap();

        assert_eq!(event.character_type, CHARACTER_KEY);
        assert_eq!(event.sources.len(), 2);
        assert_eq!(event.sources[0].kind, EquipmentSourceKind::SigilPrimary);
        assert_eq!(event.sources[1].kind, EquipmentSourceKind::SigilSecondary);
    }

    #[test]
    fn rejects_partial_array_and_wrong_character() {
        assert!(decode_snapshot(&vec![0; SIGIL_ARRAY_BYTES - 1], CHARACTER_KEY).is_err());

        let mut bytes = fixture_with_sigil();
        put_u32(&mut bytes, 0x14, 0x079D_F0CC);
        assert!(decode_snapshot(&bytes, CHARACTER_KEY).is_err());
    }
}

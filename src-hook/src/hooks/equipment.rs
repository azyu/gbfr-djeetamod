use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

use anyhow::{bail, ensure, Context, Result};
use protocol::{
    EquipmentCaptureStatus, EquipmentSourceKind, EquippedTraitSource, LocalEquipmentSnapshotEvent,
    Message,
};

use crate::event;

pub(super) const EMPTY_HASH: u32 = 0x887A_E0B0;
const SIGIL_COUNT: usize = 12;
const SIGIL_STRIDE: usize = 0x24;
pub(super) const SIGIL_ARRAY_BYTES: usize = SIGIL_COUNT * SIGIL_STRIDE;
const MAX_TRAIT_LEVEL: u32 = 10_000;

#[derive(Default)]
pub(super) struct EquipmentSnapshotCache {
    by_character: HashMap<u32, LocalEquipmentSnapshotEvent>,
}

impl EquipmentSnapshotCache {
    #[cfg(test)]
    pub(super) fn replace_if_changed(
        &mut self,
        event: LocalEquipmentSnapshotEvent,
    ) -> Option<LocalEquipmentSnapshotEvent> {
        if self.by_character.get(&event.character_type) == Some(&event) {
            return None;
        }

        self.by_character
            .insert(event.character_type, event.clone());
        Some(event)
    }

    pub(super) fn clear(&mut self) {
        self.by_character.clear();
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.by_character.len()
    }
}

static SNAPSHOTS: OnceLock<Mutex<EquipmentSnapshotCache>> = OnceLock::new();

pub(super) fn reset_snapshot_cache() {
    if let Some(snapshots) = SNAPSHOTS.get() {
        snapshots
            .lock()
            .expect("equipment snapshot cache lock poisoned")
            .clear();
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let value = bytes
        .get(offset..offset + 4)
        .context("sigil field is outside the captured array")?;
    Ok(u32::from_le_bytes(
        value.try_into().expect("four-byte slice"),
    ))
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
) -> Result<()> {
    if is_empty_hash(trait_id) {
        return Ok(());
    }

    ensure!(
        (1..=MAX_TRAIT_LEVEL).contains(&trait_level),
        "sigil slot {slot} has invalid trait level {trait_level}"
    );
    sources.push(EquippedTraitSource {
        kind,
        slot: u8::try_from(slot).expect("twelve slots fit in u8"),
        item_id,
        trait_id,
        trait_level,
    });
    Ok(())
}

pub(super) fn decode_snapshot(
    bytes: &[u8],
    character_key: u32,
) -> Result<LocalEquipmentSnapshotEvent> {
    ensure!(
        bytes.len() >= SIGIL_ARRAY_BYTES,
        "sigil snapshot is shorter than {SIGIL_ARRAY_BYTES:#x} bytes"
    );
    ensure!(
        !is_empty_hash(character_key),
        "equipment character key is empty"
    );

    let mut sources = Vec::new();
    for slot in 0..SIGIL_COUNT {
        let base = slot * SIGIL_STRIDE;
        let primary_trait = read_u32(bytes, base)?;
        let primary_level = read_u32(bytes, base + 0x04)?;
        let secondary_trait = read_u32(bytes, base + 0x08)?;
        let secondary_level = read_u32(bytes, base + 0x0C)?;
        let sigil_id = read_u32(bytes, base + 0x10)?;
        let equipped_character = read_u32(bytes, base + 0x14)?;

        if is_empty_hash(sigil_id) {
            continue;
        }
        if equipped_character != character_key {
            bail!(
                "sigil slot {slot} belongs to {equipped_character:#010x}, expected {character_key:#010x}"
            );
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

fn publish_if_changed(
    cache: &mut EquipmentSnapshotCache,
    tx: &event::Tx,
    event: LocalEquipmentSnapshotEvent,
) -> bool {
    if cache.by_character.get(&event.character_type) == Some(&event) {
        return false;
    }
    if tx
        .send(Message::LocalEquipmentSnapshot(event.clone()))
        .is_err()
    {
        return false;
    }

    cache.by_character.insert(event.character_type, event);
    true
}

pub(super) fn capture_local_snapshot(
    tx: &event::Tx,
    snapshot: *const u8,
    character_key: u32,
) -> Result<()> {
    let bytes = super::read_process_bytes(snapshot, SIGIL_ARRAY_BYTES)
        .context("could not copy local sigil array")?;
    let event = decode_snapshot(&bytes, character_key)?;
    let mut snapshots = SNAPSHOTS
        .get_or_init(|| Mutex::new(EquipmentSnapshotCache::default()))
        .lock()
        .expect("equipment snapshot cache lock poisoned");
    publish_if_changed(&mut snapshots, tx, event);
    Ok(())
}

#[cfg(test)]
mod tests {
    use protocol::{EquipmentCaptureStatus, EquipmentSourceKind};

    use super::{
        decode_snapshot, publish_if_changed, EquipmentSnapshotCache, EMPTY_HASH, SIGIL_ARRAY_BYTES,
    };

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
        put_u32(&mut bytes, 0x18, 15);
        bytes
    }

    #[test]
    fn decodes_primary_and_secondary_sigil_traits() {
        let event = decode_snapshot(&fixture_with_sigil(), CHARACTER_KEY).unwrap();

        assert_eq!(event.character_type, CHARACTER_KEY);
        assert_eq!(event.status, EquipmentCaptureStatus::Complete);
        assert_eq!(event.sources.len(), 2);
        assert_eq!(event.sources[0].kind, EquipmentSourceKind::SigilPrimary);
        assert_eq!(event.sources[0].slot, 0);
        assert_eq!(event.sources[0].item_id, 0xEE73_2781);
        assert_eq!(event.sources[0].trait_id, 0xDC58_4F60);
        assert_eq!(event.sources[0].trait_level, 15);
        assert_eq!(event.sources[1].kind, EquipmentSourceKind::SigilSecondary);
        assert_eq!(event.sources[1].trait_level, 11);
    }

    #[test]
    fn skips_all_twelve_empty_slots() {
        let event = decode_snapshot(&empty_fixture(), CHARACTER_KEY).unwrap();

        assert!(event.sources.is_empty());
    }

    #[test]
    fn rejects_partial_sigil_array() {
        assert!(decode_snapshot(&vec![0; SIGIL_ARRAY_BYTES - 1], CHARACTER_KEY).is_err());
    }

    #[test]
    fn rejects_mismatched_character_key() {
        let mut bytes = fixture_with_sigil();
        put_u32(&mut bytes, 0x14, 0x079D_F0CC);

        assert!(decode_snapshot(&bytes, CHARACTER_KEY).is_err());
    }

    #[test]
    fn rejects_unreasonable_trait_level() {
        let mut bytes = fixture_with_sigil();
        put_u32(&mut bytes, 0x04, 10_001);

        assert!(decode_snapshot(&bytes, CHARACTER_KEY).is_err());
    }

    #[test]
    fn identical_snapshots_are_suppressed_per_character() {
        let event = decode_snapshot(&fixture_with_sigil(), CHARACTER_KEY).unwrap();
        let mut cache = EquipmentSnapshotCache::default();

        assert!(cache.replace_if_changed(event.clone()).is_some());
        assert!(cache.replace_if_changed(event).is_none());
    }

    #[test]
    fn distinct_character_keys_keep_distinct_snapshots() {
        let first = decode_snapshot(&empty_fixture(), CHARACTER_KEY).unwrap();
        let second = decode_snapshot(&empty_fixture(), 0x079D_F0CC).unwrap();
        let mut cache = EquipmentSnapshotCache::default();

        assert!(cache.replace_if_changed(first).is_some());
        assert!(cache.replace_if_changed(second).is_some());
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn failed_publish_does_not_suppress_the_first_connected_snapshot() {
        let event = decode_snapshot(&fixture_with_sigil(), CHARACTER_KEY).unwrap();
        let (tx, receiver) = tokio::sync::broadcast::channel(4);
        drop(receiver);
        let mut cache = EquipmentSnapshotCache::default();

        assert!(!publish_if_changed(&mut cache, &tx, event.clone()));
        assert_eq!(cache.len(), 0);

        let mut receiver = tx.subscribe();
        assert!(publish_if_changed(&mut cache, &tx, event));
        assert!(matches!(
            receiver.try_recv(),
            Ok(protocol::Message::LocalEquipmentSnapshot(_))
        ));
    }

    #[test]
    fn clearing_cache_allows_same_snapshot_after_pipe_reconnect() {
        let event = decode_snapshot(&fixture_with_sigil(), CHARACTER_KEY).unwrap();
        let mut cache = EquipmentSnapshotCache::default();

        assert!(cache.replace_if_changed(event.clone()).is_some());
        cache.clear();
        assert!(cache.replace_if_changed(event).is_some());
    }
}

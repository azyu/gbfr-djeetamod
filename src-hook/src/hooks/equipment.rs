use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

use anyhow::{Context, Result};
use equipment_core::{decode_snapshot, SIGIL_ARRAY_BYTES};
use protocol::{LocalEquipmentSnapshotEvent, Message};

use crate::event;

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
    use equipment_core::{decode_snapshot, EMPTY_HASH, SIGIL_ARRAY_BYTES};
    use protocol::{EquipmentCaptureStatus, EquipmentSourceKind};

    use super::{publish_if_changed, EquipmentSnapshotCache};

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

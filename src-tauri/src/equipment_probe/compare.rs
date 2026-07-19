use std::{
    collections::{BTreeMap, HashMap},
    time::{Duration, Instant},
};

use equipment_core::decode_snapshot;
use protocol::{
    EquipmentCaptureStatus, EquipmentSourceKind, EquippedTraitSource, LocalEquipmentSnapshotEvent,
};
use sha2::{Digest, Sha256};

const REPEAT_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ComparisonSummary {
    pub character_key: u32,
    pub source_count: usize,
    pub snapshot_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DifferenceField {
    CharacterKey,
    ItemId,
    TraitId,
    TraitLevel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SlotDifference {
    pub slot: Option<u8>,
    pub kind: Option<EquipmentSourceKind>,
    pub field: DifferenceField,
    pub hook: Option<u32>,
    pub external: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeferredReason {
    UnstableRead,
    InvalidSnapshot,
    MissingHookTruth,
    HookUnsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompareDecision {
    Match(ComparisonSummary),
    Mismatch(Vec<SlotDifference>),
    Deferred(DeferredReason),
    Suppressed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EmissionFingerprint {
    digest: [u8; 32],
    differences: Vec<SlotDifference>,
}

#[derive(Debug, Default)]
pub(crate) struct ProbeComparator {
    hook_truth: HashMap<u32, LocalEquipmentSnapshotEvent>,
    last_emission: Option<(EmissionFingerprint, Instant)>,
}

impl ProbeComparator {
    pub fn record_hook(&mut self, event: LocalEquipmentSnapshotEvent) {
        self.hook_truth.insert(event.character_type, event);
    }

    pub fn compare_external(
        &mut self,
        character_key: u32,
        first: &[u8],
        second: &[u8],
        now: Instant,
    ) -> CompareDecision {
        if first != second {
            return CompareDecision::Deferred(DeferredReason::UnstableRead);
        }
        let external = match decode_snapshot(first, character_key) {
            Ok(snapshot) => snapshot,
            Err(_) => return CompareDecision::Deferred(DeferredReason::InvalidSnapshot),
        };
        let Some(hook) = self.hook_truth.get(&character_key) else {
            return CompareDecision::Deferred(DeferredReason::MissingHookTruth);
        };
        if hook.status != EquipmentCaptureStatus::Complete {
            return CompareDecision::Deferred(DeferredReason::HookUnsupported);
        }

        let differences = compare_snapshots(hook, &external);
        let digest: [u8; 32] = Sha256::digest(first).into();
        let fingerprint = EmissionFingerprint {
            digest,
            differences: differences.clone(),
        };
        if self
            .last_emission
            .as_ref()
            .is_some_and(|(previous, emitted_at)| {
                previous == &fingerprint
                    && now.saturating_duration_since(*emitted_at) < REPEAT_INTERVAL
            })
        {
            return CompareDecision::Suppressed;
        }
        self.last_emission = Some((fingerprint, now));

        if differences.is_empty() {
            CompareDecision::Match(ComparisonSummary {
                character_key,
                source_count: external.sources.len(),
                snapshot_digest: digest_prefix(&digest),
            })
        } else {
            CompareDecision::Mismatch(differences)
        }
    }
}

pub(crate) fn snapshot_digest_prefix(bytes: &[u8]) -> String {
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    digest_prefix(&digest)
}

fn digest_prefix(digest: &[u8; 32]) -> String {
    digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn kind_rank(kind: EquipmentSourceKind) -> u8 {
    match kind {
        EquipmentSourceKind::SigilPrimary => 0,
        EquipmentSourceKind::SigilSecondary => 1,
        EquipmentSourceKind::Weapon => 2,
        EquipmentSourceKind::Wrightstone => 3,
        EquipmentSourceKind::MasterTrait => 4,
        EquipmentSourceKind::Summon => 5,
    }
}

fn normalized_sources(sources: &[EquippedTraitSource]) -> BTreeMap<(u8, u8), &EquippedTraitSource> {
    sources
        .iter()
        .map(|source| ((source.slot, kind_rank(source.kind)), source))
        .collect()
}

fn compare_snapshots(
    hook: &LocalEquipmentSnapshotEvent,
    external: &LocalEquipmentSnapshotEvent,
) -> Vec<SlotDifference> {
    let mut differences = Vec::new();
    if hook.character_type != external.character_type {
        differences.push(SlotDifference {
            slot: None,
            kind: None,
            field: DifferenceField::CharacterKey,
            hook: Some(hook.character_type),
            external: Some(external.character_type),
        });
    }

    let hook_sources = normalized_sources(&hook.sources);
    let external_sources = normalized_sources(&external.sources);
    let keys: std::collections::BTreeSet<_> = hook_sources
        .keys()
        .chain(external_sources.keys())
        .copied()
        .collect();
    for key in keys {
        let hook_source = hook_sources.get(&key).copied();
        let external_source = external_sources.get(&key).copied();
        let kind = hook_source.or(external_source).map(|source| source.kind);
        for (field, hook_value, external_value) in [
            (
                DifferenceField::ItemId,
                hook_source.map(|source| source.item_id),
                external_source.map(|source| source.item_id),
            ),
            (
                DifferenceField::TraitId,
                hook_source.map(|source| source.trait_id),
                external_source.map(|source| source.trait_id),
            ),
            (
                DifferenceField::TraitLevel,
                hook_source.map(|source| source.trait_level),
                external_source.map(|source| source.trait_level),
            ),
        ] {
            if hook_value != external_value {
                differences.push(SlotDifference {
                    slot: Some(key.0),
                    kind,
                    field,
                    hook: hook_value,
                    external: external_value,
                });
            }
        }
    }
    differences
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use equipment_core::{decode_snapshot, EMPTY_HASH, SIGIL_ARRAY_BYTES};

    use super::{CompareDecision, DeferredReason, DifferenceField, ProbeComparator};

    const KEY: u32 = 0xE705_3919;

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn fixture() -> Vec<u8> {
        let mut bytes = vec![0; SIGIL_ARRAY_BYTES];
        for slot in 0..12 {
            let base = slot * 0x24;
            put_u32(&mut bytes, base, EMPTY_HASH);
            put_u32(&mut bytes, base + 0x08, EMPTY_HASH);
            put_u32(&mut bytes, base + 0x10, EMPTY_HASH);
            put_u32(&mut bytes, base + 0x14, EMPTY_HASH);
        }
        put_u32(&mut bytes, 0, 0xDC58_4F60);
        put_u32(&mut bytes, 4, 15);
        put_u32(&mut bytes, 0x10, 0xEE73_2781);
        put_u32(&mut bytes, 0x14, KEY);
        bytes
    }

    #[test]
    fn defers_unstable_reads_then_matches_hook_truth() {
        let first = fixture();
        let mut changed = first.clone();
        put_u32(&mut changed, 4, 16);
        let now = Instant::now();
        let mut comparator = ProbeComparator::default();

        assert!(matches!(
            comparator.compare_external(KEY, &first, &changed, now),
            CompareDecision::Deferred(DeferredReason::UnstableRead)
        ));

        comparator.record_hook(decode_snapshot(&first, KEY).unwrap());
        assert!(matches!(
            comparator.compare_external(KEY, &first, &first, now),
            CompareDecision::Match(_)
        ));
    }

    #[test]
    fn reports_only_the_changed_slot_field_and_throttles_repeats() {
        let hook = fixture();
        let mut external = hook.clone();
        put_u32(&mut external, 4, 16);
        let now = Instant::now();
        let mut comparator = ProbeComparator::default();
        comparator.record_hook(decode_snapshot(&hook, KEY).unwrap());

        let CompareDecision::Mismatch(differences) =
            comparator.compare_external(KEY, &external, &external, now)
        else {
            panic!("expected mismatch");
        };
        assert_eq!(differences.len(), 1);
        assert_eq!(differences[0].slot, Some(0));
        assert_eq!(differences[0].field, DifferenceField::TraitLevel);
        assert_eq!(differences[0].hook, Some(15));
        assert_eq!(differences[0].external, Some(16));

        assert!(matches!(
            comparator.compare_external(KEY, &external, &external, now + Duration::from_secs(4)),
            CompareDecision::Suppressed
        ));
        assert!(matches!(
            comparator.compare_external(KEY, &external, &external, now + Duration::from_secs(5)),
            CompareDecision::Mismatch(_)
        ));
    }

    #[test]
    fn match_summary_exposes_only_a_short_digest() {
        let bytes = fixture();
        let now = Instant::now();
        let mut comparator = ProbeComparator::default();
        comparator.record_hook(decode_snapshot(&bytes, KEY).unwrap());

        let CompareDecision::Match(summary) = comparator.compare_external(KEY, &bytes, &bytes, now)
        else {
            panic!("expected match");
        };
        assert_eq!(summary.snapshot_digest.len(), 16);
        assert!(summary
            .snapshot_digest
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase()));
        assert!(!format!("{summary:?}").contains("raw"));
    }
}

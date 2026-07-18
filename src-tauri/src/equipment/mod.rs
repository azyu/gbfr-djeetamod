mod analyzer;

use std::collections::HashMap;

use protocol::{EquipmentCaptureStatus, LocalEquipmentSnapshotEvent};
use serde::{Deserialize, Serialize};

use self::analyzer::{analyze_sources, TraitAnalysis};

const TRAIT_CAP_CATALOG: &str = include_str!("../../assets/trait-caps.json");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TraitCapCatalog {
    #[serde(rename = "gameVersion")]
    _game_version: String,
    #[serde(rename = "gameExeSha256")]
    _game_exe_sha256: String,
    records: Vec<TraitCapRecord>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TraitCapRecord {
    trait_id: u32,
    max_level: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CharacterEquipmentStatus {
    Complete,
    Unsupported,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterEquipmentAnalysis {
    pub character_type: u32,
    pub status: CharacterEquipmentStatus,
    pub traits: Vec<TraitAnalysis>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EquipmentAnalysisResponse {
    pub connected: bool,
    pub characters: Vec<CharacterEquipmentAnalysis>,
}

pub struct EquipmentState {
    connected: bool,
    caps: HashMap<u32, u32>,
    snapshots: HashMap<u32, LocalEquipmentSnapshotEvent>,
}

impl EquipmentState {
    pub fn from_bundled_catalog() -> serde_json::Result<Self> {
        let catalog: TraitCapCatalog = serde_json::from_str(TRAIT_CAP_CATALOG)?;
        Ok(Self::with_caps(
            catalog
                .records
                .into_iter()
                .map(|record| (record.trait_id, record.max_level)),
        ))
    }

    fn with_caps(caps: impl IntoIterator<Item = (u32, u32)>) -> Self {
        Self {
            connected: false,
            caps: caps.into_iter().collect(),
            snapshots: HashMap::new(),
        }
    }

    #[cfg(test)]
    fn for_test(caps: impl IntoIterator<Item = (u32, u32)>) -> Self {
        Self::with_caps(caps)
    }

    pub fn connect(&mut self) {
        self.connected = true;
    }

    pub fn apply(&mut self, event: LocalEquipmentSnapshotEvent) {
        self.connected = true;
        self.snapshots.insert(event.character_type, event);
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
        self.snapshots.clear();
    }

    pub fn response(&self) -> EquipmentAnalysisResponse {
        let mut characters = self
            .snapshots
            .values()
            .map(|snapshot| {
                let analyses = match snapshot.status {
                    EquipmentCaptureStatus::Complete => {
                        analyze_sources(&snapshot.sources, &self.caps).ok()
                    }
                    EquipmentCaptureStatus::Unsupported => None,
                };
                CharacterEquipmentAnalysis {
                    character_type: snapshot.character_type,
                    status: if analyses.is_some() {
                        CharacterEquipmentStatus::Complete
                    } else {
                        CharacterEquipmentStatus::Unsupported
                    },
                    traits: analyses.unwrap_or_default(),
                }
            })
            .collect::<Vec<_>>();
        characters.sort_by_key(|character| character.character_type);
        EquipmentAnalysisResponse {
            connected: self.connected,
            characters,
        }
    }
}

#[cfg(test)]
mod tests {
    use protocol::{
        EquipmentCaptureStatus, EquipmentSourceKind, EquippedTraitSource,
        LocalEquipmentSnapshotEvent,
    };

    use super::EquipmentState;

    #[test]
    fn unsupported_snapshot_exposes_no_numeric_traits() {
        let mut state = EquipmentState::for_test([(1, 65)]);
        state.apply(LocalEquipmentSnapshotEvent {
            character_type: 10,
            status: EquipmentCaptureStatus::Unsupported,
            sources: Vec::new(),
        });

        let response = state.response();
        assert!(response.connected);
        assert!(response.characters[0].traits.is_empty());
    }

    #[test]
    fn disconnect_clears_every_character() {
        let mut state = EquipmentState::for_test([(1, 65)]);
        state.apply(LocalEquipmentSnapshotEvent {
            character_type: 10,
            status: EquipmentCaptureStatus::Complete,
            sources: Vec::new(),
        });

        state.disconnect();

        let response = state.response();
        assert!(!response.connected);
        assert!(response.characters.is_empty());
    }

    #[test]
    fn bundled_catalog_contains_the_verified_damage_cap() {
        let state = EquipmentState::from_bundled_catalog().unwrap();
        assert_eq!(state.caps.get(&0xDC58_4F60), Some(&65));
    }

    #[test]
    fn response_uses_frontend_safe_camel_case_enum_values() {
        let mut state = EquipmentState::for_test([(1, 65)]);
        state.apply(LocalEquipmentSnapshotEvent {
            character_type: 10,
            status: EquipmentCaptureStatus::Complete,
            sources: vec![EquippedTraitSource {
                kind: EquipmentSourceKind::SigilPrimary,
                slot: 0,
                item_id: 2,
                trait_id: 1,
                trait_level: 65,
            }],
        });

        let json = serde_json::to_value(state.response()).unwrap();
        assert_eq!(json["characters"][0]["status"], "complete");
        assert_eq!(json["characters"][0]["traits"][0]["state"], "capped");
        assert_eq!(
            json["characters"][0]["traits"][0]["sources"][0]["kind"],
            "sigilPrimary"
        );
    }
}

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use protocol::EquippedTraitSource;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TraitState {
    Overflow,
    Capped,
    Below,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraitAnalysis {
    pub trait_id: u32,
    pub total_level: u32,
    pub max_level: Option<u32>,
    pub overflow_level: u32,
    pub state: TraitState,
    pub sources: Vec<EquippedTraitSource>,
}

pub fn analyze_sources(
    sources: &[EquippedTraitSource],
    caps: &HashMap<u32, u32>,
) -> Result<Vec<TraitAnalysis>> {
    let mut grouped: HashMap<u32, (u32, Vec<EquippedTraitSource>)> = HashMap::new();
    for source in sources {
        let entry = grouped.entry(source.trait_id).or_default();
        entry.0 = entry
            .0
            .checked_add(source.trait_level)
            .ok_or_else(|| anyhow!("trait level total overflowed for {:#010x}", source.trait_id))?;
        entry.1.push(source.clone());
    }

    let mut analyses = grouped
        .into_iter()
        .map(|(trait_id, (total_level, sources))| {
            let max_level = caps.get(&trait_id).copied();
            let (state, overflow_level) = match max_level {
                Some(max_level) if total_level > max_level => {
                    (TraitState::Overflow, total_level - max_level)
                }
                Some(max_level) if total_level == max_level => (TraitState::Capped, 0),
                Some(_) => (TraitState::Below, 0),
                None => (TraitState::Unknown, 0),
            };
            TraitAnalysis {
                trait_id,
                total_level,
                max_level,
                overflow_level,
                state,
                sources,
            }
        })
        .collect::<Vec<_>>();

    analyses.sort_by_key(|analysis| (state_rank(analysis.state), analysis.trait_id));
    Ok(analyses)
}

fn state_rank(state: TraitState) -> u8 {
    match state {
        TraitState::Overflow => 0,
        TraitState::Capped => 1,
        TraitState::Below => 2,
        TraitState::Unknown => 3,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use protocol::{EquipmentSourceKind, EquippedTraitSource};

    use super::{analyze_sources, TraitState};

    fn source(trait_id: u32, trait_level: u32) -> EquippedTraitSource {
        EquippedTraitSource {
            kind: EquipmentSourceKind::SigilPrimary,
            slot: 0,
            item_id: 1,
            trait_id,
            trait_level,
        }
    }

    #[test]
    fn classifies_all_states_and_sorts_overflow_first() {
        let caps = HashMap::from([(1, 65), (2, 50), (3, 45)]);
        let sources = vec![
            source(1, 60),
            source(1, 12),
            source(2, 50),
            source(3, 30),
            source(4, 15),
        ];

        let results = analyze_sources(&sources, &caps).unwrap();
        assert_eq!(results[0].trait_id, 1);
        assert_eq!(results[0].state, TraitState::Overflow);
        assert_eq!(results[0].overflow_level, 7);
        assert_eq!(results[1].state, TraitState::Capped);
        assert_eq!(results[2].state, TraitState::Below);
        assert_eq!(results[3].state, TraitState::Unknown);
    }

    #[test]
    fn rejects_a_total_that_cannot_fit_in_u32() {
        let sources = vec![source(1, u32::MAX), source(1, 1)];
        assert!(analyze_sources(&sources, &HashMap::new()).is_err());
    }
}

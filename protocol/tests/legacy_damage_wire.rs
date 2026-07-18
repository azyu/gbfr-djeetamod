use protocol::{ActionType, Actor, Message};
use serde::Serialize;

#[derive(Serialize)]
struct LegacyDamageEvent {
    source: Actor,
    target: Actor,
    damage: i32,
    flags: u64,
    action_id: ActionType,
    attack_rate: Option<f32>,
    stun_value: Option<f32>,
    damage_cap: Option<i32>,
}

#[allow(dead_code)]
#[derive(Serialize)]
enum LegacyMessage {
    OnAreaEnter(()),
    OnQuestComplete(()),
    DamageEvent(LegacyDamageEvent),
}

fn actor(index: u32) -> Actor {
    Actor {
        index,
        actor_type: 0x1234,
        parent_index: index,
        parent_actor_type: 0x1234,
    }
}

#[test]
fn current_parser_accepts_a_legacy_damage_frame() {
    let legacy = LegacyMessage::DamageEvent(LegacyDamageEvent {
        source: actor(1),
        target: actor(2),
        damage: 123_456,
        flags: 0x20000,
        action_id: ActionType::Normal(101),
        attack_rate: None,
        stun_value: None,
        damage_cap: Some(100_000),
    });

    let bytes = protocol::bincode::serialize(&legacy).unwrap();
    let decoded = protocol::deserialize_message(&bytes)
        .expect("the current app must continue decoding 1.8.4 damage frames");

    let Message::DamageEvent(event) = decoded else {
        panic!("legacy frame decoded as the wrong message variant");
    };
    assert_eq!(event.damage, 123_456);
    assert!(event.details.is_none());
}

#[test]
fn compatibility_decoder_preserves_current_damage_details() {
    let current = Message::DamageEvent(protocol::DamageEvent {
        source: actor(1),
        target: actor(2),
        damage: 123_456,
        flags: 0x20000,
        action_id: ActionType::Normal(101),
        attack_rate: None,
        stun_value: Some(12.5),
        damage_cap: Some(100_000),
        details: Some(protocol::DamageDetails {
            elemental_multiplier: 1.2,
            amplify_multiplier: 1.1,
            defense_multiplier: 1.0,
            attack_multiplier: 1.0,
            supplementary_multiplier: 1.0,
            formula_multiplier: 1.32,
            attack_rate: 2.0,
            uncapped_damage: 150_000.0,
            damage_cap: 100_000,
            damage_limit_multiplier: 1.0,
            statuses: Vec::new(),
        }),
    });

    let bytes = protocol::bincode::serialize(&current).unwrap();
    let decoded = protocol::deserialize_message(&bytes).unwrap();

    let Message::DamageEvent(event) = decoded else {
        panic!("current frame decoded as the wrong message variant");
    };
    assert_eq!(event.stun_value, Some(12.5));
    assert!(event.details.is_some());
}

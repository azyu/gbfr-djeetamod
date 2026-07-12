use std::ptr::NonNull;

use anyhow::{anyhow, Result};
use protocol::{ActionType, Actor, DamageEvent, Message, PlayerIdentityEvent};
use retour::static_detour;

use crate::{event, hooks::ffi::DamageInstance, process::Process};

use super::{actor_idx, actor_type_id, get_source_parent};

type ProcessDamageEventFunc =
    unsafe extern "system" fn(*const usize, *const usize, *const usize, u8) -> usize;

type ProcessDotEventFunc = unsafe extern "system" fn(*const usize, *const usize) -> usize;

static_detour! {
    static ProcessDamageEvent: unsafe extern "system" fn(*const usize, *const usize, *const usize, u8) -> usize;
    static ProcessDotEvent: unsafe extern "system" fn(*const usize, *const usize) -> usize;
}

#[derive(Clone)]
pub struct OnProcessDamageHook {
    tx: event::Tx,
}

const PROCESS_DAMAGE_EVENT_SIG: &str = "e8 $ { ' } 66 83 bc 24 ? ? ? ? ?";

const ID_HUMAN_TYPE: u32 = 0x8056ABCD;
const ID_DRAGON_TYPE: u32 = 0xF5755C0E;
const PLAYER_ACTOR_ID_BASE: u32 = 0xFFFF_FF00;

fn stable_player_actor_id(party_index: u8) -> u32 {
    PLAYER_ACTOR_ID_BASE | u32::from(party_index)
}

fn canonical_player_type(character_type: u32) -> u32 {
    if character_type == ID_DRAGON_TYPE {
        ID_HUMAN_TYPE
    } else {
        character_type
    }
}

/// Converts concrete player actors into a party-slot identity. Concrete actor
/// type/index remain on DamageEvent::source so transformed skills still retain
/// their own breakdown, while parent identity remains stable across forms.
fn normalize_player_identities(
    identities: Vec<PlayerIdentityEvent>,
    source_index: u32,
) -> (Vec<PlayerIdentityEvent>, Option<(u32, u32)>) {
    let mut source_parent = None;
    let identities = identities
        .into_iter()
        .map(|mut identity| {
            let concrete_actor_index = identity.actor_index;
            let parent_actor_type = canonical_player_type(identity.character_type);
            let parent_index = stable_player_actor_id(identity.party_index);

            if concrete_actor_index == source_index {
                source_parent = Some((parent_actor_type, parent_index));
            }

            identity.character_type = parent_actor_type;
            identity.actor_index = parent_index;
            identity
        })
        .collect();

    (identities, source_parent)
}

impl OnProcessDamageHook {
    pub fn new(tx: event::Tx) -> Self {
        OnProcessDamageHook { tx }
    }

    pub fn setup(&self, process: &Process) -> Result<()> {
        let cloned_self = self.clone();

        if let Ok(process_dmg_evt) = process.search_address(PROCESS_DAMAGE_EVENT_SIG) {
            #[cfg(feature = "console")]
            println!("Found process dmg event");

            unsafe {
                let func: ProcessDamageEventFunc = std::mem::transmute(process_dmg_evt);

                ProcessDamageEvent
                    .initialize(func, move |a1, a2, a3, a4| cloned_self.run(a1, a2, a3, a4))?;

                ProcessDamageEvent.enable()?;
            }
        } else {
            return Err(anyhow!("Could not find process_dmg_evt"));
        }

        Ok(())
    }

    fn run(&self, a1: *const usize, a2: *const usize, a3: *const usize, a4: u8) -> usize {
        // Target is the instance of the actor being damaged.
        // For example: Instance of the Em2700 class.
        let target_specified_instance_ptr: usize = unsafe { *(*a1.byte_add(0x08) as *const usize) };

        let original_value = unsafe { ProcessDamageEvent.call(a1, a2, a3, a4) };

        // This points to the first Entity instance in the 'a2' entity list.
        let source_entity_ptr = unsafe { (a2.byte_add(0x18) as *const *const usize).read() };

        // @TODO(false): For some reason, online + Ferry's Umlauf skill pet can return a null pointer here.
        // Possible data race with online?
        if source_entity_ptr.is_null() {
            return original_value;
        }

        // entity->m_pSpecifiedInstance, offset 0x70 from entity pointer.
        // Returns the specific class instance of the source entity. (e.g. Instance of Pl1200 / Pl0700Ghost)
        let source_specified_instance_ptr: usize = unsafe { *(source_entity_ptr.byte_add(0x70)) };

        let damage_instance = unsafe { NonNull::new(a2 as *mut DamageInstance).unwrap().as_ref() };
        let damage: i32 = damage_instance.damage;

        if original_value == 0 || damage <= 0 {
            return original_value;
        }

        let flags: u64 = damage_instance.flags;

        let action_type: ActionType = if ((1 << 7 | 1 << 50) & flags) != 0 {
            ActionType::LinkAttack
        } else if ((1 << 13 | 1 << 14) & flags) != 0 {
            ActionType::SBA
        } else if ((1 << 15) & flags) != 0 {
            ActionType::SupplementaryDamage(damage_instance.action_id)
        } else {
            ActionType::Normal(damage_instance.action_id)
        };

        // Get the source actor's type ID.
        let source_type_id = actor_type_id(source_specified_instance_ptr as *const usize);
        let source_idx = actor_idx(source_specified_instance_ptr as *const usize);

        let identities = super::player::identity_events_for_actor(
            source_specified_instance_ptr as *const usize,
            source_type_id,
            source_idx,
        );
        let (identities, stable_source_parent) =
            normalize_player_identities(identities, source_idx);

        for identity in identities {
            let _ = self.tx.send(Message::PlayerIdentityEvent(identity));
        }

        // Parent layouts are character-specific and changed in the 2.0 update.
        // Player identity snapshots expose a safe party slot, so use that as a
        // stable parent without dereferencing the old form-specific offsets.
        let (source_parent_type_id, source_parent_idx) =
            stable_source_parent.unwrap_or((source_type_id, source_idx));

        let target_type_id: u32 = actor_type_id(target_specified_instance_ptr as *const usize);
        let target_idx = actor_idx(target_specified_instance_ptr as *const usize);

        let event = Message::DamageEvent(DamageEvent {
            source: Actor {
                index: source_idx,
                actor_type: source_type_id,
                parent_index: source_parent_idx,
                parent_actor_type: source_parent_type_id,
            },
            target: Actor {
                index: target_idx,
                actor_type: target_type_id,
                parent_index: target_idx,
                parent_actor_type: target_type_id,
            },
            damage,
            flags,
            action_id: action_type,
            attack_rate: None,
            damage_cap: Some(damage_instance.damage_cap),
            stun_value: None,
        });

        let _ = self.tx.send(event);

        original_value
    }
}

#[derive(Clone)]
pub struct OnProcessDotHook {
    tx: event::Tx,
}

impl OnProcessDotHook {
    pub fn new(tx: event::Tx) -> Self {
        OnProcessDotHook { tx }
    }

    pub fn setup(&self, process: &Process) -> Result<()> {
        let cloned_self = self.clone();

        if let Ok(process_dot_evt) =
            process.search_address("44 89 74 24 ? 48 ? ? ? ? 48 ? ? e8 $ { ' } 4c")
        {
            #[cfg(feature = "console")]
            println!("Found process dot event");

            unsafe {
                let func: ProcessDotEventFunc = std::mem::transmute(process_dot_evt);
                ProcessDotEvent.initialize(func, move |a1, a2| cloned_self.run(a1, a2))?;
                ProcessDotEvent.enable()?;
            }
        } else {
            return Err(anyhow!("Could not find process_dot_evt"));
        }

        Ok(())
    }

    // A1: DoT Instance (StatusPl2300ParalysisArrow)
    // *A1+0x00 -> StatusAilmentPoison : StatusBase
    // A1+0x18->targetEntityInfo : CEntityInfo (Target entity of the DoT, what is being damaged)
    // A1+0x30->sourceEntityInfo : CEntityInfo (Source entity of the DoT, who applied it)
    // A1+0x50->duration : float (How much time is left for the DoT)
    fn run(&self, dot_instance: *const usize, a2: *const usize) -> usize {
        let original_value = unsafe { ProcessDotEvent.call(dot_instance, a2) };

        // @TODO(false): There's a better way to check null pointers with Option type, but I'm too dumb to figure it out right now.
        let target_info = unsafe { dot_instance.byte_add(0x18).read() } as *const usize;
        let source_info = unsafe { dot_instance.byte_add(0x30).read() } as *const usize;

        if target_info.is_null() || source_info.is_null() {
            return original_value;
        }

        let target = unsafe { target_info.byte_add(0x70).read() } as *const usize;
        let source = unsafe { source_info.byte_add(0x70).read() } as *const usize;

        if target.is_null() || source.is_null() {
            return original_value;
        }

        let dmg = unsafe { (a2 as *const i32).read() };

        let source_idx = actor_idx(source);
        let source_type_id = actor_type_id(source);

        let identities =
            super::player::identity_events_for_actor(source, source_type_id, source_idx);
        let (identities, stable_source_parent) =
            normalize_player_identities(identities, source_idx);

        for identity in identities {
            let _ = self.tx.send(Message::PlayerIdentityEvent(identity));
        }

        let target_idx = actor_idx(target);
        let target_type_id = actor_type_id(target);

        let (source_parent_type_id, source_parent_idx) =
            stable_source_parent.unwrap_or_else(|| {
                get_source_parent(source_type_id, source).unwrap_or((source_type_id, source_idx))
            });

        let event = Message::DamageEvent(DamageEvent {
            source: Actor {
                index: source_idx,
                actor_type: source_type_id,
                parent_index: source_parent_idx,
                parent_actor_type: source_parent_type_id,
            },
            target: Actor {
                index: target_idx,
                actor_type: target_type_id,
                parent_index: target_idx,
                parent_actor_type: target_type_id,
            },
            damage: dmg,
            flags: 0,
            action_id: ActionType::DamageOverTime(0),
            attack_rate: None,
            stun_value: None,
            damage_cap: None,
        });

        let _ = self.tx.send(event);

        original_value
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use protocol::PlayerIdentityEvent;

    use super::{
        normalize_player_identities, stable_player_actor_id, ID_DRAGON_TYPE, ID_HUMAN_TYPE,
    };

    fn identity(character_type: u32, actor_index: u32, party_index: u8) -> PlayerIdentityEvent {
        PlayerIdentityEvent {
            character_name: CString::new("Id").unwrap(),
            display_name: CString::new("Player").unwrap(),
            character_type,
            party_index,
            actor_index,
            is_online: true,
        }
    }

    #[test]
    fn id_forms_share_a_stable_player_parent() {
        let (human_identities, human_parent) =
            normalize_player_identities(vec![identity(ID_HUMAN_TYPE, 10, 2)], 10);
        let (dragon_identities, dragon_parent) =
            normalize_player_identities(vec![identity(ID_DRAGON_TYPE, 11, 2)], 11);

        let expected_parent = (ID_HUMAN_TYPE, stable_player_actor_id(2));
        assert_eq!(human_parent, Some(expected_parent));
        assert_eq!(dragon_parent, Some(expected_parent));
        assert_eq!(human_identities[0].character_type, ID_HUMAN_TYPE);
        assert_eq!(dragon_identities[0].character_type, ID_HUMAN_TYPE);
        assert_eq!(human_identities[0].actor_index, expected_parent.1);
        assert_eq!(dragon_identities[0].actor_index, expected_parent.1);
    }
}

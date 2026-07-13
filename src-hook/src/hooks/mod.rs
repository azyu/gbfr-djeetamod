use std::{
    collections::HashMap,
    ffi::c_void,
    mem::MaybeUninit,
    sync::{Mutex, OnceLock},
};

use anyhow::Result;
use log::{info, warn};
use windows::Win32::{Foundation::HANDLE, System::Diagnostics::Debug::ReadProcessMemory};

use crate::{event, process::Process};

use self::{damage::OnProcessDamageHook, player::OnLoadPlayerIdentityHook, quest::OnBattleEndHook};

mod area;
mod damage;
mod damage_details;
mod death;
mod ffi;
mod globals;
mod player;
mod quest;
mod sba;

type GetEntityHashID0x58 = unsafe extern "system" fn(*const usize, *const u32) -> *const usize;

const ID_HUMAN_TYPE: u32 = 0x8056ABCD;
const ID_DRAGON_TYPE: u32 = 0xF5755C0E;
const ID_DRAGON_PARENT_ENTITY_OFFSET: usize = 0x1CA98;

/// Game 2.0 removed the party index from the offset used by older releases. Keep a
/// process-local ID for every concrete actor instance instead. Two players using the
/// same character still have separate specified-instance pointers.
#[derive(Default)]
struct ActorIds {
    by_instance: HashMap<usize, u32>,
    next_id: u32,
}

static ACTOR_IDS: OnceLock<Mutex<ActorIds>> = OnceLock::new();

pub fn setup_hooks(tx: event::Tx) -> Result<()> {
    let process = Process::with_name("granblue_fantasy_relink.exe")?;

    match damage_details::setup(&process) {
        Ok(()) => info!("Detailed damage hooks enabled"),
        Err(error) => warn!("Detailed damage hooks unavailable: {error}"),
    }

    // Core DPS tracking. The main damage signature is still stable in game 2.0.2.
    OnProcessDamageHook::new(tx.clone()).setup(&process)?;

    // Game 2.0.2 still keeps player names in the per-actor identity snapshot, but
    // the function that refreshes it moved. This hook deliberately reads only the
    // stable identity fields; equipment remains disabled until its layout is known.
    OnLoadPlayerIdentityHook::new(tx.clone()).setup(&process)?;

    // This hooks the actual reward/result setup rather than the generic result
    // input operation, which is also reused by fall recovery and boss mechanics.
    match OnBattleEndHook::new(tx).setup(&process) {
        Ok(()) => info!("Game 2.0.2 result reward hook enabled"),
        Err(error) => warn!("Battle-end hook unavailable; using inactivity fallback: {error}"),
    }

    // The 2.0 update changed the layouts and signatures used by the auxiliary hooks.
    // Keep them disabled until each one has been independently verified; installing a
    // stale hook is much worse than temporarily omitting encounter metadata.
    warn!("Running in game 2.0 compatibility mode: equipment and auxiliary hooks are disabled");

    Ok(())
}

#[inline(always)]
pub unsafe fn v_func<T: Sized>(ptr: *const usize, offset: usize) -> T {
    ((ptr.read() as *const usize).byte_add(offset) as *const T).read()
}

#[inline(always)]
pub fn actor_type_id(actor_ptr: *const usize) -> u32 {
    let mut type_id: u32 = 0;

    unsafe {
        v_func::<GetEntityHashID0x58>(actor_ptr, 0x58)(actor_ptr, &mut type_id as *mut u32);
    }

    type_id
}

#[inline(always)]
pub fn actor_idx(actor_ptr: *const usize) -> u32 {
    let mut actor_ids = ACTOR_IDS
        .get_or_init(|| Mutex::new(ActorIds::default()))
        .lock()
        .expect("actor ID map lock poisoned");

    let instance = actor_ptr as usize;

    if let Some(id) = actor_ids.by_instance.get(&instance) {
        return *id;
    }

    let id = actor_ids.next_id;
    actor_ids.next_id = actor_ids.next_id.wrapping_add(1);
    actor_ids.by_instance.insert(instance, id);
    id
}

// Returns the parent entity of the source entity if necessary.
#[inline(always)]
pub fn get_source_parent(source_type_id: u32, source: *const usize) -> Option<(u32, u32)> {
    match source_type_id {
        // Pl0700Ghost -> Pl0700
        0x2AF678E8 => {
            let parent_instance = parent_specified_instance_at(source, 0xE48)?;

            Some((actor_type_id(parent_instance), actor_idx(parent_instance)))
        }
        // Pl0700GhostSatellite -> Pl0700
        0x8364C8BC => {
            let parent_instance = parent_specified_instance_at(source, 0x508)?;

            Some((actor_type_id(parent_instance), actor_idx(parent_instance)))
        }
        // Wp1890: Cagliostro's Ouroboros Dragon Sled -> Pl1800
        0xC9F45042 => {
            let parent_instance = parent_specified_instance_at(source, 0x578)?;
            Some((actor_type_id(parent_instance), actor_idx(parent_instance)))
        }
        // Pl2000: Id's Dragon Form -> Pl1900
        ID_DRAGON_TYPE => {
            let parent_instance =
                parent_specified_instance_at(source, ID_DRAGON_PARENT_ENTITY_OFFSET)?;

            // Pl2000 is always a transformed Pl1900. Do not call a virtual
            // function through this cross-object pointer: the known concrete
            // parent type plus the safe pointer reads below avoid turning a
            // stale game layout into an access violation inside the hook.
            Some((ID_HUMAN_TYPE, actor_idx(parent_instance)))
        }
        // Wp2290: Seofon's Avatar
        0x5B1AB457 => {
            let parent_instance = parent_specified_instance_at(source, 0x500)?;
            Some((actor_type_id(parent_instance), actor_idx(parent_instance)))
        }
        // Pl0600PlantRose
        0x69C0CA71 => {
            let parent_instance = parent_specified_instance_at(source, 0x7E0)?;
            Some((actor_type_id(parent_instance), actor_idx(parent_instance)))
        }
        _ => None,
    }
}

// Returns the specified instance of the parent entity.
// ptr+offset: Entity
// *(ptr+offset) + 0x70: m_pSpecifiedInstance (Pl0700, Pl1200, etc.)
#[inline(always)]
fn parent_specified_instance_at(actor_ptr: *const usize, offset: usize) -> Option<*const usize> {
    let entity = read_process_value::<*const usize>(actor_ptr.wrapping_byte_add(offset).cast())?;
    if entity.is_null() {
        return None;
    }

    let parent = read_process_value::<*const usize>(entity.wrapping_byte_add(0x70).cast())?;
    (!parent.is_null()).then_some(parent)
}

/// Reads hook-owned game memory without letting an invalid pointer raise an
/// in-process access violation. This is intentionally used for version-fragile
/// cross-object links only; a failed or partial read simply disables grouping.
pub(super) fn read_process_value<T: Copy>(address: *const T) -> Option<T> {
    if address.is_null() {
        return None;
    }

    let mut value = MaybeUninit::<T>::uninit();
    let mut bytes_read = 0usize;
    let result = unsafe {
        ReadProcessMemory(
            HANDLE(-1),
            address.cast::<c_void>(),
            value.as_mut_ptr().cast::<c_void>(),
            std::mem::size_of::<T>(),
            Some(&mut bytes_read),
        )
    };

    if result.is_err() || bytes_read != std::mem::size_of::<T>() {
        return None;
    }

    Some(unsafe { value.assume_init() })
}

#[cfg(test)]
mod tests {
    use super::{actor_idx, parent_specified_instance_at, ID_DRAGON_PARENT_ENTITY_OFFSET};

    #[test]
    fn concrete_actor_instances_receive_distinct_ids() {
        let first = 0x1000usize as *const usize;
        let second = 0x2000usize as *const usize;

        assert_eq!(actor_idx(first), actor_idx(first));
        assert_ne!(actor_idx(first), actor_idx(second));
    }

    #[test]
    fn safely_reads_parent_specified_instance() {
        let parent = Box::new(0usize);
        let mut entity = vec![0u8; 0x78];
        let mut actor = vec![0u8; ID_DRAGON_PARENT_ENTITY_OFFSET + std::mem::size_of::<usize>()];
        let parent_ptr = (&*parent as *const usize).cast::<usize>();
        let entity_ptr = entity.as_ptr().cast::<usize>();

        unsafe {
            entity
                .as_mut_ptr()
                .byte_add(0x70)
                .cast::<*const usize>()
                .write_unaligned(parent_ptr);
            actor
                .as_mut_ptr()
                .byte_add(ID_DRAGON_PARENT_ENTITY_OFFSET)
                .cast::<*const usize>()
                .write_unaligned(entity_ptr);
        }

        assert_eq!(
            parent_specified_instance_at(
                actor.as_ptr().cast::<usize>(),
                ID_DRAGON_PARENT_ENTITY_OFFSET,
            ),
            Some(parent_ptr)
        );
    }

    #[test]
    fn invalid_parent_address_fails_without_dereferencing_it() {
        assert_eq!(
            parent_specified_instance_at(1usize as *const usize, 0),
            None
        );
    }
}

use std::{
    collections::HashMap,
    ffi::{c_void, CString},
    sync::{Mutex, OnceLock},
};

use anyhow::{anyhow, Result};
use log::info;
use protocol::PlayerIdentityEvent;
use retour::static_detour;
use windows::Win32::{Foundation::HANDLE, System::Diagnostics::Debug::ReadProcessMemory};

use crate::{event, process::Process};

type RefreshPlayerIdentityFunc = unsafe extern "system" fn(*const usize);

static_detour! {
    static RefreshPlayerIdentity: unsafe extern "system" fn(*const usize);
}

/// Offset of the 0x250-byte player identity snapshot in a game 2.0.2 player
/// record. This record is not an actor and must never be passed to actor vfuncs.
const PLAYER_IDENTITY_OFFSET: usize = 0x5E60;
const PLAYER_KEY_OFFSET: usize = 0x5EA8;
const IS_ONLINE_OFFSET: usize = 0x1C8;
const CHARACTER_NAME_OFFSET: usize = 0x1E8;
const DISPLAY_NAME_OFFSET: usize = 0x208;
const PARTY_INDEX_OFFSET: usize = 0x22C;
const VBUFFER_INLINE_CAPACITY: usize = 0x0F;
const MAX_PLAYER_NAME_BYTES: usize = 0x100;
const INVALID_PLAYER_KEY: u32 = 0x887A_E0B0;
/// The owning player's key inside a concrete game 2.0.2 player actor.
/// This offset was identical for every local and online actor observed in
/// combat. Reading only this field avoids matching party-wide keys that also
/// occur elsewhere in the actor allocation.
const ACTOR_PLAYER_KEY_OFFSET: usize = 0x1AB40;

/// Unique game 2.0.2 prologue for the function that rebuilds the player
/// identity snapshot. The hook only copies metadata from the record.
const REFRESH_PLAYER_IDENTITY_SIG: &str =
    "55 41 57 41 56 41 54 56 57 53 48 83 ec 70 48 8d 6c 24 70 48 c7 45 f8 fe ff ff ff 80 b9 bc 5e 00 00 00";

#[derive(Clone, Debug)]
struct StoredPlayerIdentity {
    character_name: CString,
    display_name: CString,
    party_index: u8,
    is_online: bool,
}

#[derive(Default)]
struct IdentityStore {
    by_key: HashMap<u32, StoredPlayerIdentity>,
    active_key_by_party: HashMap<u8, u32>,
}

impl IdentityStore {
    fn insert(&mut self, player_key: u32, identity: StoredPlayerIdentity) -> bool {
        let party_index = identity.party_index;
        let previous_key = self.active_key_by_party.insert(party_index, player_key);
        if let Some(previous_key) = previous_key.filter(|key| *key != player_key) {
            self.by_key.remove(&previous_key);
        }
        self.by_key.insert(player_key, identity);

        previous_key != Some(player_key)
    }
}

static IDENTITIES: OnceLock<Mutex<IdentityStore>> = OnceLock::new();
static ACTOR_KEYS: OnceLock<Mutex<HashMap<usize, u32>>> = OnceLock::new();

#[derive(Clone)]
pub struct OnLoadPlayerIdentityHook {
    #[allow(dead_code)]
    tx: event::Tx,
}

impl OnLoadPlayerIdentityHook {
    pub fn new(tx: event::Tx) -> Self {
        Self { tx }
    }

    pub fn setup(&self, process: &Process) -> Result<()> {
        let refresh_player_identity = process
            .search_match_address(REFRESH_PLAYER_IDENTITY_SIG)
            .map_err(|_| anyhow!("Could not find refresh_player_identity"))?;
        let cloned_self = self.clone();

        unsafe {
            let func: RefreshPlayerIdentityFunc = std::mem::transmute(refresh_player_identity);
            RefreshPlayerIdentity.initialize(func, move |record| cloned_self.run(record))?;
            RefreshPlayerIdentity.enable()?;
        }

        Ok(())
    }

    fn run(&self, record: *const usize) {
        unsafe { RefreshPlayerIdentity.call(record) };

        if record.is_null() {
            return;
        }

        let snapshot = unsafe {
            (record.byte_add(PLAYER_IDENTITY_OFFSET) as *const *const u8).read_unaligned()
        };
        let player_key = unsafe {
            record
                .byte_add(PLAYER_KEY_OFFSET)
                .cast::<u32>()
                .read_unaligned()
        };

        if player_key == 0 || player_key == INVALID_PLAYER_KEY {
            return;
        }

        let Some(identity) = (unsafe { read_player_identity(snapshot) }) else {
            return;
        };

        // Before an online party is fully populated, the game creates
        // placeholder records for slots 1-3 using the local profile name.
        // They are AI/offline slots, not real remote player identities.
        if !should_cache_identity(&identity) {
            return;
        }

        info!(
            "Player identity cached: key={player_key:#010x}, party={}, online={}, name={}",
            identity.party_index,
            identity.is_online,
            identity.display_name.to_string_lossy()
        );

        let mapping_changed = {
            let mut identities = IDENTITIES
                .get_or_init(|| Mutex::new(IdentityStore::default()))
                .lock()
                .expect("player identity map lock poisoned");
            identities.insert(player_key, identity)
        };

        // Actor allocations can be reused while a lobby is changing from its
        // offline placeholders to the real online party. Force the next hit to
        // read the actor's current key after any slot mapping change.
        if mapping_changed {
            ACTOR_KEYS
                .get_or_init(|| Mutex::new(HashMap::new()))
                .lock()
                .expect("actor identity map lock poisoned")
                .clear();
        }
    }
}

fn should_cache_identity(identity: &StoredPlayerIdentity) -> bool {
    identity.party_index == 0 || identity.is_online
}

/// Resolves a cached identity against the concrete actor used by the damage
/// hook. ReadProcessMemory turns an invalid or short actor range into a failed
/// read instead of an in-process access violation.
pub fn identity_event_for_actor(
    actor: *const usize,
    character_type: u32,
    actor_index: u32,
) -> Option<PlayerIdentityEvent> {
    if actor.is_null() {
        return None;
    }

    let actor_address = actor as usize;
    let cached_key = ACTOR_KEYS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("actor identity map lock poisoned")
        .get(&actor_address)
        .copied();

    let (player_key, identity) = if let Some(player_key) = cached_key {
        let identity = IDENTITIES
            .get_or_init(|| Mutex::new(IdentityStore::default()))
            .lock()
            .expect("player identity map lock poisoned")
            .by_key
            .get(&player_key)
            .cloned()?;
        (player_key, identity)
    } else {
        let player_key = read_actor_player_key(actor)?;
        let identity = IDENTITIES
            .get_or_init(|| Mutex::new(IdentityStore::default()))
            .lock()
            .expect("player identity map lock poisoned")
            .by_key
            .get(&player_key)
            .cloned()?;

        info!(
            "Player actor matched: actor={actor:p}, type={character_type:#010x}, key={player_key:#010x}, offset={ACTOR_PLAYER_KEY_OFFSET:#x}, name={}",
            identity.display_name.to_string_lossy()
        );

        ACTOR_KEYS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .expect("actor identity map lock poisoned")
            .insert(actor_address, player_key);

        (player_key, identity)
    };

    let _ = player_key;

    Some(PlayerIdentityEvent {
        character_name: identity.character_name,
        display_name: identity.display_name,
        character_type,
        party_index: identity.party_index,
        actor_index,
        is_online: identity.is_online,
    })
}

fn read_actor_player_key(actor: *const usize) -> Option<u32> {
    let mut player_key = 0u32;
    let mut bytes_read = 0usize;
    let result = unsafe {
        ReadProcessMemory(
            HANDLE(-1),
            actor.byte_add(ACTOR_PLAYER_KEY_OFFSET).cast::<c_void>(),
            (&mut player_key as *mut u32).cast::<c_void>(),
            std::mem::size_of::<u32>(),
            Some(&mut bytes_read),
        )
    };

    if result.is_err()
        || bytes_read != std::mem::size_of::<u32>()
        || player_key == 0
        || player_key == INVALID_PLAYER_KEY
    {
        return None;
    }

    Some(player_key)
}

unsafe fn read_player_identity(snapshot: *const u8) -> Option<StoredPlayerIdentity> {
    if snapshot.is_null() {
        return None;
    }

    let is_online = snapshot
        .byte_add(IS_ONLINE_OFFSET)
        .cast::<u32>()
        .read_unaligned();
    let party_index = snapshot
        .byte_add(PARTY_INDEX_OFFSET)
        .cast::<u32>()
        .read_unaligned();

    if is_online > 1 || party_index > 3 {
        return None;
    }

    let display_name = read_vbuffer(snapshot.byte_add(DISPLAY_NAME_OFFSET))?;

    if display_name.as_bytes().is_empty() {
        return None;
    }

    let character_name = read_vbuffer(snapshot.byte_add(CHARACTER_NAME_OFFSET))
        .unwrap_or_else(|| CString::new("").expect("empty CString is valid"));

    Some(StoredPlayerIdentity {
        character_name,
        display_name,
        party_index: party_index as u8,
        is_online: is_online != 0,
    })
}

unsafe fn read_vbuffer(buffer: *const u8) -> Option<CString> {
    let used_size = buffer.byte_add(0x10).cast::<usize>().read_unaligned();
    let max_size = buffer.byte_add(0x18).cast::<usize>().read_unaligned();

    if used_size > MAX_PLAYER_NAME_BYTES || max_size < used_size || max_size > 0x1000 {
        return None;
    }

    let bytes_ptr = if max_size > VBUFFER_INLINE_CAPACITY {
        buffer.cast::<*const u8>().read_unaligned()
    } else {
        buffer
    };

    if bytes_ptr.is_null() {
        return None;
    }

    let bytes = std::slice::from_raw_parts(bytes_ptr, used_size);
    std::str::from_utf8(bytes).ok()?;
    CString::new(bytes).ok()
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use super::{
        read_vbuffer, should_cache_identity, IdentityStore, StoredPlayerIdentity,
        ACTOR_PLAYER_KEY_OFFSET,
    };

    fn identity(name: &str, party_index: u8, is_online: bool) -> StoredPlayerIdentity {
        StoredPlayerIdentity {
            character_name: CString::new("").unwrap(),
            display_name: CString::new(name).unwrap(),
            party_index,
            is_online,
        }
    }

    #[test]
    fn reads_inline_utf8_player_name() {
        let mut buffer = [0u8; 0x20];
        let name = "芙劳玩家".as_bytes();
        buffer[..name.len()].copy_from_slice(name);
        buffer[0x10..0x18].copy_from_slice(&name.len().to_ne_bytes());
        buffer[0x18..0x20].copy_from_slice(&0x0Fusize.to_ne_bytes());

        let value = unsafe { read_vbuffer(buffer.as_ptr()) }.expect("valid VBuffer");
        assert_eq!(value.to_str().unwrap(), "芙劳玩家");
    }

    #[test]
    fn rejects_unreasonably_large_player_name() {
        let mut buffer = [0u8; 0x20];
        buffer[0x10..0x18].copy_from_slice(&0x101usize.to_ne_bytes());
        buffer[0x18..0x20].copy_from_slice(&0x101usize.to_ne_bytes());

        assert!(unsafe { read_vbuffer(buffer.as_ptr()) }.is_none());
    }

    #[test]
    fn uses_verified_actor_player_key_offset() {
        assert_eq!(ACTOR_PLAYER_KEY_OFFSET, 0x1AB40);
    }

    #[test]
    fn rejects_remote_offline_placeholder_identity() {
        assert!(!should_cache_identity(&identity("Local Player", 1, false)));
        assert!(should_cache_identity(&identity("Local Player", 0, false)));
        assert!(should_cache_identity(&identity("Remote Player", 1, true)));
    }

    #[test]
    fn replacing_party_slot_removes_stale_player_key() {
        let mut identities = IdentityStore::default();
        assert!(identities.insert(0x1111, identity("Placeholder", 2, true)));
        assert!(identities.insert(0x2222, identity("Remote Player", 2, true)));

        assert!(!identities.by_key.contains_key(&0x1111));
        assert_eq!(
            identities.by_key[&0x2222].display_name.to_str().unwrap(),
            "Remote Player"
        );
    }
}

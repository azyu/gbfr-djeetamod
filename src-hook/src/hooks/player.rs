use std::ffi::CString;

use anyhow::{anyhow, Result};
use protocol::{Message, PlayerIdentityEvent};
use retour::static_detour;

use crate::{
    event,
    hooks::{actor_idx, actor_type_id},
    process::Process,
};

type RefreshPlayerIdentityFunc = unsafe extern "system" fn(*const usize);

static_detour! {
    static RefreshPlayerIdentity: unsafe extern "system" fn(*const usize);
}

/// Offset of the 0x250-byte player identity snapshot in a game 2.0.2 player
/// specified-instance. The snapshot retains the name fields used by game 1.x.
const PLAYER_IDENTITY_OFFSET: usize = 0x5E60;
const IS_ONLINE_OFFSET: usize = 0x1C8;
const CHARACTER_NAME_OFFSET: usize = 0x1E8;
const DISPLAY_NAME_OFFSET: usize = 0x208;
const PARTY_INDEX_OFFSET: usize = 0x22C;
const VBUFFER_INLINE_CAPACITY: usize = 0x0F;
const MAX_PLAYER_NAME_BYTES: usize = 0x100;

/// Unique game 2.0.2 prologue for the function that rebuilds the player
/// identity snapshot. Hooking the refresh gives us names before the first hit.
const REFRESH_PLAYER_IDENTITY_SIG: &str =
    "55 41 57 41 56 41 54 56 57 53 48 83 ec 70 48 8d 6c 24 70 48 c7 45 f8 fe ff ff ff 80 b9 bc 5e 00 00 00";

#[derive(Clone)]
pub struct OnLoadPlayerIdentityHook {
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
            RefreshPlayerIdentity.initialize(func, move |player| cloned_self.run(player))?;
            RefreshPlayerIdentity.enable()?;
        }

        Ok(())
    }

    fn run(&self, player: *const usize) {
        unsafe { RefreshPlayerIdentity.call(player) };

        if player.is_null() {
            return;
        }

        let snapshot = unsafe {
            (player.byte_add(PLAYER_IDENTITY_OFFSET) as *const *const u8).read_unaligned()
        };

        let Some(event) = (unsafe { player_identity_event(player, snapshot) }) else {
            return;
        };

        let _ = self.tx.send(Message::PlayerIdentityEvent(event));
    }
}

unsafe fn player_identity_event(
    player: *const usize,
    snapshot: *const u8,
) -> Option<PlayerIdentityEvent> {
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

    if is_online > 1 || (party_index > 3 && party_index != u32::MAX) {
        return None;
    }

    let display_name = read_vbuffer(snapshot.byte_add(DISPLAY_NAME_OFFSET))?;

    // NPC snapshots have no display name. They already fall back to their
    // character type in the meter and do not need an identity event.
    if display_name.as_bytes().is_empty() {
        return None;
    }

    let character_name = read_vbuffer(snapshot.byte_add(CHARACTER_NAME_OFFSET))
        .unwrap_or_else(|| CString::new("").expect("empty CString is valid"));

    Some(PlayerIdentityEvent {
        character_name,
        display_name,
        character_type: actor_type_id(player),
        party_index: party_index as u8,
        actor_index: actor_idx(player),
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
    use super::read_vbuffer;

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
}

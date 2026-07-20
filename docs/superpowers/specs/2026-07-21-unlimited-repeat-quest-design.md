# Unlimited Repeat Quest Toggle Design

**Date:** 2026-07-21

## Goal

Add one opt-in game-code toggle to Djeeta MOD's general settings:
`무한 퀘스트 반복` (`Unlimited Repeat Quest`). The toggle removes the
quest-repeat limit while it is enabled and restores the game's original code
when it is disabled or Djeeta MOD exits normally.

This feature is separate from the read-only equipment and inventory probes.
Those probes must retain their current `PROCESS_QUERY_INFORMATION |
PROCESS_VM_READ` access contract.

## Source and verified target

The behavior is derived from the `Unlimited Repeat Quest` entry in
`GranblueFantasyRelink-Standalone-v0.2.8.CT` with SHA-256
`65A3677AD62593617077B9655530FE52B24189CA02E1CEC93C16648CF5FC3072`.
Do not bundle or invoke Cheat Engine.

The two signatures each occur exactly once in the on-disk target executable:

- Game: Granblue Fantasy: Relink Endless Ragnarok 2.0.2, Windows x64
- Executable SHA-256:
  `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F`

The first signature identifies the repeat-state reset instruction. At signature
offset `0x28`, enabling changes `45 31 C0` to `44 8B 01`.

The second signature identifies the repeat-state getter. At signature offset
`0x12`, enabling changes `0F B6 01` to `B0 01 90`.

Absolute addresses are never fixed or persisted. Every process is located and
scanned independently.

## User experience

Add a `게임 기능` section to the general settings page. It contains:

- Switch label: `무한 퀘스트 반복`
- Description: `퀘스트 반복 횟수 제한을 해제합니다.`
- An English translation for both strings
- A short status or failure reason when the feature is unavailable

The switch is visible in release builds. It starts OFF on every Djeeta MOD
launch and is not stored in the persistent meter settings. Disable the switch
while support is being checked or a state change is in progress.

The backend reports distinct unavailable reasons for at least:

- game not running;
- unsupported executable hash;
- signature absent or ambiguous;
- unexpected target bytes;
- process access denied;
- patch or restoration failure.

Djeeta MOD remains `asInvoker` and never requests elevation automatically. If
Windows denies the additional process rights, the UI reports that the feature
cannot be changed with the current permissions.

## Architecture

Implement the feature in a dedicated Tauri backend module. Do not extend the
read-only inventory probe or make Cheat Engine a runtime dependency. The module
owns:

- process and executable validation;
- `.text` signature discovery;
- target-byte classification;
- the narrowly scoped writable process handle;
- transactional enable and best-effort safe restoration;
- runtime state for the active PID and discovered sites;
- Tauri commands for querying status and setting the enabled state.

Expose the commands as `get_repeat_quest_status` and
`set_repeat_quest_enabled(enabled)`. Both return the backend's observed state so
the frontend does not invent an optimistic ON/OFF result.

The current hook pipe is send-only from the injected DLL to the backend.
Changing it to a bidirectional command channel would expand the protocol and
hook lifecycle for a two-site patch, so the hook DLL is not the patch owner.

Within this feature path, only the dedicated patch module may explicitly request
`PROCESS_VM_WRITE | PROCESS_VM_OPERATION`, in addition to the query and read
rights needed for validation. It changes exactly the two verified three-byte
sites and calls `FlushInstructionCache` after writes. Each protection request is
limited to a three-byte target range; Windows may apply the protection change to
the containing page. Restore the previous protection after every write attempt.

## State model

The backend exposes three logical states:

- `Unavailable(reason)`
- `Off`
- `On`

An in-progress flag prevents overlapping state changes. Cached addresses are
bound to the verified PID and are discarded whenever the process changes or
exits. Status queries read the current target bytes instead of trusting only the
cached state.

The two sites are classified independently as `Original`, `Patched`, or
`Unknown`.

- `Off` means both sites are original.
- `On` means both sites are patched.
- A mixture of original and patched bytes is recoverable toward OFF.
- Any unknown site makes enabling unavailable and is never overwritten.

## Enable transaction

Enabling performs these steps immediately before writing:

1. Locate the current game process.
2. Verify the exact executable SHA-256.
3. Read the in-memory `.text` section.
4. Require exactly one match for each signature.
5. Verify that both target sites contain their original bytes.
6. Apply the reset-site patch.
7. Apply the getter-site patch.
8. If the second write fails, restore the first site.
9. Flush the instruction cache and read both sites back.
10. Report `On` only if both patched bytes are observed.

If rollback itself fails, return a restoration error and expose the actual
observed state. Never report a successful ON state from a partial patch.

## Disable and lifecycle restoration

Disabling revalidates the PID, executable, signatures, and current bytes. Each
site already containing the original bytes is left unchanged. Each site exactly
matching the known patched bytes is restored. Unknown bytes are not overwritten.
The final state is read back before reporting OFF.

Closing a window only hides it in the tray and does not disable the feature.
The tray `종료` action and the Tauri application exit event synchronously attempt
restoration before process exit.

At application startup, the desired state is always OFF. If the same game
process survived an earlier abnormal Djeeta MOD termination, startup detects
known patched bytes and attempts restoration before the UI can enable the
switch. A forced Djeeta MOD termination cannot guarantee immediate restoration;
the patch can remain until Djeeta MOD is restarted or the game exits.

When the game exits, its original executable code is naturally restored on the
next launch, and Djeeta MOD discards the old PID state.

## Testing

Use an abstract memory accessor and fake memory for deterministic backend tests:

- both signatures uniquely found;
- missing and duplicate signatures rejected;
- unsupported executable rejected before any write;
- original, patched, mixed, and unknown byte classification;
- successful two-site enable;
- second-site failure rolls back the first site;
- disable restores patched sites and leaves original sites unchanged;
- unknown bytes are never overwritten;
- read-back mismatch is not reported as success;
- PID changes invalidate cached addresses;
- startup and normal-exit cleanup request OFF.

Frontend tests cover visibility in release settings, Korean and English labels,
unavailable reasons, pending-state locking, successful toggling, and failed
requests reflecting the backend's observed state.

Manual validation uses the pinned 2.0.2 executable in an offline or private
session:

1. confirm the switch starts OFF;
2. enable it and verify repeat-quest behavior beyond the normal limit;
3. disable it and verify both original byte sequences are restored;
4. enable it again, exit Djeeta MOD from the tray, and verify restoration;
5. simulate an interrupted/partial state in a controlled test and verify safe
   recovery toward OFF;
6. confirm unsupported or inaccessible processes produce a reason without a
   write attempt.

Do not claim game compatibility from unit tests alone. Record the manual result
in the existing 2.0.2 smoke-test documentation.

## Out of scope

- Any other Cheat Engine table entry
- Bundling or launching Cheat Engine
- Persisting the ON state across Djeeta MOD launches
- Automatically elevating Djeeta MOD
- General-purpose game memory editing APIs
- Changing the existing injected-hook protocol

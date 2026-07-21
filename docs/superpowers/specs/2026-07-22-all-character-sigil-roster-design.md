# All-Character Sigil Roster Design

Date: 2026-07-22

Target: Djeeta MOD 0.1.1 and Granblue Fantasy: Relink Endless Ragnarok 2.0.2 on Windows x64

## Goal

Show every character available in the local save in the Sigil Trait Cap Analysis screen without requiring the user to put each character in the party or open each character's equipment screen first.

For each discovered character, show either a validated snapshot of the primary and secondary traits from all 12 equipped sigils or an explicit state explaining that the sigil data could not be read. A failed read must never make a character disappear or manufacture an empty or zero-valued loadout.

## Current limitation

The injected hook publishes a `LocalEquipmentSnapshot` only when the game's player identity refresh callback exposes an offline local record. At initial connection this normally covers the four local party slots; additional characters appear only after the game refreshes their records. The Tauri backend stores only snapshots that arrive, and React builds its selector directly from that response.

The verified external reader also starts from four local party keys. It proves that the current party's equipment can be located independently, but it does not enumerate the complete local roster. The inventory probe finds the owned sigil inventory, not the character-to-equipped-sigil mapping, and is not a substitute for roster discovery.

## Scope

### Included

- Characters that the active local save marks as available for use.
- The primary and secondary traits from each character's 12 equipped sigil slots.
- Automatic initial discovery after connection.
- A low-frequency five-second rediscovery interval for roster or equipment changes not observed by the hook callback.
- Per-character `complete`, `unavailable`, and `unsupported` presentation states.
- Existing callback snapshots as the trusted fallback and comparison source.
- Read-only access through `PROCESS_QUERY_INFORMATION | PROCESS_VM_READ`.

### Excluded

- Online party members' equipment.
- Owned but unequipped sigil inventory in the character analysis table.
- Weapon, wrightstone, master trait, or summon contributions.
- Game-memory writes, save changes, input automation, or forced menu navigation.
- Guessing an unlock flag, roster address, or equipment pointer that has not been verified on the pinned 2.0.2 executable.
- Claiming full 2.0.2 compatibility before the manual validation gate passes.

## Architecture

### Pure roster discovery

Add a pure Rust roster-discovery layer under `src-tauri/src/equipment_probe/`. It consumes the existing `MemoryReader` abstraction plus version-pinned roots and offsets, and produces validated roster entries without owning a process handle or UI state.

Each accepted entry must pass all of these checks:

- its character key is a known playable local character key;
- the containing pointer and all derived ranges are valid user-space addresses;
- the player record's self key matches the enumerated key;
- the save's availability marker is present and has a verified value;
- linked structures terminate without cycles, duplicate records, or traversal-limit violations;
- the equipped-sigil range can be read in full or is classified as unavailable;
- two reads of the equipment bytes agree before numeric traits are decoded.

Roster discovery returns one entry per available character. Duplicate character keys are an error for the affected entry rather than a last-write-wins condition.

### Development validation probe

Full-roster enumeration begins behind a debug-build and explicit environment-variable gate. The probe records only character keys, statuses, counts, and short snapshot digests. It must not log player names, raw sigil records, reusable absolute addresses, or save contents.

When a hook snapshot exists for the same character, the probe compares the decoded sources. A mismatch never replaces the hook result. Discovery is eligible for production only after the manual validation gate succeeds across three fresh game processes.

If the player manager does not expose a verified complete roster, development stops at the probe. Broad memory pattern scanning is not promoted as a fallback production design.

### Production reader

After validation, the same pure discovery path runs from a Tauri-owned read-only worker:

1. Verify the executable SHA-256 and all required signatures.
2. Resolve the player manager and complete local roster root.
3. Enumerate available local characters.
4. Read each character's equipped-sigil bytes twice.
5. Decode stable snapshots with `equipment-core`.
6. Merge the result with hook snapshots.
7. Publish only when the roster or a character result changes.

The worker runs once after an accepted hook connection and then every five seconds. Only one discovery run may be active. Disconnect cancels future work and invalidates all process-derived pointers before another connection may start.

### State ownership

`EquipmentState` separates roster membership from equipment snapshots.

- The roster map owns every verified available character and its current capture status.
- The snapshot map owns the latest trusted decoded sources per character.
- A hook event may add or update a character even when external discovery is unavailable.
- External discovery may add roster entries, but an unverified or mismatching external snapshot may not replace a valid hook snapshot.
- A failed read updates only the affected character and must not clear other characters.

The frontend response includes all roster entries. A character with no trusted snapshot has no numeric trait analysis.

## Data model

The backend and frontend character status supports:

- `complete`: all 12 sigil slots were read stably and validated;
- `unavailable`: roster membership is verified, but the character's equipped sigils could not be read safely;
- `unsupported`: the pinned executable, signature, or required layout is not supported.

`unavailable` is a backend/UI state. It does not need to be sent by the injected DLL, so the existing append-only wire message ordering remains unchanged. The DLL continues to publish `Complete` or `Unsupported` snapshots using the existing protocol.

The response does not expose pointers, error internals, or raw record bytes. Stable internal reason codes may be logged and tested, while the UI presents one localized unavailable message.

## Data flow

```text
game connection accepted
  -> validate pinned executable and signatures
  -> discover complete available-character roster
  -> create one state entry per character
  -> double-read and decode each equipped-sigil array
  -> compare with hook truth when available
  -> merge trusted results into EquipmentState
  -> emit the complete roster response
  -> repeat after five seconds unless disconnected

hook LocalEquipmentSnapshot
  -> record comparison truth
  -> update that character immediately
  -> emit only if its effective result changed
```

React continues to derive selector options from `response.characters`, but the backend now supplies the complete verified roster. Selection remains stable when the selected character remains present. An unavailable character remains selectable and shows a localized read-failure state instead of an empty trait table.

## Failure behavior

- A roster-reader failure never changes the damage meter's connection state or the latched hook status.
- A hash or required-signature mismatch marks sigil analysis unsupported and prevents exploratory reads.
- Invalid pointers, partial reads, unstable double reads, duplicate keys, cycles, and traversal limits produce unavailable entries when roster membership is known.
- If roster membership itself cannot be verified, the backend keeps the existing callback-derived characters and reports that automatic roster discovery is unavailable.
- Hook/external disagreement keeps the hook snapshot and logs only the character key, source counts, and short digests.
- Disconnect clears roster and snapshots and prevents stale worker results from being applied through a connection-generation token.
- No failure becomes an empty equipment array or numeric zero.

## Testing

### Pure Rust tests

- Enumerate multiple characters across empty buckets, single-node buckets, and collision chains.
- Reject cycles, duplicate keys, traversal overflow, invalid pointers, self-key mismatches, unknown character keys, and unverified availability values.
- Preserve verified roster membership when one equipment range is unreadable.
- Reject partial and unstable equipment reads without producing numeric traits.
- Prefer a hook snapshot when external data disagrees.
- Accept identical external and hook snapshots.
- Suppress unchanged rediscovery output.
- Ignore a result produced for an obsolete connection generation.

### Backend and frontend tests

- Return every roster entry, including unavailable characters.
- Keep one character's failure from deleting other characters.
- Preserve callback-only behavior when roster discovery is unavailable.
- Keep an unavailable character in the selector and render the localized unavailable state.
- Preserve selection across equivalent full-roster updates.
- Clear stale state after disconnect and rebuild it after reconnect.
- Preserve all existing damage-meter, equipment-contract, and protocol tests.

### Manual game validation gate

Use an offline or private session and a controlled save whose available-character count is known.

1. Compare the discovered roster count and character keys with the game's available-character list.
2. Compare all 12 equipped sigils for every character with the game UI.
3. Change and restore one sigil on at least two characters outside the active party; require an update within five seconds.
4. Fully exit and restart the game three times; require the roster and snapshots to be rediscovered without reusable absolute addresses.
5. Exercise an unavailable read and confirm that the character stays visible without numeric traits.
6. Confirm the meter, encounter persistence, and repeat-quest controls continue to work.

Production activation is blocked unless every run passes. Automated tests or a successful package build do not satisfy this gate.

## Success criteria

- On connection, every character available in the active local save appears without manual character switching.
- Every complete character matches the game's equipped primary and secondary sigil traits.
- A character whose equipment cannot be read stays in the selector with an unavailable state.
- Changes outside the active party appear within five seconds.
- Unsupported or ambiguous layouts do not trigger broad memory scans or fabricated results.
- Existing hook snapshots remain a safe fallback.
- The implementation uses read-only process access and does not affect the damage-meter connection state.
- Full-roster production activation occurs only after the three-process manual validation gate passes.

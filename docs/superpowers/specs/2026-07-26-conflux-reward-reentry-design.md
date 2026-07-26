# Conflux Reward Selection and Re-entry Design

**Date:** 2026-07-26

## Decision

Granblue Fantasy: Relink's unattended option continues to own every in-run
route, Power, monk, and ordinary single-OK decision. Djeeta MOD adds only the
remaining repeat-cycle boundaries:

1. select the configured final treasure reward;
2. advance TOTAL RESULTS;
3. choose Tredame Palace as the return destination;
4. activate the Tredame Conflux gate;
5. keep the current party;
6. confirm the depth already focused by the game.

The existing timer reduction remains a separate Tauri-owned data patch. Reward
selection and re-entry require native game callbacks and therefore belong to an
optional injected-hook capability.

## Product Behavior

The existing `극돈공소` page keeps one OFF-by-default automation switch and
adds a non-clearable final-reward dropdown. The dropdown presents localized
treasure names but stores the version-pinned internal reward ID.

The switch is the single master control. Enabling first validates and arms the
hook capability, then applies the timer reduction. If either step fails, the
hook returns to OFF and the original timer configuration is restored.
Disabling stops hook actions first and then restores the timer.

On the final reward screen, automation selects the first selectable row whose
internal ID matches the preference. If no selectable row matches, it selects
the first selectable row. Acquisition counts, favorite state, and remaining
quantity do not affect selection.

After selection, automation advances TOTAL RESULTS, returns to Tredame Palace,
activates the Conflux gate, keeps the current party, and confirms the depth
already focused by the game. It does not synthesize keyboard, mouse, or player
movement. An unrecognized state disables reward/re-entry automation without
disabling the damage meter or timer restoration.

The first catalog covers floor five. Floor-three validation uses the
first-selectable fallback and does not add a floor-three catalog.

## Analysis Ladder

### Stage A: independent analysis

Each boundary receives one bounded independent pass using the existing
2.0.2 executable, RTTI/vtables, callers, and offline/private live captures. A
boundary is considered directly implementable only when all of these are
known:

- exact active-state discriminator, including a hidden/stale negative;
- bounded data and indices required by the action;
- accept callback or event, distinguished from cancel and adjacent actions;
- callback ABI and game-thread invocation point;
- observable successor proving the action was accepted.

The pass ends after the existing candidate function and its immediate
callers/callees have been traced and one positive/negative live comparison has
been attempted. It does not expand into speculative global scans or broad
detours.

### Stage B: reference-assisted analysis

If any required item remains unresolved after Stage A, analysis switches to the
user-supplied `GBFR-Conflux-Infinite-Retry.zip`. The reference boundary is:

- pin archive SHA-256
  `02FE3756F47118D5F957EE597C4C4776877AE906A9D373A913C1D0D9FADCBA71`;
- extract only to a temporary directory outside the repository;
- inspect only the bundled PDB metadata and relevant .NET IL/methods;
- exclude `Auto Portal Selector`, `Auto Power Selector`, and unrelated mods;
- do not execute or load the reference DLL into the game;
- do not copy or redistribute its DLL, PDB, source reconstruction, or bundled
  dependencies;
- treat addresses, signatures, offsets, and method names as hypotheses only.

Every hypothesis from the reference must be independently relocated in the
pinned game executable and pass the same active-state, ABI, negative-state, and
successor checks before implementation. The design does not require preserving
the reference mod's architecture or implementation.

## Architecture

### Hook capability

A focused `conflux_retry` hook module owns:

- a pure state machine;
- version-pinned native observations;
- one-shot action dispatch on verified game-thread callbacks;
- per-screen action acknowledgement and timeout handling;
- an immutable status/config snapshot.

The capability is optional. Signature or state validation failure reports only
Conflux automation as unavailable; existing damage, identity, equipment, and
timer features continue to work.

### Control boundary

Reward preference and enable state cross a dedicated local-only control
channel. Existing append-only gameplay messages and their variant ordering
remain unchanged. The control channel supports:

- status read;
- reward-preference update;
- enable and disable.

Disconnect, app exit, game-process replacement, configuration mismatch, or
transition timeout moves the hook capability to OFF. Re-enabling is always an
explicit user action.

### State sequence

The minimal state machine is:

`Off → Armed → RewardSelection → TotalResults → ReturnDestination →
TredameGate → PartyFormation → DifficultyConfirmation → Armed`

Each state may emit at most one action for a stable screen identity. The next
action is prohibited until the expected successor is observed. An unexpected
controller, invalid row/index, duplicate screen identity, timeout, or unknown
dialog fails closed.

## Native Boundary Requirements

### Final reward

The verified `ControllerEndlessResultReward` and
`MenuResultRewardTreasure` relationship must expose:

- bounded reward rows;
- internal reward ID and selectable state;
- selected/current index;
- menu-change path;
- confirm path.

Selection acknowledgement precedes confirmation. A localized label or visible
screen coordinate is never authoritative.

### TOTAL RESULTS and return destination

The implementation must distinguish ready TOTAL RESULTS from the preceding
reward screen even when controller allocations are reused. It advances only
from the verified ready state and then requires the return-destination
controller.

The return action selects the independently verified Tredame value and confirms
it. Default focus alone is insufficient, and cancel must be independently
excluded.

### Tredame gate, party, and depth

The Tredame portal must be distinguished from route portals and the final-boss
field gate by place, phase, or destination state. Native interaction must
produce the party-formation successor without requiring player range or
movement.

Party confirmation keeps the existing party. Depth confirmation accepts the
game's already focused valid depth; it does not force floor five. Each screen
requires its own active discriminator, confirm callback, cancel exclusion, and
successor.

## Safety

- Target only the pinned 2.0.2 executable SHA-256 already enforced by Djeeta
  MOD.
- Require one unique masked code signature for every installed detour or native
  callback.
- Invoke game functions only from a verified game-thread callback.
- Never allocate a remote thread or synthesize input.
- Never act from allocation count, localized text, icon color, or coordinates.
- Keep the feature OFF when any boundary is unavailable.
- Do not claim compatibility until the offline/private live checklist passes.

## Validation

For each boundary:

1. pure state-machine tests cover success, fallback, duplicate suppression,
   invalid observations, timeout, and OFF;
2. static tests prove unique signatures and bounded field interpretation;
3. a debug observation build records positive and negative states without
   invoking actions;
4. an action-capable build performs exactly one approved transition;
5. the expected successor and unrelated-screen negatives are recorded.

The complete floor-three smoke test starts with the game unattended option and
Djeeta MOD automation enabled, uses first-selectable reward fallback, returns
to Tredame, and re-enters the game-focused depth without manual input after the
final reward screen.

## Excluded Work

- route, Power, monk, and generic dialog automation;
- per-floor reward catalogs other than floor five;
- favorite-state or acquisition-count policies;
- forced party composition or forced depth;
- loading, executing, or redistributing third-party mod binaries.

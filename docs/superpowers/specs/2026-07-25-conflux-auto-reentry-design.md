# Conflux Auto Re-entry Design

> **Superseded on 2026-07-25:** Route, Power, monk, and dialog-specific
> automation moved to follow-up TODOs. The active design is
> `docs/superpowers/specs/2026-07-25-conflux-minimal-autodrive-design.md`.

**Date:** 2026-07-25

## Goal

Add an opt-in `극돈공소 자동 재진입` feature to Djeeta MOD that independently
implements this cycle without bundling or loading the downloaded Reloaded-II
mods:

1. whenever one or two route portals interrupt the run, take the sole route;
   when continuation and return are paired, always continue to `공무의 경지`;
   otherwise prefer a route with the rare Power appearance-rate bonus, then
   combat, and use route slot zero on a remaining tie;
2. in `공무의 경지`, interact with the monk before taking either outgoing
   portal, spend CP while the game offers another purchase, apply the Power
   priority rule to each five-choice list, acknowledge each acquisition, and
   close the remaining monk root menu after the game ends the purchase loop;
3. periodically detect and dismiss an allowlisted Conflux-only blocking dialog
   when its exact single-OK state is active;
4. acknowledge the exact Endless area-result information screen after a route;
5. acknowledge the exact mid-boss reward result;
6. whenever a Power of the Boundary selection interrupts the run, choose
   display index zero when a Chaos type is present; otherwise choose the highest
   game-computed grade and use the lowest display index on a tie;
7. after the final boss, activate the exact field return gate;
8. on the floor-five final-reward screen, choose the first selectable row whose
   internal reward ID matches the persisted user preference, or the first
   selectable row when no matching row is available;
9. accept the reward and advance TOTAL RESULTS;
10. return to Tredame Palace;
11. activate the Tredame gate;
12. keep the current party and confirm the currently focused depth to re-enter
    battle;
13. re-arm for route selection, monk CP spending, blocking-dialog checks,
    area/boss-result
    acknowledgement, Power
    selection, and the next final-reward screen.

The feature starts OFF for every game process. It is controlled only through a
dedicated `극돈공소` management page. There is no hotkey in this scope.

## Scope

The first implementation covers the floor-five final-reward catalog and the
complete Tredame return/re-entry flow. It does not include:

- reducing the Conflux inactivity timer;
- reward catalogs or per-floor preferences for floors other than five;
- changing MSP limits;
- importing, executing, or redistributing any downloaded DLL;
- installing Reloaded-II, Reloaded.Hooks, or a .NET runtime.

The omitted gameplay features remain follow-up TODOs. Each requires
its own design, tests, and offline/private live validation before implementation.

Fast live validation may use floor three. Because the configured reward catalog
is floor-five-only, any floor-three reward screen uses the documented first-
selectable fallback and does not add floor-three entries to the dropdown.

## Product Contract

The management sidebar adds a navigable `극돈공소` page rather than extending
the existing `무한 퀘스트 반복` switch. The existing feature patches the
normal quest-repeat limit; it does not automate the Conflux result or re-entry
UI.

The `극돈공소` page contains:

- an `자동 실행` switch;
- a non-clearable `5층 보상 선택` dropdown populated from a version-pinned
  floor-five reward catalog.

The dropdown displays localized reward names but stores and sends the
corresponding internal reward ID. Its preference persists across app and game
restarts. Acquisition-count text and remaining-count fields are neither shown
as configuration nor consulted by selection logic. The initial preference is
the first entry in the pinned floor-five catalog.

The game-provided `극돈공소 무조작 시 설정` is expected to be ON during
unattended operation and live validation. Djeeta MOD does not duplicate,
modify, or claim ownership of that game option. It remains a fallback for
unrecognized blocking UI; Djeeta MOD still performs the validated policy and
world-interaction actions immediately.

The auto-reentry switch:

- starts OFF whenever a new game process loads `hook.dll`;
- is not persisted across app or game restarts;
- becomes unavailable when its optional hook capability cannot be validated;
- does not affect the damage meter or other hook capabilities when unavailable;
- disables automation immediately when switched OFF;
- disables automation when the control client disconnects, including an app
  restart while the same game process remains alive;
- requires an explicit new enable action after any failure.

Changing the reward preference while automation is ON applies to the current
reward screen only if no selection action has been issued. Once selection has
started, the new preference applies from the next reward screen.

The feature is compatibility-unverified until the dedicated live checklist
passes in an offline or private session. Automated tests, signature matches, and
successful packaging do not establish game 2.0.2 compatibility.

## Architecture

### Hook-owned automation

All UI-controller inspection, detours, state transitions, and internal game
function calls live in a focused `src-hook` Conflux auto-reentry module. Native
game functions are invoked only from verified game-thread hook callbacks. Pipe
tasks and Tauri commands may update requested state or read status snapshots,
but they never invoke a game function.

The module is an optional hook capability. Core damage, identity, equipment, and
reward-boundary hooks are installed first and continue to determine
`HookStatus::{Ready, Unsupported}`. Failure to validate an auto-reentry
signature records only an auto-reentry `Unavailable` status.

The module is divided conceptually into:

- a pure state machine that consumes observations and produces one requested
  native action at a time;
- an allowlisted blocking-dialog monitor that can temporarily suspend and
  resume the current state-machine stage;
- a native adapter that validates controller types, vtables, active state,
  indices, callbacks, and event identities;
- detours that preserve the original game call and pass post-call observations
  to the state machine;
- an immutable status snapshot shared with the control server;
- a debug-only observation probe used before any candidate is promoted to an
  action-capable production hook.

### Separate control pipe

The existing send-only damage/event pipe and its `Message` enum remain
unchanged. A second local-only, length-delimited named pipe provides a small
request/response control protocol:

- app to hook: `GetStatus`, `SetRewardPreference`, `SetEnabled(true)`, and
  `SetEnabled(false)`;
- hook to app: the correlated status response for that request.

The control pipe rejects remote clients. Requests are serialized so only one
state mutation is in flight. A monotonically increasing request ID correlates
responses and prevents a stale response from replacing a newer UI state.

The control protocol uses new standalone bincode types. Existing `Message`
variants retain their indices and legacy decoding behavior. The hook closes a
control session in the OFF state and fails closed when a client disconnects.
Status includes the effective reward ID and configuration revision. Enabling
is rejected until the hook has acknowledged a valid floor-five preference.

### Tauri and React boundary

A focused Tauri state owns the control-pipe client and exposes:

- `get_conflux_retry_status`;
- `set_conflux_reward_preference`;
- `set_conflux_retry_enabled`.

The React page mirrors the existing repeat-quest request lifecycle: initial
status read, one pending mutation at a time, and replacement with the returned
authoritative status. It persists the selected floor-five reward ID locally
and synchronizes it to a newly connected hook before enabling automation.
While the returned state is `On`, it polls the authoritative control status
every 250ms so stage changes and fail-closed transitions become visible without
adding unsolicited frames to the control protocol. Polling stops as soon as
the state becomes `Off` or `Unavailable`. The sidebar contains only the
navigation entry; the switch and dropdown live on the dedicated page.

No Tauri auto-reentry module requests remote write, operation, allocation, or
thread-creation rights. The only game-memory mutation and native invocation
belong to the already injected Rust hook.

## Version and Target Validation

The action-capable module is enabled only for the pinned Granblue Fantasy:
Relink Endless Ragnarok 2.0.2 executable SHA-256 already used by Djeeta MOD.
Every required code signature must match exactly once. Every fixed vtable RVA,
field offset, callback target, and event ID is treated as version-specific.

Signature presence alone is insufficient. Before each native action, the
adapter verifies the live object against all available invariants for that
stage:

- exact controller or menu vtable;
- expected active/root state;
- readable vector bounds and a bounded row count;
- selected and current indices within the live row count;
- callback ownership by the expected executable module;
- expected menu/controller relationship;
- exact event identity for FSM publication.

Any validation failure returns an observation error to the state machine and
does not call the candidate function.

## State Machine

The pure state machine has these externally reportable stages:

- `Off`
- `Armed`
- `RouteSelection`
- `SanctumMonk`
- `SanctumCpConfirmation`
- `SanctumPowerSelection`
- `SanctumPowerAcquired`
- `SanctumExit`
- `BlockingDialogAcknowledgement`
- `AreaResultAcknowledgement`
- `BossResultAcknowledgement`
- `PowerSelection`
- `FinalBossReturnGate`
- `RewardSelection`
- `TotalResults`
- `ReturnDestination`
- `TredameGate`
- `PartyFormation`
- `DifficultyConfirmation`

Enabling moves `Off` to `Armed`. Only the exact active final-reward controller,
independently verified route portals, monk shop relationship, or independently
verified Power-selection owner may begin a transition.

### Allowlisted blocking-dialog monitor

While automatic execution is ON, a game-thread observation path checks for
allowlisted Conflux blocking dialogs at most once every 250ms. This monitor is
orthogonal to the main progression stage: when it accepts a dialog, it stores
the suspended stage, reports `BlockingDialogAcknowledgement`, and prevents all
ordinary progression actions until the dialog is dismissed.

An allowlist entry contains the exact controller and dialog identity, required
Conflux context, active/ready state, single-OK action, dismissal
acknowledgement, and permitted successor states. A generic modal, localized
button text, visible button count, or screen coordinates are never sufficient.
The initial allowlist covers only independently verified variants of:

- Endless area-result information;
- mid-boss result;
- `경지의 힘 획득` acquisition.

No other dialog is included in the initial allowlist. A newly observed blocker
requires its own positive and negative live evidence before a later allowlist
update.

The monitor issues one OK action per stable dialog identity, waits for the
dialog to become inactive, and then resumes the suspended stage or accepts its
verified successor. Repeated update callbacks cannot produce another action.
If an unknown modal prevents the expected stage transition, automation does
not press it. It allows more than the game's normal inactivity interval for the
game-provided fallback to advance, then expires to OFF with an
`unknownBlockingDialog` reason if the modal remains. This preserves fail-closed
behavior while making the allowlist independently extensible.

### Route selection

An unattended run stops when one or two route portals appear. The adapter uses
the validated active `BaEndlessPortal` objects as authority. Gate-icon objects
are supporting evidence only because an inactive/reused icon can remain after
the number of live portals falls from two to one.

One valid active portal is selected without applying priority rules. For two
valid portals, the adapter reads the game's route modifier and route-type enum
rather than localized text, icon color, or screen coordinates.

If the pair contains the post-boss continuation destination (`공무의 경지`) and
the return destination (`귀환하기`), continuation wins unconditionally.
Otherwise, if exactly one portal has the game's rare-Power appearance-rate-up
modifier, that portal wins regardless of route type. If no such modifier wins
and exactly one portal is a combat route, that portal wins. A remaining tie
uses the lowest generator slot index.

Destination, modifier, and route type come from verified game fields, never
localized text, color, or image recognition.

The adapter then invokes the verified native portal interaction path once. It
does not synthesize movement or keyboard input. The chosen portal must
acknowledge the transition and the portal pair must become inactive before the
state returns to `Armed`.

### `공무의 경지` monk and CP loop

The monk must be serviced before either outgoing portal is selected. Live
observation showed two `BaEndlessPortal` objects already allocated while the
monk root menu was open, so portal presence alone cannot authorize route
selection in this area.

The adapter identifies the exact active `EtEndlessModeShop` relationship and
opens its verified interaction path. In the monk root menu it selects the
game-defined `경지의 힘 획득` entry. The subsequent
`ControllerEndlessShopDialog` confirms the CP cost; the adapter reads the
game's affordability/availability state rather than assuming a fixed cost or
parsing the displayed number.

Each accepted purchase opens a five-choice vertical Power list. It uses the
same semantic priority as ordinary Power selection: if any visible choice is
the verified Chaos type, select display index zero; otherwise select the
highest game-computed grade and use the lowest display index on a tie. After
selection, the exact active `ControllerEndlessBuffAcquired` OK action is
acknowledged once.

When enough CP remains, the game returns to the CP confirmation and the loop
repeats. When CP is insufficient, the game automatically leaves the purchase
subflow and returns to the monk root menu; automation must not issue a
speculative cancel from the purchase dialog. It closes the verified root menu
once, waits for the shop relationship to become inactive, and only then
permits route selection. An unexpected dialog, zero or more than five visible
choices, a missing acquisition acknowledgement, or a portal action while the
shop is active fails closed.

### Area-result acknowledgement

An Endless route can display a blocking result-information screen containing
the area name, clear time, reward, and one OK action. The adapter accepts only
the exact active `ControllerEndlessEventResult` relationship and its verified
ready state. A merely allocated `ControllerEndlessBossResult`, a generic popup,
or another confirmation dialog is insufficient.

After the settle interval, the state machine requests the result controller's
exact decide callback once. It waits for the active result state to clear and
for the expected Power screen or next route transition. A callback return by
itself is not acknowledgement.

### Power selection

Power selection is required because an unattended run otherwise stops before
the final reward. The adapter reads every live visible Power choice and its
game-computed type and grade. If any visible choice has the verified Chaos type,
the adapter chooses display index zero without comparing grades. Otherwise it
chooses the maximum grade; equal grades keep the lowest left-to-right display
index. Localized name, description text, icon shape, and effect magnitude do
not affect priority.

The state machine requests at most one selection change and one confirmation
for a Power screen. It waits for the live selected index to acknowledge the
target before confirming. Screen dismissal must be observed before returning
to `Armed`; a reused object, hidden template, invalid grade, or visible-count
mismatch fails closed.

### Boss-result acknowledgement

After a mid-boss, the game presents one blocking reward-result screen with an
OK action and then opens the normal Power-selection screen. There is no
separate single-choice boss reward.

The adapter requires the independently verified active
`ControllerEndlessBossResult` state and its result-controller relationship. It
does not infer the stage from a single route, an allocated
`MenuEndlessResultReward`, or a hidden boss-result object. After the settle
interval, the exact boss-result decide callback is requested once. The state
machine requires result dismissal and the subsequent Power-selection state
before accepting the transition.

The final boss instead returns control to the field with one active
`귀환하기` gate and no active result or reward controller. The adapter enters
`FinalBossReturnGate` only when the portal's independently verified destination
and phase identify this exact final-boss boundary. It invokes the native
interaction once and requires the floor-five reward controller to become
active. A generic single portal is not enough.

### Reward selection

The floor-five reward screen is owned by the validated
`ControllerEndlessResultReward` and `MenuResultRewardTreasure` relationship,
not the `MenuEndlessResultReward` objects used by ordinary Power selection.

The native adapter reads the live variable-length reward-row vector, including
rows outside the visible viewport. It ignores acquisition-count and
remaining-count fields. It selects the lowest selectable index whose internal
reward ID equals the acknowledged floor-five preference. If no selectable row
matches, it selects the first selectable row. If no selectable row exists, it
fails closed without issuing an action.

Localized reward names are presentation only and are never used for matching.
An already selected target is accepted without a redundant selection call.
Otherwise the game's menu-change path performs selection, scrolling, and row
rebinding.

After selection, the state machine waits for the live selected/current indices
to acknowledge the target and for the conservative UI-settle interval. It then
requests the reward menu's exact decide callback once.

### TOTAL RESULTS

In the observed floor-five flow, reward selection transitions directly to
`TOTAL RESULTS`; no reward-conversion dialog appears. The same result-owner
object families remain allocated with changed state fingerprints, while
`ControllerResultReward` and `DialogRewardResult` remain absent. The state
machine therefore requires the verified direct transition and does not wait
for or attempt a speculative confirmation dialog.

TOTAL RESULTS is advanced only when its animation controller reports the
verified ready state. After that observation and its settle interval, the hook
publishes the dedicated `ToNext` event once. Event publication alone is not
success: the expected return-destination controller must subsequently appear.

### Tredame return and gate

The return-destination menu selects and confirms Tredame Palace through its
native menu path. The design does not identify a destination from localized
text or screen pixels.

After Tredame Palace loads, the exact live palace `BaEndlessPortal` gate object
must initialize and remain valid through the configured settle interval. The
adapter distinguishes it from route and final-boss portals using independently
verified place and phase state. Its internal interaction function is then
called once without synthesizing player movement or input. The party-formation
screen relationship must subsequently become active before the gate-stage
deadline. A merely allocated `ControllerPresetParty` is insufficient: live
observation showed its bounded fingerprint unchanged across the transition,
while `ControllerEndlessDifficulty`, `ControllerEndlessTop`,
`ControllerEndlessTopFrame`, and `DialogRewardResult` appeared together.

### Party and depth

The verified party-formation relationship keeps the current party by invoking
the exact `탐사 시작` action once; it does not edit party slots. The difficulty
menu confirms the currently focused depth through its exact confirm callback;
adjacent cancel callbacks are excluded by target validation.

In the observed floor-five flow, depth confirmation transitions directly into
battle; there is no separate final-ready prompt. The cycle returns to `Armed`
only after the difficulty/top/frame UI, palace portal, and dialog-reward
relationship become inactive and the battle successor is observed. It never
re-arms merely because the depth-confirm callback returned success. Hidden
`ControllerPresetParty`, `ControllerEndlessEventResult`, or
`ControllerEndlessBossResult` allocations do not block acknowledgement and
cannot authorize an action.

## Timing and Duplicate Suppression

Timing values are internal, version-specific constants in the first release.
They are not user-configurable in the initial UI. Live probe evidence determines
their conservative defaults.

Every stage records:

- the earliest permitted action time;
- the absolute stage deadline;
- whether its one-shot native action has been issued;
- the controller/menu identity that owns the current cycle.

Repeated update callbacks, controller reuse, and duplicate observations cannot
issue the same action twice. A new reward cycle cannot reuse an object identity
processed by the previous cycle. Switching OFF clears pending identities,
deadlines, and one-shot flags before any later callback can act.

## Failure Handling and Status

The public status contains:

- `state`: `Unavailable`, `Off`, or `On`;
- `stage`: the current stage when `On`, otherwise absent;
- `reason`: a stable machine-readable failure/unavailability reason;
- `last_successful_stage`: the last acknowledged stage after a runtime failure.

Unavailable reasons distinguish version mismatch, missing signature, ambiguous
signature, control-pipe failure, and internal initialization failure. Runtime
reasons distinguish reward validation, TOTAL RESULTS, return destination, gate,
party formation, difficulty confirmation, battle re-entry, timeout, and
user/control disconnect.

Any runtime validation error, unacknowledged transition, or timeout atomically:

1. prevents further native actions;
2. clears all pending controller identities and deadlines;
3. changes the requested enabled flag to false;
4. publishes `Off` with the failure reason and last successful stage.

The state remains failed-Off until the user explicitly enables it again. A new
enable clears the prior runtime failure only if the optional capability is still
available. Initialization failures remain `Unavailable`.

## Debug Probe and Promotion Gate

No action-capable candidate is added directly from the downloaded binary's
metadata. Each required boundary first exists as a debug-only, opt-in probe that
observes but does not invoke or modify the game UI:

- final reward controller/menu update;
- TOTAL RESULTS ready/update boundary;
- return-destination controller;
- Tredame gate initialization and update;
- party-formation controller;
- difficulty controller and battle re-entry boundary.

Probe logs contain only a fixed event name, a process-local call count, and a
fixed validation outcome. They contain no addresses, pointers, player names,
reward contents, inventory values, configured reward IDs, or preferences.

Offline/private live evidence must show each positive boundary at the expected
point and show no false positive during battle, ordinary results, unrelated
dialogs, fall recovery, boss mechanics, normal town interaction, or a fresh
process restart. Only candidates that pass the checklist are promoted to the
production native adapter.

## Automated Testing

### Pure state-machine tests

Use a fake monotonic clock and a fake native-action sink. Tests cover:

- the complete acknowledged happy path and return to `Armed`;
- configured floor-five reward ID selecting the first matching selectable row;
- acquisition limits having no effect on reward priority;
- a missing or unselectable configured reward falling back to the first
  selectable row;
- no selectable reward failing closed;
- a preference change applying to the current screen only before action issue;
- localized reward names having no effect on matching;
- an allowlisted blocker suspending the current stage and issuing one OK;
- dismissal resuming the suspended stage or its verified successor;
- repeated blocker observations never issuing duplicate OK actions;
- an unrelated or unknown single-OK dialog remaining untouched;
- an unknown modal timing out to `unknownBlockingDialog`;
- highest Power grade priority and leftmost tie fallback;
- a Power screen containing Chaos choosing display index zero without grade
  comparison;
- Power selection ignoring hidden/template menu objects;
- repeated Power update callbacks issuing one selection and one confirmation;
- monk interaction taking priority over two already allocated route portals;
- each affordable monk purchase selecting from exactly five visible Powers;
- repeated monk purchases until the game reports the purchase subflow ended;
- insufficient CP relying on the game's automatic subflow exit rather than a
  speculative cancel;
- the acquisition OK being issued once per purchased Power;
- the monk root menu closing before route selection becomes eligible;
- the exact active boss result being acknowledged once;
- a boss result without the expected Power transition timing out fail-closed;
- a hidden/inactive boss result never starting acknowledgement;
- the exact final-boss return gate starting one interaction;
- an ordinary single portal never being treated as the final-boss return gate;
- a hidden town `ControllerPresetParty` never authorizing party start;
- the verified party-screen relationship issuing `탐사 시작` once without
  changing party slots;
- depth confirmation transitioning directly to battle without a second ready
  action;
- a successful depth callback without UI dismissal timing out fail-closed;
- hidden preset-party and result-controller allocations not blocking the
  acknowledged battle transition;
- exactly one combat portal winning over a non-combat portal;
- post-boss continuation always winning over return;
- a rare-Power appearance-rate modifier winning over route-type priority;
- both-combat and neither-combat portal pairs falling back to slot zero;
- one active portal being selected without consulting a stale second icon;
- zero active portals causing no action and more than two failing closed;
- duplicate portal updates issuing one interaction;
- an inactive or merely allocated result controller never issuing OK;
- the exact active area-result controller issuing OK once;
- generic and unrelated confirmation dialogs remaining untouched;
- already selected reward without a redundant selection action;
- off-screen reward selection through the abstract menu-change action;
- every stage refusing observations from the wrong controller identity;
- duplicate update callbacks issuing each native action once;
- publication success without the next controller timing out and disabling;
- a timeout at every stage;
- user disable cancelling every pending action;
- control disconnect cancelling every pending action;
- a runtime failure requiring an explicit new enable;
- an unavailable capability never becoming enabled.

### Hook and protocol tests

- Prove every required signature matcher accepts exactly one candidate and
  rejects zero or multiple candidates.
- Test callback target/module and index-bound validation with byte fixtures.
- Test that detours preserve the original game call and only enqueue
  observations/actions in the documented order.
- Prove existing `Message` serialization bytes remain unchanged.
- Round-trip every new control request, correlated response, stage, and reason.
- Prove optional auto-reentry setup failure does not change core
  `HookStatus::Ready`.

### Backend and frontend tests

- Test initial OFF, unavailable, enabled-stage, failed-Off, pending, and
  disconnect presentation.
- Test the dedicated `극돈공소` route, automatic-run switch, and non-clearable
  floor-five reward dropdown.
- Test that reward IDs persist while automatic execution starts OFF for every
  game process.
- Test preference synchronization before enabling and stale configuration
  responses not replacing newer selections.
- Test that only the dedicated page sends enable/disable commands.
- Test localized presentation for the blocking-dialog stage and
  `unknownBlockingDialog` failure reason.
- Test that stale correlated responses cannot replace newer status.
- Test that no duplicate control is added to Settings.
- Keep the existing repeat-quest control and updater restoration behavior
  unchanged.
- Extend the repository security regression so the new Tauri module contains no
  remote process write, memory-operation, allocation, or thread-creation API.

### Required verification

Run focused tests first, followed by:

- `npm.cmd run format-check`
- `npm.cmd run lint`
- `npm.cmd run tsc`
- `npm.cmd test -- --run`
- `npm.cmd run build`
- `cargo build --release --locked --package hook`
- `cargo test --workspace --all-targets --locked`

Packaging and hash-document updates are outside ordinary implementation
verification unless the user explicitly requests a release build.

## Live Acceptance

The dedicated offline/private checklist must include:

- the final-boss field return gate and transition to the reward screen;
- one floor-three validation cycle using first-selectable reward fallback;
- one floor-five reward list with the configured reward below the visible top
  row;
- one floor-five reward list without the configured reward, proving first-
  selectable fallback;
- one row with an acquisition-count display, proving that count does not alter
  selection;
- every initial allowlisted blocking dialog appearing once and being dismissed
  exactly once;
- an unrelated single-OK dialog remaining untouched and producing a
  fail-closed timeout when it blocks progression;
- the direct reward-selection-to-TOTAL-RESULTS transition and TOTAL RESULTS
  animation completion;
- Tredame return, gate activation, current-party retention, focused-depth
  confirmation, and direct battle re-entry;
- two complete consecutive cycles;
- manual disable at every stage where it can be performed safely;
- a forced timeout or rejected validation that produces failed-Off;
- app restart while the game remains running;
- full game-process restart and confirmation that the feature starts OFF;
- normal battle, result, town, and unrelated-dialog negative cases;
- existing meter, equipment, item analysis, battle records, and unlimited
  repeat quest regression checks.

Compatibility remains unverified until every required row records an observed
result and PASS/MISMATCH outcome.

## Follow-up TODOs

These are deliberately excluded from this implementation:

- Fast Conflux inactivity timer reduction;
- reward catalogs and preferences for floors other than five.

Each follow-up must remain independently switchable and must not become an
implicit dependency of auto re-entry.

# Conflux Auto Re-entry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking. Do not use subagents unless the user
> explicitly requests delegation.

**Goal:** Add a separately controlled, fail-closed `극돈공소 자동 재진입`
feature that always continues instead of returning at the post-boss fork,
spends available CP through the `공무의 경지` monk before leaving, prefers
rare-Power route modifiers and then combat portals, selects Power index zero
when Chaos is present or otherwise the highest grade with a lowest-index tie
break, selects the preferred final reward, returns through Tredame Palace, and
re-enters the currently focused depth without importing any downloaded mod
binary.

**Architecture:** Independently identify and validate every Granblue Fantasy:
Relink 2.0.2 UI/FSM boundary with a debug-only observation probe before adding
native actions. After the live gate passes, keep the deterministic state machine
and control protocol testable outside the game, call native functions only from
validated game-thread detours in `hook.dll`, and expose a dedicated local duplex
control pipe to the Tauri backend and React sidebar.

**Tech Stack:** Rust nightly-2024-05-04, retour, pelite, interprocess named
pipes, tokio, append-compatible bincode types, Tauri 1, React 18, TypeScript,
Vitest, Windows x64, offline/private live validation.

## Global Constraints

- Target only Granblue Fantasy: Relink Endless Ragnarok 2.0.2 on Windows x64.
- Pin the executable SHA-256 to
  `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F`.
- Do not import, execute, redistribute, decompile into source, or copy
  implementation constants from the downloaded mod DLLs. Their documented
  behavior is reference material only.
- Do not add Reloaded-II, Reloaded.Hooks, or .NET runtime dependencies.
- The feature starts OFF for every game process and is never persisted.
- A control-client disconnect, timeout, invalid object, or unacknowledged
  transition disables all further auto-reentry actions.
- Native game functions may be called only from the game thread after exact
  controller, vtable, state, bounds, callback-owner, and event validation.
- Auto-reentry hook failure is optional-capability failure. It must not change a
  valid core meter `HookStatus::Ready` to `Unsupported`.
- Keep the existing send-only damage/event pipe and every existing `Message`
  variant byte-compatible. New control types are standalone types.
- Do not synthesize keyboard or mouse input.
- Do not expose timing controls in the first UI. Probe evidence determines
  conservative internal constants.
- Never read, modify, stage, or commit `logs.db`.
- Do not stage or commit any file without explicit user approval.
- Do not claim game compatibility from automated tests, successful builds, or
  an incomplete live checklist.
- Live validation requires an offline or private session. Do not launch, stop,
  or control the game without explicit user instruction.
- The following remain follow-up TODOs only: Fast Conflux timer reduction and
  completed-favorite removal.

---

## File and Responsibility Map

### New files

- `docs/research/2026-07-25-conflux-auto-reentry-candidates.md`
  - Independent reverse-engineering record for exact 2.0.2 candidates.
- `docs/testing/game-2.0.2-conflux-auto-reentry-probe.md`
  - Probe and production live-evidence contract.
- `src-hook/src/conflux_retry/mod.rs`
  - Optional capability setup, shared runtime handle, and game-thread entry
    points.
- `src-hook/src/conflux_retry/state.rs`
  - Pure fail-closed state machine with no FFI or process access.
- `src-hook/src/conflux_retry/native.rs`
  - Version-pinned layouts, safe reads, object validation, detours, and native
    action dispatch.
- `src-hook/src/conflux_retry/probe.rs`
  - Debug-only observation counters and fixed diagnostics.
- `src-hook/src/conflux_retry/control.rs`
  - Local duplex control server; it never calls game functions.
- `src-tauri/src/conflux_retry.rs`
  - Persistent control-pipe client, Tauri state, and invoke commands.
- `src/pages/useConfluxRetry.ts`
  - Frontend status and mutation hook.
- `src/pages/useConfluxRetry.test.tsx`
  - Frontend request-ordering and status tests.
- `src/pages/Logs.confluxRetry.test.tsx`
  - Dedicated sidebar and localization regression tests.

### Existing files modified

- `src-hook/Cargo.toml`
  - Probe feature and SHA-256 dependency for exact executable validation.
- `src-hook/src/lib.rs`
  - Start the control server beside the unchanged event server.
- `src-hook/src/hooks/mod.rs`
  - Install the optional auto-reentry capability after required meter hooks.
- `src-hook/src/process.rs`
  - Add strict unique-match support used by the new capability.
- `protocol/src/lib.rs`
  - Add the control pipe constant and standalone control/status wire types.
- `src-tauri/src/main.rs`
  - Register Tauri state and commands.
- `src/cargoTargets.test.ts`
  - Protect debug-only probe gating.
- `src/securityConfiguration.test.ts`
  - Keep the Tauri control client free of remote mutation/injection APIs.
- `src/pages/Logs.tsx`
  - Render the dedicated switch and status text.
- `src/pages/Settings.localization.test.ts`
  - Require complete Korean and English copy.
- `src-tauri/lang/ko/ui.json`
  - Korean label, stages, and stable failure messages.
- `src-tauri/lang/en/ui.json`
  - English label, stages, and stable failure messages.

---

### Task 1: Establish the independent 2.0.2 native candidate contract

**Files:**

- Create:
  `docs/research/2026-07-25-conflux-auto-reentry-candidates.md`
- Create:
  `docs/testing/game-2.0.2-conflux-auto-reentry-probe.md`

**Interfaces:**

- Consumes: the pinned 2.0.2 executable, public Relink modding references, the
  approved behavior sequence, and existing Djeeta MOD signature-scanning
  conventions.
- Produces: an exact candidate table for Task 2 containing signature and ABI
  contracts for observation-only hooks. It produces no product code and no
  native action authorization.

- [ ] **Step 1: Verify the research input**

Resolve the installed or running `granblue_fantasy_relink.exe` without writing
to it. Calculate SHA-256 and stop if it is not:

```text
63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F
```

Record the resolved executable path only in local working notes, not in the
checked-in research document. Record the version and hash in the document.

- [ ] **Step 2: Identify each observation boundary independently**

Using the current executable and public FSM/controller naming references,
identify these boundaries:

1. one/two active route portals, stale-icon exclusion, continuation/return
   destination, rare-Power modifier, route-type enum, slot index, and
   interaction boundary;
2. Endless area-result controller, active state, and exact OK callback;
3. mid/final-boss result active state and exact OK callback;
4. `공무의 경지` monk interaction, root menu, CP confirmation, five-choice
   Power list, acquisition OK, automatic insufficient-CP exit, and root-menu
   close;
5. Power-selection owner, visible-choice list, Chaos type, grade, and selected
   index;
6. final reward row/controller update;
7. reward-conversion dialog update;
8. TOTAL RESULTS update and ready-state field;
9. return-destination dialog update;
10. Tredame Palace gate initialization and update;
11. party-formation update;
12. difficulty menu update;
13. final-ready update.

For every boundary, record:

- semantic owner and why it is not a generic UI callback;
- exact masked byte signature;
- exact `.text` match count, which must be `1`;
- x64 calling convention, parameter count/types, and return type;
- whether the observation must occur before or after the original function;
- exact vtable RVA and controller/menu relationship used for validation;
- relevant field offsets and their bounded validity rules;
- callback target or FSM event ID, when the boundary later needs an action;
- positive and negative scenarios that distinguish the boundary.

Do not use a constant solely because it appears in a downloaded DLL. Every
constant needs independent executable or live-object evidence.

- [ ] **Step 3: Apply the static gate**

Mark a candidate `STATIC PASS` only when:

- its signature matches exactly once in the pinned executable;
- its complete instructions support the recorded ABI;
- every followed call target lies inside the expected executable section;
- every offset is supported by more than one access site or by a named
  constructor/update relationship;
- adjacent cancel and unrelated dialog callbacks are explicitly distinguished.

If any required boundary lacks a `STATIC PASS`, stop the plan and report the
missing evidence. Do not create a guessed probe.

- [ ] **Step 4: Write the live probe checklist**

Create
`docs/testing/game-2.0.2-conflux-auto-reentry-probe.md` with:

- pinned version and hash;
- exact debug feature and opt-in build command;
- one row per positive boundary;
- negative rows for ordinary battle, ordinary quest results, unrelated dialogs,
  fall recovery, boss mechanics, town interaction, and process restart;
- columns for date, PID, start/end fixed counters, observed validation outcome,
  PASS/MISMATCH, and concise notes;
- a rule that a missing, duplicate, out-of-order, or negative-case call blocks
  production promotion;
- a separate later section for two complete production cycles and manual
  cancellation.

- [ ] **Step 5: Review the research diff**

Run:

```powershell
git diff --check -- `
  docs/research/2026-07-25-conflux-auto-reentry-candidates.md `
  docs/testing/game-2.0.2-conflux-auto-reentry-probe.md
```

Expected: exit `0`, no private paths, no downloaded binary code, no pointer
values, and no unresolved candidate.

---

### Task 2: Add strict scanning and an observation-only debug probe

**Files:**

- Modify: `src-hook/Cargo.toml`
- Modify: `src-hook/src/process.rs`
- Modify: `src-hook/src/hooks/mod.rs`
- Create: `src-hook/src/conflux_retry/mod.rs`
- Create: `src-hook/src/conflux_retry/probe.rs`
- Create: `src-hook/src/conflux_retry/native.rs`
- Modify: `src/cargoTargets.test.ts`

**Interfaces:**

- Consumes: Task 1 `STATIC PASS` signatures and ABI contracts.
- Produces: `conflux-retry-probe`, a debug-only feature that observes fixed
  boundaries without invoking game functions or sending product protocol
  messages.

- [ ] **Step 1: Read the test-quality rules**

Before changing tests, read:

```text
C:\Users\azyu\.codex\plugins\cache\openai-curated-remote\superpowers\6.2.0\skills\test-driven-development\writing-good-tests.md
```

For each planned test, state which production change would make it fail.

- [ ] **Step 2: Write failing unique-match tests**

Extract a pure helper in `src-hook/src/process.rs` with this contract:

```rust
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum SignatureMatchError {
    #[error("{label} signature count was {count}")]
    Count { label: &'static str, count: usize },
}

fn require_unique_match(
    label: &'static str,
    matches: impl IntoIterator<Item = usize>,
) -> Result<usize, SignatureMatchError>;
```

Add tests for zero, one, and two addresses. The one-address case returns the
address; zero and two return the exact count.

Run:

```powershell
cargo test --locked --package hook require_unique_match
```

Expected: FAIL because the helper and error do not exist.

- [ ] **Step 3: Implement strict in-module scanning**

Implement `require_unique_match` minimally and add:

```rust
pub fn search_unique_match_address(
    &self,
    label: &'static str,
    signature_pattern: &str,
) -> Result<usize, SignatureMatchError>;
```

It must scan executable code, collect at most two matches, return the absolute
address only for exactly one match, and never silently select the last match.

Run the focused test again. Expected: PASS.

- [ ] **Step 4: Write the failing feature-contract test**

Extend `src/cargoTargets.test.ts` to require:

```ts
expect(hookManifest).toContain("conflux-retry-probe = []");
expect(hookSetup).toContain('#[cfg(feature = "conflux-retry-probe")]');
expect(hookSetup).not.toContain("conflux_retry::install_required");
```

Also assert that the default release hook feature list does not include
`conflux-retry-probe`.

Run:

```powershell
npm.cmd test -- --run src/cargoTargets.test.ts
```

Expected: FAIL because the feature and guarded setup do not exist.

- [ ] **Step 5: Implement fixed probe accounting**

Add:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProbeEvent {
    RewardMenu,
    RewardConfirmation,
    TotalResults,
    ReturnDestination,
    GateInitialize,
    GateUpdate,
    PartyFormation,
    Difficulty,
    FinalReady,
}

#[derive(Default)]
struct ProbeCounters {
    counts: [u64; 9],
}

impl ProbeCounters {
    fn record(&mut self, event: ProbeEvent) -> u64;
}
```

Test that every event starts at `1`, repeated recording increments only its own
counter, and no counter can affect another event.

Diagnostics must have this bounded form:

```text
CONFLUX RETRY PROBE event=<fixed-name> call=<n> validation=<pass|reject>
```

Do not log addresses, pointers, object data, reward content, favorites, or raw
bytes.

- [ ] **Step 6: Install observation-only detours**

Put the exact Task 1 signatures, ABI aliases, static detours, and vtable/active
validation in `native.rs`. Under `conflux-retry-probe`, each detour:

1. preserves the Task 1 before/after-original ordering;
2. validates only enough live state to classify the fixed observation;
3. records one fixed counter result;
4. never calls a menu decision, interaction, selection, or FSM publication
   function.

Install probes from `hooks::setup_hooks` only under:

```rust
#[cfg(feature = "conflux-retry-probe")]
match conflux_retry::install_probe(&process) {
    Ok(()) => info!("Conflux auto-reentry observation probe enabled"),
    Err(error) => warn!("Conflux auto-reentry observation probe unavailable: {error}"),
}
```

Probe setup failure must not be propagated with `?`.

- [ ] **Step 7: Verify the probe build**

Run:

```powershell
cargo test --locked --package hook conflux_retry
npm.cmd test -- --run src/cargoTargets.test.ts
cargo build --release --locked --package hook --features hook/console,hook/conflux-retry-probe
```

Expected: all commands exit `0`. Review the produced diff and confirm no native
action call is present.

---

### Task 3: Validate every probe boundary in an offline/private live session

**Files:**

- Modify only with personally observed results:
  `docs/testing/game-2.0.2-conflux-auto-reentry-probe.md`

**Interfaces:**

- Consumes: Task 2 debug-only hook and Task 1 checklist.
- Produces: a complete `LIVE PASS` promotion gate, or a mismatch/incomplete
  result that stops Tasks 4-10.

- [ ] **Step 1: Use the live-probe workflow**

Read and follow:

```text
C:\Users\azyu\Documents\GBFR\.agents\skills\gbfr-live-probe-validation\SKILL.md
```

Ask the user to start the pinned game in an offline or private session. Do not
launch, stop, or control the game without explicit instruction.

- [ ] **Step 2: Build and inject only the debug probe**

Build:

```powershell
cargo build --release --locked --package hook --features hook/console,hook/conflux-retry-probe
Copy-Item -LiteralPath 'target/release/hook.dll' -Destination 'hook-dbg.dll' -Force
cargo build --locked --package gbfr-logs
```

Run the debug Djeeta MOD app only after the user confirms the session. The
existing debug-only DLL selection must inject `hook-dbg.dll`; do not package the
probe.

- [ ] **Step 3: Observe the complete positive sequence**

Record counter deltas while the user completes one Extreme Conflux result and
re-entry manually:

1. route selection, including a sole route, a continuation/return pair, a
   both-combat pair, and a rare-Power modifier;
2. area-result acknowledgement and dismissal;
3. mid/final-boss reward-result acknowledgement;
4. `공무의 경지` monk CP loop, including repeated purchases, five-choice
   selection, acquisition OK, automatic insufficient-CP exit, and root-menu
   close before either outgoing portal;
5. Power selection, including Chaos and a highest-grade tie;
6. final reward menu;
7. reward selection and conversion dialog;
8. TOTAL RESULTS readiness and progression;
9. return-destination dialog;
10. Tredame load and gate initialization/update;
11. party formation;
12. difficulty confirmation;
13. final ready.

Every boundary must appear at the expected stage, in order, with the expected
single-call or documented repeated-update behavior. Repeated update callbacks
must retain one stable validated object relationship.

- [ ] **Step 4: Observe required negative cases**

Record no accepted probe observation during ordinary battle activity, unrelated
dialogs, ordinary quest results, normal town interaction, fall recovery when
available, and a boss mechanic transition when available. Mark an unobserved
required case incomplete rather than PASS.

- [ ] **Step 5: Verify a fresh process**

Only after the user explicitly restarts the game, verify a new PID and exact
hash. Repeat the positive sequence through at least reward, return destination,
and gate initialization. Counters must restart and retain the expected
relationships.

- [ ] **Step 6: Apply the promotion gate**

If any required row is missing, duplicate beyond its documented update
behavior, out of order, or positive during a negative scenario:

- record MISMATCH or incomplete;
- remove only the confirmed repository-root `hook-dbg.dll` after resolving and
  checking its path;
- stop this implementation plan before Task 4.

If every required row passes, record `LIVE PASS`. Do not mark general 2.0.2
compatibility complete.

---

### Task 4: Define the standalone control/status wire contract

**Files:**

- Modify: `protocol/src/lib.rs`
- Modify: `protocol/tests/legacy_damage_wire.rs`

**Interfaces:**

- Consumes: Task 3 `LIVE PASS`.
- Produces:
  `CONFLUX_RETRY_PIPE_NAME`, `ConfluxRetryRequest`,
  `ConfluxRetryCommand`, `ConfluxRetryResponse`, and the public status enums
  consumed by Tasks 5, 7, 8, and 9.

- [ ] **Step 1: Write failing legacy and round-trip tests**

Preserve the existing serialized bytes for all `Message` variants. Add tests
that round-trip:

```rust
ConfluxRetryRequest {
    request_id: 42,
    command: ConfluxRetryCommand::SetEnabled(true),
}
```

and:

```rust
ConfluxRetryResponse {
    request_id: 42,
    status: ConfluxRetryStatus {
        state: ConfluxRetryState::On,
        stage: Some(ConfluxRetryStage::RewardSelection),
        reason: None,
        last_successful_stage: Some(ConfluxRetryStage::Armed),
    },
}
```

Run:

```powershell
cargo test --locked --package protocol
```

Expected: compile failure because the new standalone types do not exist.

- [ ] **Step 2: Add the exact public wire types**

Add:

```rust
pub const CONFLUX_RETRY_PIPE_NAME: &str =
    r"\\.\pipe\djeeta-mod-conflux-retry";

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfluxRetryCommand {
    GetStatus,
    SetEnabled(bool),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfluxRetryRequest {
    pub request_id: u64,
    pub command: ConfluxRetryCommand,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfluxRetryResponse {
    pub request_id: u64,
    pub status: ConfluxRetryStatus,
}
```

Define `ConfluxRetryState::{Unavailable, Off, On}` with lowercase JSON names.
Define the approved stages with camelCase JSON names:

```text
Armed
RewardSelection
RewardConfirmation
TotalResults
ReturnDestination
TredameGate
PartyFormation
DifficultyConfirmation
FinalReady
```

Define camelCase reasons:

```text
GameNotRunning
UnsupportedGame
SignatureMissing
SignatureAmbiguous
ControlUnavailable
InitializationFailed
RewardValidation
RewardConfirmation
TotalResults
ReturnDestination
TredameGate
PartyFormation
DifficultyConfirmation
FinalReady
Timeout
ControlDisconnected
Internal
```

Define `ConfluxRetryStatus` with camelCase field serialization:

```rust
pub struct ConfluxRetryStatus {
    pub state: ConfluxRetryState,
    pub stage: Option<ConfluxRetryStage>,
    pub reason: Option<ConfluxRetryReason>,
    pub last_successful_stage: Option<ConfluxRetryStage>,
}
```

- [ ] **Step 3: Prove append compatibility**

Do not add a new variant to `Message` or `LegacyMessage`. Run the full protocol
test suite and confirm the existing variant-index and legacy fixture tests are
unchanged and PASS.

---

### Task 5: Implement the pure fail-closed state machine with TDD

**Files:**

- Create: `src-hook/src/conflux_retry/state.rs`
- Modify: `src-hook/src/conflux_retry/mod.rs`

**Interfaces:**

- Consumes: Task 4 public status enums.
- Produces:

```rust
pub(crate) struct ConfluxRetryMachine;
pub(crate) enum Observation;
pub(crate) enum NativeAction;
pub(crate) struct RewardRow;
pub(crate) struct RewardMenuSnapshot;
```

with these methods:

```rust
impl ConfluxRetryMachine {
    pub(crate) fn unavailable(reason: ConfluxRetryReason) -> Self;
    pub(crate) fn available() -> Self;
    pub(crate) fn status(&self) -> ConfluxRetryStatus;
    pub(crate) fn set_enabled(
        &mut self,
        enabled: bool,
        now_ms: u64,
    ) -> ConfluxRetryStatus;
    pub(crate) fn observe(
        &mut self,
        observation: Observation,
        now_ms: u64,
    ) -> Option<NativeAction>;
    pub(crate) fn action_result(
        &mut self,
        action: NativeAction,
        succeeded: bool,
        now_ms: u64,
    );
    pub(crate) fn tick(&mut self, now_ms: u64);
    pub(crate) fn disconnect(&mut self);
}
```

- [ ] **Step 1: Write the failing reward-priority tests**

Use:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RewardRow {
    pub favorite: bool,
    pub selectable: bool,
}
```

Test:

- `[false, true, true]` chooses index `1`;
- no favorite chooses index `0`;
- a favorite on an unselectable row is skipped;
- a currently selected preferred index does not emit `SelectReward`.

Run:

```powershell
cargo test --locked --package hook conflux_retry::state::tests::reward
```

Expected: compile failure because the state machine does not exist.

- [ ] **Step 2: Implement the minimum reward transition**

Implement only `Off -> Armed -> RewardSelection`, bounded preferred-index
selection, and a one-shot `NativeAction::SelectReward` or
`NativeAction::DecideReward`. Use the snapshot's opaque `controller` and `menu`
identity only for equality and duplicate suppression; never log them.

Run the reward tests. Expected: PASS.

- [ ] **Step 3: Add failing full-sequence tests**

Build one acknowledged sequence:

```text
Armed
-> RewardSelection
-> RewardConfirmation
-> TotalResults
-> ReturnDestination
-> TredameGate
-> PartyFormation
-> DifficultyConfirmation
-> FinalReady
-> Armed
```

Assert the exact one-shot action order:

```text
SelectReward (only when needed)
DecideReward
AcceptRewardConfirmation
PublishToNext
SelectTredame
InteractGate
KeepCurrentParty
ConfirmFocusedDifficulty
PublishButton01
```

Require the next expected controller observation after every successful native
action. A successful function return alone must not advance two stages.

- [ ] **Step 4: Add failing duplicate and identity tests**

For every stage:

- repeat the same update observation and assert one native action;
- present a different controller/menu identity and assert no action;
- acknowledge an action from the prior cycle and assert no transition;
- complete one cycle, reuse its processed reward-menu identity, and assert no
  second cycle begins.

- [ ] **Step 5: Add failing cancellation and timeout tests**

Test:

- user disable from every stage clears stage, pending action, identities, and
  deadlines and returns clean `Off`;
- disconnect from every stage returns `Off` with
  `ControlDisconnected`;
- each stage deadline produces `Off`, `Timeout`, and the last acknowledged
  stage;
- `SetEnabled(true)` after runtime failure clears the old reason and returns
  `On/Armed`;
- an `Unavailable` machine never enables.

- [ ] **Step 6: Implement the complete minimum state machine**

Add internal per-stage deadlines, earliest-action times, the active cycle
identity, last acknowledged stage, and one issued-action marker. Centralize all
fail-closed cleanup in:

```rust
fn fail(&mut self, reason: ConfluxRetryReason);
```

`fail` must clear every identity, action marker, earliest time, and deadline
before publishing failed-Off status.

- [ ] **Step 7: Verify state-machine coverage**

Run:

```powershell
cargo test --locked --package hook conflux_retry::state
```

Expected: all tests PASS with no FFI or live process dependency.

---

### Task 6: Promote validated candidates to the native adapter

**Files:**

- Modify: `src-hook/Cargo.toml`
- Modify: `src-hook/src/conflux_retry/native.rs`
- Modify: `src-hook/src/conflux_retry/mod.rs`
- Modify: `src-hook/src/hooks/mod.rs`
- Modify: `src/cargoTargets.test.ts`

**Interfaces:**

- Consumes: Task 3 `LIVE PASS`, Task 5 `Observation` and `NativeAction`, and the
  Task 1 exact native contract.
- Produces:

```rust
pub(crate) fn install_optional(
    process: &Process,
    runtime: ConfluxRetryRuntime,
) -> Result<(), ConfluxRetryInstallError>;
```

and game-thread observation/action dispatch. It does not expose pointers to the
control thread.

- [ ] **Step 1: Write failing executable-hash tests**

Add a pure uppercase hexadecimal parser/comparator for the pinned hash. Test
exact match, one-bit mismatch, invalid length, and non-hex input.

Add `sha2 = "0.10"` to `src-hook/Cargo.toml` only after the test fails because
the verifier is missing. At setup, hash `std::env::current_exe()` once and
return `UnsupportedGame` before installing an action-capable detour on mismatch.

- [ ] **Step 2: Write failing layout-validation fixture tests**

For each live object family, create byte fixtures that exercise the pure
snapshot decoder and validator:

- correct exact vtable and active state;
- wrong vtable;
- inactive root;
- null or unreadable linked menu;
- vector end before begin;
- row count above the documented bound;
- selected/current index outside the row count;
- callback outside the executable `.text` range;
- adjacent cancel callback where confirm is required;
- wrong FSM event ID.

Each invalid fixture must return the stage-specific error without producing a
`NativeAction`.

- [ ] **Step 3: Implement safe bounded reads and validation**

Reuse the existing in-process `ReadProcessMemory(HANDLE(-1), ...)` approach
instead of directly dereferencing version-fragile cross-object pointers.
Implement typed decoding only after the complete required byte range was read.
Keep row count and pointer arithmetic checked.

Separate:

```rust
fn observe_reward_menu(...) -> Result<Observation, NativeValidationError>;
fn observe_reward_confirmation(...) -> Result<Observation, NativeValidationError>;
fn observe_total_results(...) -> Result<Observation, NativeValidationError>;
fn observe_return_destination(...) -> Result<Observation, NativeValidationError>;
fn observe_tredame_gate(...) -> Result<Observation, NativeValidationError>;
fn observe_party_formation(...) -> Result<Observation, NativeValidationError>;
fn observe_difficulty(...) -> Result<Observation, NativeValidationError>;
fn observe_final_ready(...) -> Result<Observation, NativeValidationError>;
```

from:

```rust
unsafe fn execute_native(action: NativeAction) -> Result<(), NativeActionError>;
```

- [ ] **Step 4: Write failing detour-order tests**

Extract small pure ordering helpers for the Task 1 verified before/after-original
contracts. Test that:

- the original function is called exactly once;
- observation is taken at the validated side of the original;
- a pending action executes only after validation;
- a validation error calls the original but fails the optional automation;
- no detour unwinds across the game FFI boundary.

- [ ] **Step 5: Implement action-capable detours**

Promote only Task 3 `LIVE PASS` signatures. Each detour catches internal Rust
errors, preserves the original call, submits one observation to the runtime,
and executes at most one state-machine action on that game-thread callback.

Implement exact actions:

- menu-change callback for the preferred reward index;
- reward menu decide callback;
- reward-conversion dialog decide callback;
- `ToNext` publication from the ready TOTAL RESULTS controller;
- native Tredame destination selection and decision;
- `EtEndlessModeCounter` interaction;
- current-party FSM event;
- exact focused-difficulty confirm callback;
- `Button01` publication from the live final-ready flow.

Do not add cached dialog invocation, localized text matching, pixel inspection,
or input synthesis.

- [ ] **Step 6: Keep the capability optional**

Create `ConfluxRetryRuntime::available()` before optional setup. On any install
error, call:

```rust
runtime.set_unavailable(error.reason());
warn!("Conflux auto-reentry unavailable: {error}");
```

Do not propagate the optional error through core `setup_hooks`.

Update the source-contract test to require optional `match` handling and reject
an `install_optional(...)?` call.

- [ ] **Step 7: Remove production dependence on the probe feature**

Keep `conflux-retry-probe` available for future diagnostics but ensure release
setup uses only the promoted production functions outside its `cfg` block. The
probe feature must remain absent from ordinary release commands.

- [ ] **Step 8: Verify the native adapter**

Run:

```powershell
cargo test --locked --package hook conflux_retry
npm.cmd test -- --run src/cargoTargets.test.ts
cargo build --release --locked --package hook
```

Expected: every command exits `0`; optional setup cannot affect core
`HookStatus`.

---

### Task 7: Add the hook-side local control server

**Files:**

- Modify: `src-hook/src/conflux_retry/mod.rs`
- Create: `src-hook/src/conflux_retry/control.rs`
- Modify: `src-hook/src/lib.rs`

**Interfaces:**

- Consumes: Task 4 control types and Task 5/6 `ConfluxRetryRuntime`.
- Produces:

```rust
pub(crate) async fn run_control_server(runtime: ConfluxRetryRuntime);
```

The server accepts one persistent local duplex client at a time, serializes
request/response frames, and disables the runtime on EOF or decode/write error.

- [ ] **Step 1: Write failing command-handler tests**

Test a pure async handler or synchronous command reducer:

- `GetStatus` returns the current status without mutation;
- `SetEnabled(true)` returns `On/Armed` when available;
- `SetEnabled(false)` returns clean `Off`;
- response request ID exactly matches request ID;
- malformed frames do not mutate enabled state;
- disconnect calls `runtime.disconnect()` exactly once.

Run:

```powershell
cargo test --locked --package hook conflux_retry::control
```

Expected: compile failure because the control server does not exist.

- [ ] **Step 2: Implement bounded frame exchange**

Create the listener with:

```rust
PipeListenerOptions::new()
    .path(protocol::CONFLUX_RETRY_PIPE_NAME)
    .mode(PipeMode::Bytes)
    .accept_remote(false)
    .create_tokio_duplex::<pipe_mode::Bytes>()
```

Use `LengthDelimitedCodec` with a small explicit maximum frame length sufficient
for the fixed request/response types. Decode only one request per frame and send
one correlated response.

- [ ] **Step 3: Enforce one control owner**

Handle a connected client to completion before accepting the next client. On
EOF, decode failure, or write failure, disable the runtime before accepting a
replacement client. A replacement connection observes OFF.

- [ ] **Step 4: Start control and event servers independently**

In `src-hook/src/lib.rs`, keep the existing event server behavior unchanged and
spawn `run_control_server(runtime)` beside it. A control-listener creation
failure sets the optional capability to `ControlUnavailable` but must not stop
the event server or change the core handshake.

- [ ] **Step 5: Verify event-pipe regression**

Run existing hook/protocol handshake tests plus:

```powershell
cargo test --locked --package hook
cargo test --locked --package protocol
```

Confirm the original `PIPE_NAME`, send-only listener, first `HookStatus`
message, and message framing are unchanged.

---

### Task 8: Add the persistent Tauri control client and commands

**Files:**

- Create: `src-tauri/src/conflux_retry.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src/securityConfiguration.test.ts`

**Interfaces:**

- Consumes: Task 4 wire types and Task 7 control server.
- Produces:

```rust
#[tauri::command]
pub(crate) async fn get_conflux_retry_status(
    state: tauri::State<'_, ConfluxRetryState>,
) -> Result<ConfluxRetryStatus, ()>;

#[tauri::command]
pub(crate) async fn set_conflux_retry_enabled(
    state: tauri::State<'_, ConfluxRetryState>,
    enabled: bool,
) -> Result<ConfluxRetryStatus, ()>;
```

- [ ] **Step 1: Write failing client-state tests**

Introduce a testable transport boundary:

```rust
#[async_trait]
trait ConfluxRetryTransport: Send {
    async fn exchange(
        &mut self,
        request: ConfluxRetryRequest,
    ) -> Result<ConfluxRetryResponse, ConfluxRetryClientError>;
}
```

If adding `async-trait` would be the only new backend dependency, use a boxed
future method instead and avoid the dependency.

With a fake transport, test:

- request IDs begin at `1` and increment;
- a matching response updates status;
- a mismatched response ID returns `Internal` and preserves the prior status;
- one mutation holds the async mutex for the complete exchange;
- connection failure maps to `Unavailable/ControlUnavailable`;
- absent game maps to `Unavailable/GameNotRunning`;
- a rejected enable never reports `On`.

- [ ] **Step 2: Implement the persistent duplex transport**

Use
`interprocess::os::windows::named_pipe::tokio::DuplexPipeStream<pipe_mode::Bytes>`
and `LengthDelimitedCodec`. Connect lazily on the first command after hook
injection and keep the framed stream inside the managed state for the app
lifetime.

On EOF or I/O failure:

- drop the stream so the hook observes disconnect and disables;
- cache `Unavailable/ControlUnavailable`;
- allow a later explicit status refresh to reconnect.

- [ ] **Step 3: Register state and commands**

Add `mod conflux_retry`, manage `ConfluxRetryState::default()`, and register the
two commands in `tauri::generate_handler!`.

Do not add startup persistence or automatically enable after connection.

- [ ] **Step 4: Write the failing security regression**

Extend `src/securityConfiguration.test.ts` to read the new Tauri module and
reject:

```text
PROCESS_VM_WRITE
PROCESS_VM_OPERATION
PROCESS_CREATE_THREAD
WriteProcessMemory
VirtualProtectEx
VirtualAllocEx
CreateRemoteThread
dll_syringe
```

Run:

```powershell
npm.cmd test -- --run src/securityConfiguration.test.ts
```

Expected before the final module implementation: FAIL because the new security
contract or source file is absent. Expected after implementation: PASS.

- [ ] **Step 5: Verify the backend**

Run:

```powershell
cargo test --locked --package gbfr-logs conflux_retry
npm.cmd test -- --run src/securityConfiguration.test.ts
```

Expected: PASS.

---

### Task 9: Add the dedicated management-sidebar switch

**Files:**

- Create: `src/pages/useConfluxRetry.ts`
- Create: `src/pages/useConfluxRetry.test.tsx`
- Create: `src/pages/Logs.confluxRetry.test.tsx`
- Modify: `src/pages/Logs.tsx`
- Modify: `src/pages/Settings.localization.test.ts`
- Modify: `src-tauri/lang/ko/ui.json`
- Modify: `src-tauri/lang/en/ui.json`

**Interfaces:**

- Consumes: Task 8 Tauri commands and Task 4 JSON status shape.
- Produces:

```ts
export type ConfluxRetryStatus = {
  state: "unavailable" | "off" | "on";
  stage: ConfluxRetryStage | null;
  reason: ConfluxRetryReason | null;
  lastSuccessfulStage: ConfluxRetryStage | null;
};

export default function useConfluxRetry(): {
  status: ConfluxRetryStatus | null;
  pending: boolean;
  setEnabled(enabled: boolean): Promise<void>;
};
```

- [ ] **Step 1: Write failing hook tests**

Model the tested request lifecycle after `useRepeatQuest`, then add stricter
ordering tests:

- initial status is null/pending until backend response;
- no persisted default is read;
- connection-state refresh obtains authoritative status;
- while state is `on`, a 250ms poll obtains authoritative stage changes;
- polling stops immediately after an `off` or `unavailable` response;
- one pending mutation blocks a second mutation;
- a late initial status response cannot replace a newer mutation response;
- enable rejection preserves failed/unavailable authoritative status;
- app connection loss never leaves the switch visually ON.

Run:

```powershell
npm.cmd test -- --run src/pages/useConfluxRetry.test.tsx
```

Expected: FAIL because the hook does not exist.

- [ ] **Step 2: Implement the minimum frontend hook**

Use a monotonically increasing local request generation in addition to backend
request correlation. Apply a response only when its generation is the newest.
Use one 250ms interval only while the authoritative state is `on`; clear it
before applying an `off` or `unavailable` response and on unmount. Do not use
local storage or the settings store.

- [ ] **Step 3: Write failing sidebar tests**

Mock `useConfluxRetry` and assert:

- `극돈공소 자동 재진입` is a separate switch immediately after `무한 퀘스트
반복`;
- clicking its switch calls only `setEnabled`;
- pending, null, and unavailable disable the switch;
- `On/RewardSelection` renders the localized current stage;
- a failed-Off status renders the localized reason and last successful stage;
- `gameNotRunning` is suppressed because the common header owns it;
- Settings contains no duplicate control;
- existing damage-meter and repeat-quest switches retain their behavior.

- [ ] **Step 4: Add exact localization keys**

Under `ui.game-features.conflux-retry`, add:

- `label`;
- `stage` entries for all nine public stages;
- `reason` entries for every public reason;
- `last-successful-stage`.

Korean label must be `극돈공소 자동 재진입`. English label must be
`Conflux Auto Re-entry`. Avoid claiming compatibility or safety in the copy.

- [ ] **Step 5: Render the switch and status**

Add `useConfluxRetry()` to `Logs.tsx`. Use a distinct door/re-entry icon already
available from `@phosphor-icons/react`. Stop click propagation exactly like the
existing switches.

Render:

- current stage as neutral `Text size="xs"` while ON;
- reason as red text while failed or unavailable, except `gameNotRunning`;
- last successful stage only when both a reason and last stage exist.

- [ ] **Step 6: Verify frontend scope**

Run:

```powershell
npm.cmd test -- --run `
  src/pages/useConfluxRetry.test.tsx `
  src/pages/Logs.confluxRetry.test.tsx `
  src/pages/Logs.repeatQuest.test.tsx `
  src/pages/Settings.localization.test.ts
```

Expected: PASS.

---

### Task 10: Full verification and live production acceptance

**Files:**

- Modify only with observed results:
  `docs/testing/game-2.0.2-conflux-auto-reentry-probe.md`
- Modify only when the new feature's status needs an explicit row:
  `docs/testing/game-2.0.2-smoke-test.md`

**Interfaces:**

- Consumes: Tasks 4-9 after Task 3 `LIVE PASS`.
- Produces: fresh automated evidence and offline/private production behavior
  evidence. It does not produce a release or compatibility claim.

- [ ] **Step 1: Run focused verification**

Run the focused test commands from Tasks 4-9 again. Every command must exit `0`
before broad verification.

- [ ] **Step 2: Run required frontend verification**

```powershell
npm.cmd run format-check
npm.cmd run lint
npm.cmd run tsc
npm.cmd test -- --run
npm.cmd run build
```

Expected: every command exits `0`.

- [ ] **Step 3: Run required Rust verification**

Load the Visual Studio developer environment if MSVC is not already available,
then run:

```powershell
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
```

Expected: both commands exit `0`.

- [ ] **Step 4: Review the final diff**

Run:

```powershell
git diff --check
git status --short
```

Review every changed file for:

- downloaded binaries or Reloaded-II/.NET dependencies;
- accidental `logs.db` access;
- generated package artifacts;
- secrets or private paths;
- protocol variant reordering;
- auto-enable persistence;
- unrelated formatting or cleanup;
- TODO scope leaking into the first implementation.

- [ ] **Step 5: Run the first production live cycle**

Use the `gbfr-live-probe-validation` skill and an offline/private session.
Confirm the feature starts OFF. Enable it from the management sidebar before
entering the final reward flow, then observe:

1. preferred favorite row or first-row fallback;
2. reward conversion;
3. TOTAL RESULTS;
4. Tredame return;
5. gate interaction;
6. current-party retention;
7. focused-depth confirmation;
8. final ready;
9. next Conflux entry.

Record actual outcomes without reward contents or pointer data.

- [ ] **Step 6: Run consecutive-cycle and cancellation cases**

Complete two consecutive cycles. Then exercise safe manual cancellation points
from the checklist. After OFF, no later native action may occur. Force or
observe one rejected validation/timeout and confirm failed-Off with the correct
last successful stage and explicit re-enable requirement.

- [ ] **Step 7: Verify restart contracts**

Exit only the Djeeta MOD app while the game remains running and confirm the hook
disables on control disconnect. Restart the app and confirm OFF. After the user
explicitly restarts the game, confirm the new process also starts OFF.

- [ ] **Step 8: Run regressions**

Confirm the meter, reward-boundary clearing, equipment analysis, item analysis,
battle records, item-acquisition notification, updater preparation, and
existing unlimited repeat quest remain functional.

- [ ] **Step 9: Record the gate honestly**

Mark each observed row PASS or MISMATCH. Leave unobserved rows incomplete. Do
not mark the general 2.0.2 smoke test compatible unless all of its independent
required rows are complete.

Packaging, release hash updates, commit, push, PR, and publication remain
outside this plan unless the user explicitly authorizes them.

# Conflux Reward Selection and Re-entry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the existing game-owned Conflux unattended progression with
configured final-reward selection, TOTAL RESULTS advancement, Tredame return,
gate activation, current-party confirmation, and game-focused depth re-entry.

**Architecture:** Keep the verified timer data patch in the Tauri backend and
put callback-driven reward/re-entry actions in an optional injected-hook
capability. Resolve every native boundary independently first; when a bounded
pass cannot prove it, use only the PDB and relevant IL from the pinned
user-supplied Infinite Retry archive as hypotheses, then revalidate them
against the pinned game executable and live offline/private states.

**Tech Stack:** Rust nightly-2024-05-04, Windows x64, retour, pelite,
interprocess named pipes, tokio, bincode, Tauri 1, React 18, TypeScript,
Mantine, Vitest.

## Global Constraints

- Target only Granblue Fantasy: Relink Endless Ragnarok 2.0.2 on Windows x64.
- Require executable SHA-256
  `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F`.
- Reference archive SHA-256 must be
  `02FE3756F47118D5F957EE597C4C4776877AE906A9D373A913C1D0D9FADCBA71`.
- Reference extraction and generated IL stay in a fresh temporary directory
  outside the repository and are removed after recording bounded findings.
- Never execute, inject, load, redistribute, or commit the reference DLL, PDB,
  reconstructed source, or its dependencies.
- Every reference-derived hypothesis must be relocated and independently
  validated in the pinned game executable.
- The game unattended option owns route, Power, monk, and ordinary single-OK
  progression.
- Do not synthesize keyboard, mouse, or player movement.
- Invoke native game functions only from a verified game-thread callback.
- Never use remote threads, remote allocation, localized text, screen
  coordinates, or allocation count as an action authority.
- The master switch starts OFF and is not persisted.
- Enable is transactional: arm the hook first, then shorten the timer; failure
  turns the hook OFF and restores the original timer configuration.
- Disable stops hook actions before restoring the timer.
- Hook capability failure must not change a valid core meter
  `HookStatus::Ready` to `Unsupported`.
- Existing gameplay `Message` variants remain in their current order.
- Never read, modify, stage, or commit `logs.db`.
- Ask before every commit, push, release, package, or other external write.
- Automated tests and builds do not establish game compatibility.
- Live validation is limited to the user's offline/private session.

---

## File and Responsibility Map

### New files

- `src-hook/src/conflux_retry/state.rs`
  - Pure progression state machine and duplicate/timeout suppression.
- `src-hook/src/conflux_retry/native.rs`
  - Version-pinned layouts, observations, signatures, and native actions.
- `src-hook/src/conflux_retry/control.rs`
  - Local-only control server; it updates atomics and never calls game
    functions.
- `src-hook/src/conflux_retry/probe.rs`
  - Feature-gated fixed diagnostics for observation-only live validation.
- `src-hook/src/conflux_retry/mod.rs`
  - Optional capability setup and game-thread update entry point.
- `src-tauri/src/conflux.rs`
  - Hook control client and transactional orchestration with
    `ConfluxTimerState`.
- `src-tauri/assets/conflux-rewards-2.0.2.json`
  - Version-pinned floor-five reward IDs and Korean/English display names.
- `src/pages/useConfluxAutomation.ts`
  - Authoritative master status, preference update, and switch mutation hook.
- `src/pages/useConfluxAutomation.test.tsx`
  - Frontend request ordering, rollback, and connection refresh tests.

### Existing files modified

- `docs/research/2026-07-25-conflux-auto-reentry-candidates.md`
  - Independent and reference-assisted evidence ledger.
- `docs/testing/game-2.0.2-conflux-auto-reentry-probe.md`
  - Observation and action promotion gates.
- `protocol/src/lib.rs`
  - Standalone control types and control-pipe name.
- `protocol/tests/legacy_damage_wire.rs`
  - Existing variant-index protection and standalone type round trips.
- `src-hook/Cargo.toml`
  - Observation-only `conflux-retry-probe` feature.
- `src-hook/src/lib.rs`
  - Start the optional control server beside the unchanged event server.
- `src-hook/src/hooks/mod.rs`
  - Call the optional game-thread observation/update entry point.
- `src-hook/src/process.rs`
  - Strict unique signature helper.
- `src-tauri/src/conflux_timer.rs`
  - Expose internal status/mutation methods to the master orchestrator.
- `src-tauri/src/main.rs`
  - Manage Conflux state and register master commands.
- `src-tauri/src/update_install.rs`
  - Stop hook automation before timer restoration and update installation.
- `src/App.tsx`
  - Retain the dedicated Conflux route.
- `src/pages/Conflux.tsx`
  - Master switch, reward dropdown, stage, and failure copy.
- `src/pages/Conflux.test.tsx`
  - Page behavior and accessibility tests.
- `src/pages/Logs.tsx`
  - Retain the dedicated sidebar entry.
- `src-tauri/lang/ko/ui.json`
  - Korean labels, stages, and stable failure reasons.
- `src-tauri/lang/en/ui.json`
  - English labels, stages, and stable failure reasons.
- `src/pages/Settings.localization.test.ts`
  - Complete translation contract.
- `src/securityConfiguration.test.ts`
  - Write/isolation and control-channel security assertions.
- `src/cargoTargets.test.ts`
  - Observation probe cannot enter the default release hook.

---

### Task 1: Resolve native boundaries with the bounded analysis ladder

**Files:**

- Modify:
  `docs/research/2026-07-25-conflux-auto-reentry-candidates.md`
- Modify:
  `docs/testing/game-2.0.2-conflux-auto-reentry-probe.md`
- Create from verified data:
  `src-tauri/assets/conflux-rewards-2.0.2.json`

**Interfaces:**

- Consumes: existing RTTI/vtable observations, the pinned game executable, and
  the pinned Infinite Retry archive only when the independent pass fails.
- Produces: `STATIC PASS` records for six boundaries and a non-empty
  floor-five catalog matching these exact deserialization types:

```rust
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfluxRewardCatalog {
    game_exe_sha256: String,
    rewards: Vec<ConfluxRewardCatalogEntry>,
}

#[derive(serde::Deserialize)]
struct ConfluxRewardCatalogEntry {
    id: u32,
    ko: String,
    en: String,
}
```

The generated JSON contains only IDs and names verified from the game data and
live final-reward rows.

- [ ] **Step 1: Reconfirm immutable inputs**

Run:

```powershell
Get-FileHash -Algorithm SHA256 -LiteralPath `
  'D:\SteamLibrary\steamapps\common\Granblue Fantasy Relink\granblue_fantasy_relink.exe'
Get-FileHash -Algorithm SHA256 -LiteralPath `
  'C:\Users\azyu\Downloads\drive-download-20260725T084431Z-1-001\GBFR-Conflux-Infinite-Retry.zip'
```

Expected: the game and archive hashes exactly match the two global constants.
Stop before extraction on any mismatch.

- [ ] **Step 2: Run one independent pass per boundary**

Use the existing candidate RVAs, vtables, callers, and read-only live probe to
resolve:

1. final reward rows, ID, selectable state, selected index, change, and decide;
2. TOTAL RESULTS ready state and next event;
3. Tredame return value and confirmation;
4. Tredame portal discriminator and interaction;
5. party screen active state and start confirmation;
6. difficulty screen active state and confirmation.

For each boundary, record the exact signature count, ABI, field bounds,
positive state, hidden/stale negative, accept/cancel distinction, and successor.
Mark it `STATIC PASS` only when every item is present.

- [ ] **Step 3: Trigger reference assistance only for failed boundaries**

Create a fresh temporary directory and extract only the pinned archive:

```powershell
$confluxReferenceDir = Join-Path ([System.IO.Path]::GetTempPath()) `
  ('djeeta-conflux-reference-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $confluxReferenceDir | Out-Null
Expand-Archive -LiteralPath `
  'C:\Users\azyu\Downloads\drive-download-20260725T084431Z-1-001\GBFR-Conflux-Infinite-Retry.zip' `
  -DestinationPath $confluxReferenceDir
```

Prefer an already installed `ildasm.exe`. If it is absent, install
`ilspycmd` into `$confluxReferenceDir\tools`, not a user or machine tool store:

```powershell
dotnet tool install ilspycmd --tool-path `
  (Join-Path $confluxReferenceDir 'tools')
```

Inspect PDB metadata and decompile only methods whose names or callers relate
to reward selection, results, city selection, gate interaction, party, or
difficulty. Write IL/decompiler output only inside `$confluxReferenceDir`.

- [ ] **Step 4: Convert reference findings into game hypotheses**

For each referenced method, record only:

```text
Reference method:
Behavioral boundary:
Candidate game function:
Candidate field/event:
Independent relocation evidence:
Positive/negative live evidence:
Promotion result:
```

Do not paste reconstructed method bodies into the repository. Re-run the
pinned executable xref/signature and live-state checks. A reference hint that
cannot be independently corroborated remains `REJECTED`.

- [ ] **Step 5: Generate and validate the reward catalog**

Generate `conflux-rewards-2.0.2.json` from the verified static reward table and
cross-check every ID against at least one live floor-five reward row. Reject
duplicate IDs, empty localized names, zero IDs, and a zero-length catalog.

Add a focused Rust deserialization test later in Task 7; at this stage validate
the JSON bytes:

```powershell
Get-Content src-tauri/assets/conflux-rewards-2.0.2.json -Raw |
  ConvertFrom-Json | Select-Object gameExeSha256,@{n='count';e={$_.rewards.Count}}
```

Expected: the pinned hash and a positive count.

- [ ] **Step 6: Remove reference artifacts**

Resolve the temporary path and verify it is under the system temporary
directory before removal:

```powershell
$resolvedReferenceDir = (Resolve-Path -LiteralPath $confluxReferenceDir).Path
$resolvedTempRoot = (Resolve-Path -LiteralPath ([System.IO.Path]::GetTempPath())).Path
if (-not $resolvedReferenceDir.StartsWith($resolvedTempRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw 'Reference directory escaped the temporary root'
}
Remove-Item -LiteralPath $resolvedReferenceDir -Recurse -Force
```

- [ ] **Step 7: Review the evidence gate**

Run:

```powershell
git diff --check -- `
  docs/research/2026-07-25-conflux-auto-reentry-candidates.md `
  docs/testing/game-2.0.2-conflux-auto-reentry-probe.md `
  src-tauri/assets/conflux-rewards-2.0.2.json
rg -n 'drive-download|Users\\azyu|\\.dll|\\.pdb|IL_[0-9A-Fa-f]+' `
  docs/research/2026-07-25-conflux-auto-reentry-candidates.md `
  src-tauri/assets/conflux-rewards-2.0.2.json
```

Expected: clean diff and no private path, binary name, PDB name, or pasted IL
instruction labels in checked-in artifacts.

- [ ] **Step 8: Commit only after explicit approval**

Ask the user to review Task 1 evidence. If approval is granted, stage only the
three listed files and commit with:

```powershell
git commit -m "docs: validate Conflux reward and re-entry boundaries"
```

---

### Task 2: Define standalone control types and pure state transitions

**Files:**

- Modify: `protocol/src/lib.rs`
- Modify: `protocol/tests/legacy_damage_wire.rs`
- Create: `src-hook/src/conflux_retry/state.rs`
- Create: `src-hook/src/conflux_retry/mod.rs`

**Interfaces:**

- Produces:

```rust
pub const CONFLUX_CONTROL_PIPE_NAME: &str = r"\\.\pipe\djeeta-mod-conflux";

pub enum ConfluxControlCommand {
    GetStatus,
    Configure { enabled: bool, reward_id: u32, revision: u64 },
}

pub struct ConfluxControlRequest {
    pub request_id: u64,
    pub command: ConfluxControlCommand,
}

pub struct ConfluxControlResponse {
    pub request_id: u64,
    pub status: ConfluxAutomationStatus,
}

pub struct ConfluxAutomationStatus {
    pub state: ConfluxAutomationState,
    pub stage: ConfluxAutomationStage,
    pub reason: Option<ConfluxAutomationReason>,
    pub reward_id: Option<u32>,
    pub revision: u64,
}

pub enum ConfluxAutomationState {
    Unavailable,
    Off,
    On,
}

pub enum ConfluxAutomationStage {
    Off,
    Armed,
    RewardSelection,
    TotalResults,
    ReturnDestination,
    TredameGate,
    PartyFormation,
    DifficultyConfirmation,
    Unavailable,
}

pub enum ConfluxAutomationReason {
    CapabilityUnavailable,
    InvalidPreference,
    InvalidObservation,
    UnexpectedSuccessor,
    TransitionTimeout,
    ControlDisconnected,
    Internal,
}
```

- [ ] **Step 1: Write protocol round-trip and legacy-index tests**

Add a test that serializes/deserializes `GetStatus` and `Configure`, checks
request IDs and revisions, and reasserts existing `Message` variant indices.

Run:

```powershell
cargo test --locked --package protocol
```

Expected: FAIL because the standalone control types do not exist.

- [ ] **Step 2: Implement the standalone protocol**

Add the exact types above after `PIPE_NAME`, derive
`Serialize, Deserialize, Debug, Clone, PartialEq, Eq`, and do not add a
`Message` variant.

Run the protocol tests. Expected: PASS.

- [ ] **Step 3: Write pure state-machine tests**

Define:

```rust
pub(crate) enum Observation {
    None,
    RewardReady { screen_id: u64, target_index: u32 },
    RewardSelected { screen_id: u64, selected_index: u32 },
    TotalResultsReady { screen_id: u64 },
    ReturnDestinationReady { screen_id: u64 },
    TredameLoaded,
    TredameGateReady { screen_id: u64 },
    PartyReady { screen_id: u64 },
    DifficultyReady { screen_id: u64 },
    BattleLoaded,
    Invalid,
}

pub(crate) enum RequestedAction {
    SelectReward(u32),
    ConfirmReward,
    AdvanceTotalResults,
    ConfirmTredame,
    ActivateGate,
    ConfirmCurrentParty,
    ConfirmFocusedDepth,
    Disable(ConfluxAutomationReason),
}
```

Tests must cover the complete sequence, first-selectable target supplied by the
native adapter, duplicate `screen_id`, out-of-order successors, timeout, OFF,
and re-arm after `BattleLoaded`.

Run:

```powershell
cargo test --locked --package hook conflux_retry::state
```

Expected: FAIL before implementation.

- [ ] **Step 4: Implement the minimal pure state machine**

Implement one action per stable screen ID, an explicit deadline per pending
transition, and no FFI. `disable()` clears the pending identity and preference
revision. `configure()` rejects zero reward IDs.

Run the focused hook tests. Expected: PASS.

- [ ] **Step 5: Commit only after explicit approval**

Ask for approval, then stage only the protocol and pure-state files and commit:

```powershell
git commit -m "feat: define Conflux retry control and state"
```

---

### Task 3: Add strict scanning and observation-only hook probes

**Files:**

- Modify: `src-hook/Cargo.toml`
- Modify: `src-hook/src/process.rs`
- Modify: `src-hook/src/hooks/mod.rs`
- Modify: `src-hook/src/lib.rs`
- Create: `src-hook/src/conflux_retry/native.rs`
- Create: `src-hook/src/conflux_retry/probe.rs`
- Modify: `src/cargoTargets.test.ts`

**Interfaces:**

- Consumes: Task 1 `STATIC PASS` records and Task 2 state types.
- Produces:

```rust
pub(crate) fn install_optional(process: &Process) -> Result<ConfluxRuntime>;
pub(crate) fn observe_and_step(runtime: &ConfluxRuntime);
```

The `conflux-retry-probe` feature records validation results but does not
dispatch `RequestedAction`.

- [ ] **Step 1: Write strict-signature tests**

Add pure tests for zero, one, and two matches:

```rust
pub(crate) fn require_unique_match(
    label: &'static str,
    matches: impl IntoIterator<Item = usize>,
) -> Result<usize, SignatureMatchError>;
```

Run:

```powershell
cargo test --locked --package hook require_unique_match
```

Expected: FAIL before implementation, then PASS after the minimal helper.

- [ ] **Step 2: Protect the debug feature**

Add `conflux-retry-probe = []` to `src-hook/Cargo.toml`. Extend
`src/cargoTargets.test.ts` to assert the feature is absent from default release
features and all probe setup is `#[cfg(feature = "conflux-retry-probe")]`.

Run:

```powershell
npm.cmd test -- --run src/cargoTargets.test.ts
```

Expected: FAIL before the feature guard, then PASS.

- [ ] **Step 3: Implement bounded native observations**

For every Task 1 boundary, create a typed validator that returns `Observation`
only after checking exact vtable, active state, vector/index bounds, callback
ownership, and successor context. Keep raw pointers private to `native.rs`.

Diagnostic output is limited to:

```text
CONFLUX RETRY PROBE boundary=<fixed-name> call=<n> validation=<pass|reject>
```

Do not log addresses, reward IDs, item names, row data, or raw bytes.

- [ ] **Step 4: Install optional observation detours**

Install after required meter hooks. Failure logs an optional capability warning
and does not propagate into `HookStatus::Unsupported`. Under the probe feature,
requested actions are counted but never called.

Run:

```powershell
cargo test --locked --package hook conflux_retry
cargo build --release --locked --package hook `
  --features hook/console,hook/conflux-retry-probe
npm.cmd test -- --run src/cargoTargets.test.ts
```

Expected: PASS and a build containing only observation-capable Conflux code.

- [ ] **Step 5: Record offline/private positive and negative observations**

Follow
`.agents/skills/gbfr-live-probe-validation/SKILL.md`. Record one positive and
one hidden/stale or unrelated negative for all six boundaries. Do not promote
any boundary whose counter is missing, duplicated outside documented updates,
or out of order.

- [ ] **Step 6: Commit only after explicit approval**

Ask for approval, stage only Task 3 files and the updated live checklist, then:

```powershell
git commit -m "feat: observe Conflux retry boundaries"
```

---

### Task 4: Implement final-reward selection and TOTAL RESULTS

**Files:**

- Modify: `src-hook/src/conflux_retry/native.rs`
- Modify: `src-hook/src/conflux_retry/state.rs`
- Modify: `src-hook/src/conflux_retry/mod.rs`
- Modify: `docs/testing/game-2.0.2-conflux-auto-reentry-probe.md`

**Interfaces:**

- Consumes: `RequestedAction::{SelectReward, ConfirmReward,
  AdvanceTotalResults}`.
- Produces native adapter methods:

```rust
fn select_reward(&self, index: u32) -> Result<(), NativeActionError>;
fn confirm_reward(&self) -> Result<(), NativeActionError>;
fn advance_total_results(&self) -> Result<(), NativeActionError>;
```

- [ ] **Step 1: Add action-gating tests**

Use fake observations and a fake adapter to prove:

- matching selectable ID chooses the lowest matching index;
- absent/unselectable preference chooses the first selectable row;
- no selectable row disables automation;
- selection acknowledgement precedes confirmation;
- repeated update callbacks issue no duplicate action;
- TOTAL RESULTS must be ready before advancing.

Run the focused state tests and expect them to fail before dispatch exists.

- [ ] **Step 2: Implement one-shot native dispatch**

Call only the exact Task 1 accept functions from the verified game-thread
update. Validate the controller and indices again immediately before each call.
After a call, wait for the state-machine successor rather than trusting the
return value.

- [ ] **Step 3: Run automated validation**

```powershell
cargo test --locked --package hook conflux_retry
cargo build --release --locked --package hook --features hook/console
```

Expected: PASS.

- [ ] **Step 4: Run one-action live promotions**

In the offline/private session:

1. select a non-first configured row;
2. confirm only after the selected index changes;
3. repeat with an absent preference and observe first-selectable fallback;
4. advance ready TOTAL RESULTS once;
5. show that an ordinary Power screen and unrelated result screen produce no
   action.

Record successor evidence after every action.

- [ ] **Step 5: Commit only after explicit approval**

Ask for approval, then commit only Task 4 files:

```powershell
git commit -m "feat: automate Conflux final rewards"
```

---

### Task 5: Implement Tredame return and re-entry

**Files:**

- Modify: `src-hook/src/conflux_retry/native.rs`
- Modify: `src-hook/src/conflux_retry/state.rs`
- Modify: `src-hook/src/conflux_retry/mod.rs`
- Modify: `docs/testing/game-2.0.2-conflux-auto-reentry-probe.md`

**Interfaces:**

- Consumes the remaining `RequestedAction` variants.
- Produces:

```rust
fn confirm_tredame(&self) -> Result<(), NativeActionError>;
fn activate_tredame_gate(&self) -> Result<(), NativeActionError>;
fn confirm_current_party(&self) -> Result<(), NativeActionError>;
fn confirm_focused_depth(&self) -> Result<(), NativeActionError>;
```

- [ ] **Step 1: Add successor and negative-state tests**

Prove:

- return confirmation requires the verified Tredame value;
- cancel and other destinations cannot dispatch;
- `TredameLoaded` must precede gate action;
- route and final-boss portals cannot satisfy `TredameGateReady`;
- hidden party controller cannot confirm;
- invalid or unfocused depth cannot confirm;
- `BattleLoaded` re-arms the cycle;
- every timeout disables automation.

- [ ] **Step 2: Implement each native action separately**

Promote one action at a time in this order:

1. `confirm_tredame`;
2. `activate_tredame_gate`;
3. `confirm_current_party`;
4. `confirm_focused_depth`.

After each implementation, run the focused state/hook tests and one
offline/private positive transition before implementing the next.

- [ ] **Step 3: Validate unrelated negatives**

Observe no action for an ordinary town gate, an in-run route portal, the
final-boss return gate, party editing, cancel, or a depth screen with no valid
focus.

- [ ] **Step 4: Commit only after explicit approval**

Ask for approval and, if granted:

```powershell
git commit -m "feat: automate Conflux re-entry"
```

---

### Task 6: Add the local control server and transactional Tauri master

**Files:**

- Create: `src-hook/src/conflux_retry/control.rs`
- Modify: `src-hook/src/conflux_retry/mod.rs`
- Modify: `src-hook/src/lib.rs`
- Create: `src-tauri/src/conflux.rs`
- Modify: `src-tauri/src/conflux_timer.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/update_install.rs`
- Modify: `src/securityConfiguration.test.ts`

**Interfaces:**

- Tauri commands:

```rust
get_conflux_automation_status()
get_conflux_reward_catalog()
set_conflux_reward_preference(reward_id: u32)
set_conflux_automation_enabled(enabled: bool)
```

- [ ] **Step 1: Write control ordering tests**

Use fake hook and timer backends to prove:

- enabling configures/arms hook before timer;
- timer failure sends hook OFF and restores timer;
- hook failure never changes the timer;
- disabling sends hook OFF before timer restore;
- disconnect and process replacement return OFF;
- update preparation stops the hook before timer restore.

- [ ] **Step 2: Implement the local-only control server**

Accept only local clients, frame bincode requests/responses with bounded frame
length, serialize configuration mutations, and correlate response
`request_id`. The control task updates atomic requested state only; it never
calls native game functions.

- [ ] **Step 3: Implement the Tauri orchestrator**

Load and validate the bundled catalog, reject reward IDs absent from it, and
perform the exact transactional ordering from Step 1. Remove direct frontend
registration of `set_conflux_timer_enabled`; keep the timer method internal to
the orchestrator.

- [ ] **Step 4: Extend security regression tests**

Assert:

- Tauri Conflux orchestration contains no `CreateRemoteThread`,
  `VirtualAllocEx`, or native game-function address;
- only `conflux_timer.rs` owns the timer data write;
- the observation probe contains no process-write APIs;
- control frames are bounded and remote clients are rejected.

- [ ] **Step 5: Run focused and workspace tests**

```powershell
cargo test --locked --package protocol
cargo test --locked --package hook conflux_retry
cargo test --locked --package gbfr-logs conflux
npm.cmd test -- --run src/securityConfiguration.test.ts
```

Expected: PASS.

- [ ] **Step 6: Commit only after explicit approval**

Ask for approval, then:

```powershell
git commit -m "feat: control Conflux automation"
```

---

### Task 7: Complete the Conflux page

**Files:**

- Create: `src/pages/useConfluxAutomation.ts`
- Create: `src/pages/useConfluxAutomation.test.tsx`
- Modify: `src/pages/Conflux.tsx`
- Modify: `src/pages/Conflux.test.tsx`
- Delete after migration: `src/pages/useConfluxTimer.ts`
- Delete after migration: `src/pages/useConfluxTimer.test.tsx`
- Modify: `src-tauri/lang/ko/ui.json`
- Modify: `src-tauri/lang/en/ui.json`
- Modify: `src/pages/Settings.localization.test.ts`

**Interfaces:**

- Frontend status:

```ts
type ConfluxAutomationStatus = {
  state: "unavailable" | "off" | "on";
  stage:
    | "off"
    | "armed"
    | "rewardSelection"
    | "totalResults"
    | "returnDestination"
    | "tredameGate"
    | "partyFormation"
    | "difficultyConfirmation"
    | "unavailable";
  reason: string | null;
  rewardId: number | null;
  revision: number;
};
```

- [ ] **Step 1: Write hook and page tests**

Test initial status, connection refresh, preference-before-enable ordering,
transactional failure, switch disable, dropdown persistence, non-clearable
selection, stage display, unavailable reason, and one invocation per action.

Run:

```powershell
npm.cmd test -- --run `
  src/pages/useConfluxAutomation.test.tsx `
  src/pages/Conflux.test.tsx
```

Expected: FAIL before the new hook and dropdown exist.

- [ ] **Step 2: Implement the master frontend hook**

Persist only `rewardId` under local-storage key
`djeeta-conflux-reward-id`; do not persist `enabled`. On connection, read
status and catalog, synchronize a valid preference, then leave the feature OFF
until the user explicitly enables it.

- [ ] **Step 3: Implement the page**

Render:

- game unattended-option requirement;
- master `자동 실행` switch;
- required reward dropdown;
- current stage or stable failure reason;
- compatibility warning until the live checklist passes.

Remove the prior follow-up copy for reward selection and re-entry.

- [ ] **Step 4: Run frontend verification**

```powershell
npm.cmd run format-check
npm.cmd run lint
npm.cmd run tsc
npm.cmd test -- --run
npm.cmd run build
```

Expected: PASS.

- [ ] **Step 5: Commit only after explicit approval**

Ask for approval, then:

```powershell
git commit -m "feat: configure Conflux reward automation"
```

---

### Task 8: Complete offline/private cycle validation

**Files:**

- Modify:
  `docs/testing/game-2.0.2-conflux-auto-reentry-probe.md`
- Modify:
  `docs/research/2026-07-25-conflux-auto-reentry-candidates.md`

**Interfaces:**

- Produces the final compatibility evidence for this capability.

- [ ] **Step 1: Run the full automated matrix**

```powershell
npm.cmd run format-check
npm.cmd run lint
npm.cmd run tsc
npm.cmd test -- --run
npm.cmd run build
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
git diff --check
```

Expected: every command exits `0`; existing warnings are recorded separately
and no new warning is introduced by Conflux files.

- [ ] **Step 2: Verify startup and teardown**

With the pinned game in the user's offline/private session:

1. app/game restart starts OFF;
2. enabling arms the hook and applies the 1/2-second timer;
3. disabling stops actions and restores 3/60/30 timer values;
4. app exit and update preparation stop hook actions and restore timer values;
5. reconnect requires explicit enable.

- [ ] **Step 3: Run two complete cycles**

Run one floor-three fallback cycle and one floor-five configured-reward cycle.
For each, record:

- selected reward ID/index and fallback decision;
- one action per stable screen identity;
- TOTAL RESULTS successor;
- Tredame destination;
- Tredame gate successor;
- unchanged party;
- game-focused depth;
- battle re-entry;
- re-armed status.

- [ ] **Step 4: Run failure cases**

Disable during every stage, select a preference absent from a captured row set,
show an unrelated result/dialog, and stop the app during a pending transition.
Every case must stop further hook actions and restore the timer configuration.

- [ ] **Step 5: Review final scope**

Run:

```powershell
git status --short
git diff --check
git diff --stat
rg -n 'drive-download|Users\\azyu|CreateRemoteThread|VirtualAllocEx' `
  src-hook/src/conflux_retry src-tauri/src/conflux.rs `
  docs/research/2026-07-25-conflux-auto-reentry-candidates.md
```

Confirm `logs.db` remains untracked and untouched, reference artifacts are
absent, no unrelated formatting is present, and no compatibility claim exceeds
the recorded checklist.

- [ ] **Step 6: Commit or package only after explicit approval**

Report the final diff, automated results, live evidence, skipped checks, and
remaining uncertainty. Ask separately before a final commit or canonical NSIS
package.

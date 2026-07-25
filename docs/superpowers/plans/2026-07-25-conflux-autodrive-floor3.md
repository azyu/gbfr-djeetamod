# Conflux Autodrive Floor-Three Validation Implementation Plan

> **Superseded on 2026-07-25:** The user chose game-owned unattended
> progression. Keep this detailed route/Power/dialog plan only as a follow-up
> reference. Execute
> `docs/superpowers/plans/2026-07-25-conflux-minimal-autodrive.md` instead.

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking. Do not use subagents unless the user
> explicitly requests delegation.

**Goal:** Ship a separately controlled, fail-closed Extreme Conflux autodrive
feature and validate its first live cycle on floor three while retaining the
floor-five reward preference UI.

**Architecture:** Promote each currently read-only UI boundary only after an
offline/private observation proves its exact active state, callback ABI, and
successor. Keep the deterministic policy in a pure Rust state machine, execute
native game functions only from validated game-thread callbacks in `hook.dll`,
and expose status/configuration through a separate local control pipe and a
dedicated React page.

**Tech Stack:** Rust nightly-2024-05-04, retour, pelite, interprocess named
pipes, bincode, Tauri 1, React 18, TypeScript, Mantine, Vitest, Windows x64.

## Global Constraints

- Target only Granblue Fantasy: Relink Endless Ragnarok 2.0.2 with SHA-256
  `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F`.
- Require an offline or private session for every live run.
- Treat the game-provided `극돈공소 무조작 시 설정` as manually ON; do not
  duplicate or modify it.
- Floor three is validation-only and always uses first-selectable reward
  fallback. Only floor-five rewards appear in the preference dropdown.
- Never infer active UI from allocation count alone.
- Never invoke an unverified callback or synthesize keyboard, mouse, or player
  movement.
- Start automatic execution OFF for every game process; persist only the
  floor-five reward preference.
- Do not read, modify, stage, or commit `logs.db`.
- Do not stage or commit without explicit user approval.
- Do not claim compatibility until the live checklist passes.

---

### Task 1: Complete the action-boundary observation gate

**Files:**

- Modify: `src-tauri/examples/probe_conflux_ui.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `docs/research/2026-07-25-conflux-auto-reentry-candidates.md`
- Modify: `docs/testing/game-2.0.2-conflux-auto-reentry-probe.md`

**Interfaces:**

- Consumes: pinned executable, verified vtable RVAs, and the recorded live
  screen sequence.
- Produces: exact unique signatures, callback ABIs, active-state predicates,
  action arguments, and successor predicates for every action used by Task 3.

- [ ] **Step 1: Add failing unit tests for bounded vtable/function metadata**

Test that the offline helper rejects a target outside `.text`, duplicate
signatures, an unreadable slot, and an RVA outside the pinned image.

- [ ] **Step 2: Run the focused example tests and verify RED**

Run:

```powershell
cargo test --locked --package gbfr-logs --example probe_conflux_ui
```

Expected: the new metadata tests fail because the helper is absent.

- [ ] **Step 3: Implement the minimal read-only metadata helper**

Add only bounded image-relative function metadata. Do not emit live object
addresses, memory contents, reward IDs, or player data.

- [ ] **Step 4: Run the focused tests and verify GREEN**

Run the command from Step 2 and require every example test to pass.

- [ ] **Step 5: Observe and promote each required boundary**

Record exact positive and negative evidence for:

1. route portal selection and final-boss/palace portal discrimination;
2. allowlisted area-result, mid-boss-result, and Power-acquired OK actions;
3. ordinary and monk Power choice vectors, types, grades, and indices;
4. monk root, CP confirmation, repeat, automatic insufficient-CP exit, and
   root close;
5. floor-five reward vector/ID and floor-three first-selectable fallback;
6. TOTAL RESULTS next action;
7. Tredame destination selection;
8. palace portal interaction;
9. party start;
10. focused-depth confirmation and direct battle successor.

For each boundary, require one exact `.text` match, x64 ABI proof through all
callers, two independent field references, accept/cancel separation, and a
visible successor observation. Keep the boundary observation-only until all
requirements pass.

- [ ] **Step 6: Re-run the static and live gates**

Task 3 native action dispatch may not begin while any required row remains
`BLOCKED` or `MISMATCH`. Tasks 2 and 4 may proceed because their pure policy,
wire, and presentation code cannot invoke or modify the game. Automated tests
alone cannot promote a native boundary.

### Task 2: Implement the pure protocol and state machine with TDD

**Files:**

- Modify: `protocol/src/lib.rs`
- Modify: `protocol/tests/legacy_damage_wire.rs`
- Create: `src-hook/src/conflux_retry/state.rs`
- Create: `src-hook/src/conflux_retry/mod.rs`
- Modify: `src-hook/src/lib.rs`

**Interfaces:**

- Produces:
  - `ConfluxControlRequest::{GetStatus, SetRewardPreference, SetEnabled}`
  - `ConfluxControlResponse { request_id, status }`
  - `ConfluxStatus { state, stage, reason, reward_id, config_revision }`
  - `ConfluxMachine::observe(Observation) -> Option<NativeAction>`

- [ ] **Step 1: Write failing protocol byte-compatibility and round-trip tests**

Protect every existing `Message` byte sequence and round-trip the new standalone
control types without appending to `Message`.

- [ ] **Step 2: Run protocol tests and verify RED**

```powershell
cargo test --locked --package protocol
```

- [ ] **Step 3: Implement the standalone control types**

Use a new pipe constant and bounded enums. Keep the existing event pipe and
`Message` ordering unchanged.

- [ ] **Step 4: Write failing state-machine policy tests**

Cover portal priority, Chaos/highest-grade Power choice, monk repetition,
allowlisted blockers, unknown-blocker fallback timeout, floor-five configured
reward, floor-three first-selectable fallback, direct TOTAL RESULTS,
Tredame/party/depth flow, duplicate suppression, disable, disconnect, timeout,
and direct battle acknowledgement.

- [ ] **Step 5: Run hook unit tests and verify RED**

```powershell
cargo test --locked --package hook conflux_retry::state
```

- [ ] **Step 6: Implement the minimal pure machine and verify GREEN**

The state machine must contain no process access, FFI, timer thread, or UI text
matching.

### Task 3: Add the validated hook-native adapter and local control server

**Files:**

- Create: `src-hook/src/conflux_retry/native.rs`
- Create: `src-hook/src/conflux_retry/control.rs`
- Modify: `src-hook/src/conflux_retry/mod.rs`
- Modify: `src-hook/src/hooks/mod.rs`
- Modify: `src-hook/src/process.rs`
- Modify: `src-hook/src/lib.rs`
- Modify: `src-hook/Cargo.toml`

**Interfaces:**

- Consumes only Task 1 promoted signatures/ABIs and Task 2
  `Observation`/`NativeAction`.
- Produces `ConfluxRetryRuntime`, optional capability status, game-thread
  dispatch, and a local-only request/response pipe.

- [ ] **Step 1: Write failing unique-signature, validator, and dispatch tests**

Test zero/duplicate signature rejection, vtable/owner/state/index validation,
wrong-stage refusal, one-shot dispatch, callback-return-without-successor
timeout, and optional-capability isolation.

- [ ] **Step 2: Run focused hook tests and verify RED**

```powershell
cargo test --locked --package hook conflux_retry
```

- [ ] **Step 3: Implement strict signatures and read-only observations**

Install only uniquely matched detours. Call the original first where Task 1
requires post-call observation.

- [ ] **Step 4: Implement native action dispatch**

Dispatch only from the validated game thread. Revalidate the complete object
relationship immediately before every call and wait for the documented
successor before advancing.

- [ ] **Step 5: Implement the local control server**

Reject remote clients, serialize requests, disable on disconnect, and require a
valid acknowledged preference before enabling.

- [ ] **Step 6: Run focused and workspace Rust tests**

```powershell
cargo test --locked --package hook conflux_retry
cargo test --workspace --all-targets --locked
```

### Task 4: Add the Tauri client and dedicated `극돈공소` page

**Files:**

- Create: `src-tauri/src/conflux_retry.rs`
- Modify: `src-tauri/src/main.rs`
- Create: `src/pages/Conflux.tsx`
- Create: `src/pages/useConfluxRetry.ts`
- Create: `src/pages/useConfluxRetry.test.tsx`
- Create: `src/pages/Conflux.test.tsx`
- Modify: `src/pages/Logs.tsx`
- Modify: `src/App.tsx`
- Modify: `src-tauri/lang/ko/ui.json`
- Modify: `src-tauri/lang/en/ui.json`
- Modify: `src/pages/Settings.localization.test.ts`
- Modify: `src/securityConfiguration.test.ts`

**Interfaces:**

- Tauri commands:
  - `get_conflux_retry_status`
  - `set_conflux_reward_preference`
  - `set_conflux_retry_enabled`
- Route: `/logs/conflux`

- [ ] **Step 1: Write failing hook/page/localization tests**

Require the sidebar route, process-local OFF switch, persisted non-clearable
floor-five dropdown, preference-before-enable ordering, pending-state
suppression, stage/reason presentation, and no duplicate control in Settings.

- [ ] **Step 2: Run focused frontend tests and verify RED**

```powershell
npm.cmd test -- --run src/pages/useConfluxRetry.test.tsx src/pages/Conflux.test.tsx
```

- [ ] **Step 3: Implement the Tauri client and commands**

The client may use only the local control pipe. It must not request game-process
write, operation, allocation, or remote-thread rights.

- [ ] **Step 4: Implement the page and preference storage**

Use Mantine `Switch` and non-clearable `Select`. Display localized names while
sending the pinned internal reward ID. Do not duplicate the game's inactivity
option.

- [ ] **Step 5: Run focused frontend and security tests**

Run the Step 2 command, localization tests, and
`src/securityConfiguration.test.ts`.

### Task 5: Verify floor-three autodrive, then floor-five reward policy

**Files:**

- Modify: `docs/testing/game-2.0.2-conflux-auto-reentry-probe.md`
- Modify: `docs/research/2026-07-25-conflux-auto-reentry-candidates.md`

- [ ] **Step 1: Run the ordinary build/test suite**

```powershell
npm.cmd run format-check
npm.cmd run lint
npm.cmd run tsc
npm.cmd test -- --run
npm.cmd run build
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
```

- [ ] **Step 2: Start a fresh offline/private floor-three run**

Manually set the game-provided inactivity option ON. Confirm Djeeta MOD starts
OFF, enable it from `/logs/conflux`, and use first-selectable reward fallback.

- [ ] **Step 3: Record one complete autonomous floor-three cycle**

Require ordered stage evidence, no manual UI/gameplay input after enable, direct
battle re-entry, and no duplicate native action.

- [ ] **Step 4: Record failure and negative cases**

Disable mid-stage, disconnect the app, present an unrelated single-OK dialog
when safe, and verify fail-closed behavior. Never manufacture a destructive
game state.

- [ ] **Step 5: Validate the floor-five configured reward**

Select a non-first floor-five reward in the app and prove internal-ID matching.
Then validate absent/unselectable fallback and acquisition-count independence.

- [ ] **Step 6: Review the final diff**

Check for unrelated changes, secrets, raw addresses, generated artifacts,
`logs.db`, stale assumptions, and scope creep. Do not commit until the user
explicitly approves.

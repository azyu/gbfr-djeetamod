# Battle-Start Damage Reset Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Clear all prior encounter damage data when Granblue Fantasy: Relink 2.0.2 begins normal quest entry or Play Again loading, before the next accepted hit.

**Architecture:** First validate the historical quest-load candidate as a debug-only, opt-in hook in an offline or private session. Only after it passes the positive and negative boundary cases, append `OnBattleStart` to the wire protocol, add an idempotent parser reset handler, and promote the validated hook to the required production hook set.

**Tech Stack:** Rust nightly-2024-05-04, retour detours, append-only bincode protocol, Tauri 1 events, Cargo tests, Windows offline/private live validation.

## Global Constraints

- First accepted hit starts an encounter.
- Inactivity must not split or hide a live encounter.
- The meter clears immediately before the reward UI and additionally at verified quest-entry or Play Again loading.
- Append protocol variants; never reorder existing bincode variants.
- `HookStatus::Unsupported` is latched; an unavailable required start hook must not later become ready.
- Never read, modify, stage, or commit `logs.db`.
- Do not claim game 2.0.2 compatibility from automated tests or one probe result.
- Live work requires an offline or private session; Codex must not launch, stop, or control the game without explicit user instruction.

---

### Task 1: Add a debug-only loading-boundary probe

**Files:**
- Modify: `src-hook/Cargo.toml`
- Modify: `src-hook/src/hooks/quest.rs`
- Modify: `src-hook/src/hooks/mod.rs`
- Create: `docs/testing/game-2.0.2-battle-start-probe.md`

**Interfaces:**
- Consumes: historical `ON_LOAD_QUEST_STATE`, `OnLoadQuestStateFunc`, and `OnLoadQuestState` detour, which remain disabled in normal builds.
- Produces: feature `battle-start-probe` and bounded `BATTLE START PROBE call=<n>` console observations only; no protocol message or product reset.

- [ ] **Step 1: Add the feature gate**

Add an empty `battle-start-probe = []` feature to `src-hook/Cargo.toml`. Do not add it to default, release, or `npm run dev` features.

- [ ] **Step 2: Write the failing feature-contract test**

Add a source-contract test in `src/cargoTargets.test.ts` that reads `src-hook/Cargo.toml` and `src-hook/src/hooks/mod.rs`, then asserts:

```ts
expect(hookManifest).toContain("battle-start-probe = []");
expect(hookSetup).toContain('#[cfg(feature = "battle-start-probe")]');
expect(hookSetup).not.toMatch(/OnBattleStartHook::new\([^)]*\)\.setup\(&process\)\?;/);
```

Run:

```powershell
npm.cmd test -- --run src/cargoTargets.test.ts
```

Expected: FAIL because the feature and guarded setup do not exist.

- [ ] **Step 3: Implement the probe-only hook**

Keep the historical candidate isolated behind `#[cfg(feature = "battle-start-probe")]`. Rename its public wrapper to `OnBattleStartProbeHook`, pass `event::Tx` only if required by existing hook construction, and log a monotonically increasing call number after the candidate original function returns:

```rust
#[cfg(feature = "battle-start-probe")]
static BATTLE_START_PROBE_CALLS: AtomicU64 = AtomicU64::new(0);

#[cfg(feature = "battle-start-probe")]
fn record_battle_start_probe_call() -> u64 {
    BATTLE_START_PROBE_CALLS.fetch_add(1, Ordering::Relaxed) + 1
}
```

The detour log must contain only the fixed event name and call number. Do not log pointers, player names, quest data, or raw memory.

In `setup_hooks`, install it only under the feature gate and keep failure non-product and explicit:

```rust
#[cfg(feature = "battle-start-probe")]
match quest::OnBattleStartProbeHook::new().setup(&process) {
    Ok(()) => info!("Battle-start candidate probe enabled"),
    Err(error) => warn!("Battle-start candidate probe unavailable: {error}"),
}
```

- [ ] **Step 4: Unit-test bounded probe accounting**

Reset the counter in the test, invoke `record_battle_start_probe_call()` twice, and assert `1` then `2`. Run:

```powershell
cargo test --locked --package hook battle_start_probe
npm.cmd test -- --run src/cargoTargets.test.ts
```

Expected: both commands PASS.

- [ ] **Step 5: Create the evidence contract**

Create `docs/testing/game-2.0.2-battle-start-probe.md` with:

- pinned game version `2.0.2` and executable SHA-256 `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F`;
- debug-only feature and exact opt-in build command;
- rows for normal quest entry, Play Again, battle activity, fall recovery, boss mechanics, result presentation, and a fresh process restart;
- expected positive count delta `1` for each loading event and delta `0` for every negative case;
- columns for date, PID, start/end call numbers, PASS/FAIL, and concise notes;
- an explicit rule that any missing, duplicate, or negative-case call is FAIL and blocks production promotion.

- [ ] **Step 6: Commit the probe**

```powershell
git add src-hook/Cargo.toml src-hook/src/hooks/quest.rs src-hook/src/hooks/mod.rs src/cargoTargets.test.ts docs/testing/game-2.0.2-battle-start-probe.md
git commit -m "test: probe the battle-start loading boundary"
```

---

### Task 2: Validate the candidate in a live offline/private session

**Files:**
- Modify only with personally observed results: `docs/testing/game-2.0.2-battle-start-probe.md`

**Interfaces:**
- Consumes: Task 1 debug-only feature and evidence contract.
- Produces: a PASS gate with observed positive/negative cases, or a FAIL gate that stops this plan before product changes.

- [ ] **Step 1: Confirm the session and executable**

Ask the user to start game build 2.0.2 in an offline or private session. Do not launch, stop, or control the game. Verify the running executable hash matches the evidence contract and record only PID and hash.

- [ ] **Step 2: Build and start only the required debug app**

Build the hook with:

```powershell
cargo build --release --locked --package hook --features hook/console,hook/battle-start-probe
Copy-Item -LiteralPath 'target/release/hook.dll' -Destination 'hook-dbg.dll' -Force
```

Build the debug Tauri executable without launching the game, then run it with the
repository root as its working directory so its existing debug-only
`hook-dbg.dll` selection injects the probe:

```powershell
cargo build --locked --package gbfr-logs
Start-Process -FilePath 'target/debug/gbfr-logs.exe' -WorkingDirectory (Get-Location)
```

Do not package or enable the probe in a release build. When observation is
finished, exit the debug app and remove only the exact repository-root
`hook-dbg.dll` file after confirming its resolved path remains under the
repository root.

- [ ] **Step 3: Observe positive cases**

Record the call counter immediately before and after:

1. one normal quest entry;
2. one Play Again selection and loading transition.

Each transition must increment the counter exactly once.

- [ ] **Step 4: Observe negative cases**

Confirm no increment during ordinary battle damage, fall recovery if available, one boss mechanic transition if available, and result presentation. Mark unavailable scenarios unobserved rather than passing them.

- [ ] **Step 5: Verify a fresh process**

Fully exit and restart only when the user explicitly performs the game restart. Verify a new PID, rediscover the candidate, and repeat one normal quest entry. The counter must again increment exactly once.

- [ ] **Step 6: Apply the gate**

If any required scenario is unobserved or has the wrong delta, record FAIL or incomplete and stop. Do not add `OnBattleStart` to the product protocol. If every required row passes, record PASS and commit only the evidence document:

```powershell
git add docs/testing/game-2.0.2-battle-start-probe.md
git commit -m "docs: validate the battle-start loading boundary"
```

---

### Task 3: Append the start message and reset parser state

**Files:**
- Modify: `protocol/src/lib.rs`
- Modify: `src-tauri/src/parser/v1/mod.rs`
- Modify: `src-tauri/src/main.rs`

**Interfaces:**
- Consumes: Task 2 PASS gate.
- Produces: appended `Message::OnBattleStart` and `Parser::on_battle_start_event()`.

- [ ] **Step 1: Write failing protocol and parser tests**

Extend the protocol variant-index test:

```rust
let battle_start = bincode::serialize(&Message::OnBattleStart).unwrap();
assert_eq!(&battle_start[..4], &13_u32.to_le_bytes());
```

Add a parser test that creates damage, SBA/stun state, multiple targets, skill breakdown and raw events through existing public event handlers, invokes `on_battle_start_event`, then asserts:

```rust
assert_eq!(parser.status, ParserStatus::Waiting);
assert_eq!(parser.derived_state.status, ParserStatus::Waiting);
assert_eq!(parser.derived_state.total_damage, 0);
assert_eq!(parser.derived_state.dps, 0.0);
assert_eq!(parser.derived_state.total_stun_value, 0.0);
assert!(parser.derived_state.party.is_empty());
assert!(parser.derived_state.targets.is_empty());
assert!(parser.encounter.raw_event_log.is_empty());
assert!(parser.encounter.player_data.iter().all(Option::is_none));
```

Invoke the handler a second time to prove idempotence, then submit one accepted hit and assert the new encounter contains only that hit.

Run:

```powershell
cargo test --locked --package protocol battle_start
cargo test --locked --package gbfr-logs battle_start
```

Expected: compile failure because the variant and handler do not exist.

- [ ] **Step 2: Append the wire message**

Append after `LocalEquipmentSnapshot` without moving any existing variant:

```rust
/// Quest entry or Play Again loading has begun.
OnBattleStart,
```

Do not modify `LegacyMessage`.

- [ ] **Step 3: Implement the parser reset**

Add one handler using existing reset helpers:

```rust
pub fn on_battle_start_event(&mut self) {
    self.encounter.reset_player_data();
    self.reset();
    self.update_status(ParserStatus::Waiting);
    self.emit_party_update();
    if let Some(window) = &self.window_handle {
        let _ = window.emit("encounter-update", &self.derived_state);
    }
}
```

Do not save the active encounter in this handler and do not alter inactivity behavior.

- [ ] **Step 4: Route the message**

Add the exhaustive match arm in `src-tauri/src/main.rs`:

```rust
protocol::Message::OnBattleStart => {
    state.on_battle_start_event();
}
```

- [ ] **Step 5: Verify and commit**

Run the focused commands from Step 1 and confirm PASS, then:

```powershell
git add protocol/src/lib.rs src-tauri/src/parser/v1/mod.rs src-tauri/src/main.rs
git commit -m "feat: reset damage at battle start"
```

---

### Task 4: Promote the validated boundary to the production hook

**Files:**
- Modify: `src-hook/src/hooks/quest.rs`
- Modify: `src-hook/src/hooks/mod.rs`
- Modify: `src-hook/Cargo.toml`
- Modify: `src/cargoTargets.test.ts`

**Interfaces:**
- Consumes: Task 2 PASS evidence and Task 3 `Message::OnBattleStart`.
- Produces: required `OnBattleStartHook` installed alongside damage and reward hooks.

- [ ] **Step 1: Write the failing ordering and setup tests**

Add a pure ordering test using the same `notify_before_original` pattern as the reward hook:

```rust
#[test]
fn battle_start_notification_precedes_loading_operation() {
    let calls = RefCell::new(Vec::new());
    notify_before_original(
        || calls.borrow_mut().push("notify"),
        || calls.borrow_mut().push("original"),
    );
    assert_eq!(*calls.borrow(), vec!["notify", "original"]);
}
```

Update the source-contract test to require `OnBattleStartHook::new(...).setup(&process)?;`. Run the focused hook and TypeScript tests; expected result is FAIL before promotion.

- [ ] **Step 2: Promote only the validated signature**

Rename the validated probe wrapper to `OnBattleStartHook`, retain the exact validated signature, remove the debug counter and `battle-start-probe` feature, and send before the original operation:

```rust
notify_before_original(
    || {
        super::reset_battle_identity_state();
        let _ = self.tx.send(Message::OnBattleStart);
    },
    || unsafe { OnBattleStart.call(a1) },
);
```

- [ ] **Step 3: Make the hook required**

Install the start hook with `?` in `setup_hooks`, before the reward hook. A missing start boundary must prevent `HookStatus::Ready`; do not add a fallback to identity refresh or an unverified auxiliary hook.

- [ ] **Step 4: Verify and commit**

```powershell
cargo test --locked --package hook battle_start
npm.cmd test -- --run src/cargoTargets.test.ts
git add src-hook/Cargo.toml src-hook/src/hooks/quest.rs src-hook/src/hooks/mod.rs src/cargoTargets.test.ts
git commit -m "feat: signal the verified battle-start boundary"
```

---

### Task 5: Full verification and packaged behavior check

**Files:**
- Modify only with observed results: `docs/testing/game-2.0.2-smoke-test.md`

**Interfaces:**
- Consumes: Tasks 3-4 product implementation.
- Produces: automated verification evidence and a packaged manual result without claiming broader compatibility.

- [ ] **Step 1: Run frontend verification**

```powershell
npm.cmd run format-check
npm.cmd run lint
npm.cmd run tsc
npm.cmd test -- --run
npm.cmd run build
```

Expected: every command exits 0.

- [ ] **Step 2: Run Rust verification**

```powershell
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
```

Expected: both commands exit 0.

- [ ] **Step 3: Build through the canonical package command**

Use `npm.cmd run package:nsis` only when the updater signing key and password are available. Stop only an exact `Djeeta MOD` process if locked files require it; never stop the game merely to package.

- [ ] **Step 4: Verify packaged behavior**

In an offline or private session, create visible damage, start Play Again, and confirm the meter becomes empty during loading before any next-battle hit. Repeat with a fresh normal quest entry. Confirm the first hit starts at its own damage total and reward-boundary clearing still works.

- [ ] **Step 5: Record only observed results**

Add or update the smoke-test row for battle-start clearing with the actual date, packaged artifact hash, and PASS/FAIL result. Leave unrelated checklist items unchanged. Commit only the intended code and observed evidence; generated release hash-document updates remain a separate release operation.

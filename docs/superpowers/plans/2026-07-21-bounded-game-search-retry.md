# Bounded Game Search and Retry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop game discovery after ten failed attempts, show a not-found state with a working retry button, and bound the named-pipe wait so stale processes cannot trap the connection lifecycle.

**Architecture:** A focused Rust module owns deterministic attempt, deadline, and single-flight decisions. The existing Tauri orchestration applies those decisions without rewriting parser message handling, while the management header invokes one retry command and continues to observe backend-emitted connection state.

**Tech Stack:** Rust, Tauri 1, dll-syringe 0.15, Tokio, React 18, TypeScript, Mantine 7, i18next, Vitest, Testing Library

## Global Constraints

- Process search attempts are exactly 10 per run.
- The first process search is immediate; failed searches are separated by 1 second.
- Named-pipe connection attempts are separated by 100 milliseconds and stop at 10 seconds.
- `not-found` is append-only in the public connection-state model.
- Only `not-found` exposes retry; retry state is not persisted.
- Existing injection, handshake, parser message handling, encounter behavior, and `HookStatus::Unsupported` latching remain unchanged.
- Compact meter remains visible only for `connected`.
- Repeat quest continues refreshing from connection events and does not repeat the common game-not-running message.
- No new icons, badges, animations, toasts, status colors, settings, or exponential backoff.
- Preserve unrelated working-tree changes and do not stage `logs.db`.

---

## File Structure

- Create `src-tauri/src/game_search.rs`: bounded search decisions, pipe-wait decisions, constants, and single-flight guard.
- Modify `src-tauri/src/main.rs`: append `NotFound`, register the search state and retry command, bound process/pipe polling, and preserve the existing parser dispatch block.
- Modify `src/types.ts`: append frontend `not-found` connection state.
- Modify `src/pages/Logs.tsx`: render and invoke the retry action only for `not-found`.
- Modify `src/pages/Logs.test.tsx`: cover not-found rendering, retry invocation, and duplicate-click prevention.
- Modify `src/pages/Settings.localization.test.ts`: assert complete bilingual connection/retry copy.
- Modify `src-tauri/lang/ko/ui.json`: Korean not-found and retry copy.
- Modify `src-tauri/lang/en/ui.json`: English not-found and retry copy.
- Verify `src/pages/useConnectionState.ts`, `src/pages/useCompactMeter.ts`, and `src/pages/useRepeatQuest.ts` without changing their event semantics.

### Task 1: Deterministic Search Lifecycle Decisions

**Files:**
- Create: `src-tauri/src/game_search.rs`
- Modify: `src-tauri/src/main.rs`

**Interfaces:**
- Produces: `PROCESS_SEARCH_ATTEMPTS`, `PROCESS_SEARCH_INTERVAL`, `PIPE_CONNECT_INTERVAL`, `PIPE_CONNECT_TIMEOUT`.
- Produces: `ProcessSearchBudget::new()` and `record(found: bool) -> ProcessSearchDecision`.
- Produces: `pipe_wait_decision(process_alive: bool, elapsed: Duration) -> PipeWaitDecision`.
- Produces: `GameSearchState::try_begin() -> Option<GameSearchRun>` with drop-based release.

- [ ] **Step 1: Write failing unit tests for the exact limits**

Create `src-tauri/src/game_search.rs` with the test module first. The tests define the intended private API before implementation:

```rust
#[cfg(test)]
mod tests {
    use super::{
        pipe_wait_decision, GameSearchState, PipeWaitDecision, ProcessSearchBudget,
        ProcessSearchDecision, PIPE_CONNECT_TIMEOUT, PROCESS_SEARCH_ATTEMPTS,
    };
    use std::time::Duration;

    #[test]
    fn tenth_missing_process_exhausts_the_search() {
        let mut budget = ProcessSearchBudget::new();

        for _ in 1..PROCESS_SEARCH_ATTEMPTS {
            assert_eq!(budget.record(false), ProcessSearchDecision::Retry);
        }
        assert_eq!(budget.record(false), ProcessSearchDecision::NotFound);
    }

    #[test]
    fn finding_a_process_on_the_tenth_attempt_wins_over_exhaustion() {
        let mut budget = ProcessSearchBudget::new();

        for _ in 1..PROCESS_SEARCH_ATTEMPTS {
            assert_eq!(budget.record(false), ProcessSearchDecision::Retry);
        }
        assert_eq!(budget.record(true), ProcessSearchDecision::Found);
    }

    #[test]
    fn search_run_guard_rejects_overlap_and_releases_on_drop() {
        let state = GameSearchState::default();
        let run = state.try_begin().expect("first run should start");

        assert!(state.try_begin().is_none());
        drop(run);
        assert!(state.try_begin().is_some());
    }

    #[test]
    fn dead_process_leaves_pipe_wait_immediately() {
        assert_eq!(
            pipe_wait_decision(false, false, Duration::ZERO),
            PipeWaitDecision::ProcessExited
        );
    }

    #[test]
    fn live_process_times_out_at_the_exact_deadline() {
        assert_eq!(
            pipe_wait_decision(false, true, PIPE_CONNECT_TIMEOUT - Duration::from_millis(1)),
            PipeWaitDecision::Retry
        );
        assert_eq!(
            pipe_wait_decision(false, true, PIPE_CONNECT_TIMEOUT),
            PipeWaitDecision::TimedOut
        );
    }

    #[test]
    fn connected_pipe_wins_before_the_deadline() {
        assert_eq!(
            pipe_wait_decision(true, true, PIPE_CONNECT_TIMEOUT - Duration::from_millis(1)),
            PipeWaitDecision::Connected
        );
    }
}
```

Add `mod game_search;` to `src-tauri/src/main.rs` so Cargo compiles the new module.

- [ ] **Step 2: Run the Rust tests and verify RED**

Run:

```powershell
cargo test --locked --package gbfr-logs game_search::tests
```

Expected: FAIL because the constants, types, and functions imported by the tests do not exist.

- [ ] **Step 3: Implement the minimal decision module**

Above the tests in `src-tauri/src/game_search.rs`, add:

```rust
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

pub const PROCESS_SEARCH_ATTEMPTS: u8 = 10;
pub const PROCESS_SEARCH_INTERVAL: Duration = Duration::from_secs(1);
pub const PIPE_CONNECT_INTERVAL: Duration = Duration::from_millis(100);
pub const PIPE_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, PartialEq, Eq)]
pub enum ProcessSearchDecision {
    Found,
    Retry,
    NotFound,
}

pub struct ProcessSearchBudget {
    attempts: u8,
}

impl ProcessSearchBudget {
    pub fn new() -> Self {
        Self { attempts: 0 }
    }

    pub fn record(&mut self, found: bool) -> ProcessSearchDecision {
        self.attempts = self.attempts.saturating_add(1);
        if found {
            ProcessSearchDecision::Found
        } else if self.attempts >= PROCESS_SEARCH_ATTEMPTS {
            ProcessSearchDecision::NotFound
        } else {
            ProcessSearchDecision::Retry
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PipeWaitDecision {
    Connected,
    Retry,
    ProcessExited,
    TimedOut,
}

pub fn pipe_wait_decision(connected: bool, process_alive: bool, elapsed: Duration) -> PipeWaitDecision {
    if connected {
        PipeWaitDecision::Connected
    } else if !process_alive {
        PipeWaitDecision::ProcessExited
    } else if elapsed >= PIPE_CONNECT_TIMEOUT {
        PipeWaitDecision::TimedOut
    } else {
        PipeWaitDecision::Retry
    }
}

#[derive(Clone, Default)]
pub struct GameSearchState(Arc<AtomicBool>);

impl GameSearchState {
    pub fn try_begin(&self) -> Option<GameSearchRun> {
        self.0
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| GameSearchRun(self.0.clone()))
    }
}

pub struct GameSearchRun(Arc<AtomicBool>);

impl Drop for GameSearchRun {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
```

- [ ] **Step 4: Run the focused tests and verify GREEN**

Run:

```powershell
cargo test --locked --package gbfr-logs game_search::tests
```

Expected: 6 tests pass.

- [ ] **Step 5: Commit the lifecycle decisions**

```powershell
git add -- src-tauri/src/game_search.rs src-tauri/src/main.rs
git commit -m "feat: define bounded game search lifecycle"
```

### Task 2: Bounded Backend Search, Pipe Wait, and Retry Command

**Files:**
- Modify: `src-tauri/src/main.rs`
- Test: `src-tauri/src/main.rs` test module

**Interfaces:**
- Consumes: all Task 1 exports.
- Produces: serialized `ConnectionState::NotFound` as `not-found`.
- Produces: Tauri command `retry_game_search(app: AppHandle, state: State<ConnectionStatus>) -> ConnectionState`.
- Produces: `spawn_game_search(app: AppHandle, delay: Duration) -> bool` as the only search-task entry point.

- [ ] **Step 1: Add failing state and retry-eligibility tests**

Append these tests to the existing `src-tauri/src/main.rs` test module and import `retry_allowed` and `ConnectionState` from `super`:

```rust
#[test]
fn not_found_connection_state_is_append_only_frontend_copy() {
    assert_eq!(
        serde_json::to_string(&ConnectionState::NotFound).unwrap(),
        "\"not-found\""
    );
}

#[test]
fn retry_is_allowed_only_after_search_exhaustion() {
    assert!(retry_allowed(ConnectionState::NotFound));
    assert!(!retry_allowed(ConnectionState::Searching));
    assert!(!retry_allowed(ConnectionState::Connected));
    assert!(!retry_allowed(ConnectionState::Disconnected));
    assert!(!retry_allowed(ConnectionState::Unsupported));
}
```

- [ ] **Step 2: Run the focused tests and verify RED**

Run:

```powershell
cargo test --locked --package gbfr-logs not_found_connection_state
cargo test --locked --package gbfr-logs retry_is_allowed
```

Expected: FAIL because `NotFound` and `retry_allowed` do not exist.

- [ ] **Step 3: Append the public state and register lifecycle state**

In `src-tauri/src/main.rs`:

```rust
use std::time::{Duration, Instant};

use dll_syringe::{
    process::{OwnedProcess, Process},
    Syringe,
};
use game_search::{
    pipe_wait_decision, GameSearchState, PipeWaitDecision, ProcessSearchBudget,
    ProcessSearchDecision, PIPE_CONNECT_INTERVAL, PROCESS_SEARCH_INTERVAL,
};
```

Append `NotFound` after `Unsupported` in `ConnectionState`. Add:

```rust
fn retry_allowed(state: ConnectionState) -> bool {
    state == ConnectionState::NotFound
}
```

Register `.manage(GameSearchState::default())` next to `ConnectionStatus`.

- [ ] **Step 4: Replace unbounded process discovery with the guarded runner**

Add this single task entry point:

```rust
fn spawn_game_search(app: AppHandle, delay: Duration) -> bool {
    let Some(run) = app.state::<GameSearchState>().inner().clone().try_begin() else {
        return false;
    };

    if delay.is_zero() {
        emit_connection_state(&app, ConnectionState::Searching);
    }

    tauri::async_runtime::spawn(async move {
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
            emit_connection_state(&app, ConnectionState::Searching);
        }
        check_and_perform_hook(app).await;
        drop(run);
    });
    true
}
```

Rewrite only the outer process-search loop in `check_and_perform_hook`:

```rust
async fn check_and_perform_hook(app: AppHandle) {
    let mut budget = ProcessSearchBudget::new();

    loop {
        let target = OwnedProcess::find_first_by_name("granblue_fantasy_relink.exe");
        match budget.record(target.is_some()) {
            ProcessSearchDecision::Found => {
                let target = target.expect("found decision requires a process");
                let pipe_target = match target.try_clone() {
                    Ok(process) => process,
                    Err(error) => {
                        warn!("Could not retain game process handle: {:?}", error);
                        emit_connection_state(&app, ConnectionState::Unsupported);
                        return;
                    }
                };

                // Keep the existing debug/release DLL choice and injection block unchanged here.
                // Replace the old call with the retained process handle:
                connect_and_run_parser(app, pipe_target);
                return;
            }
            ProcessSearchDecision::Retry => {
                tokio::time::sleep(PROCESS_SEARCH_INTERVAL).await;
            }
            ProcessSearchDecision::NotFound => {
                emit_connection_state(&app, ConnectionState::NotFound);
                return;
            }
        }
    }
}
```

The comment marks the existing contiguous DLL-selection and `syringe.inject` block that remains byte-for-byte equivalent; do not change its logging, debug-DLL precedence, or existing-pipe compatibility behavior.

- [ ] **Step 5: Bound the existing pipe-connect error branch**

Change `connect_and_run_parser` to accept `target: OwnedProcess`. Inside its spawned task, initialize `let pipe_started = Instant::now();` immediately before the existing outer pipe loop. Store each connection result, calculate the tested decision, and match it:

```rust
let connection = RecvPipeStream::connect_by_path(protocol::PIPE_NAME).await;
match pipe_wait_decision(connection.is_ok(), target.is_alive(), pipe_started.elapsed()) {
    PipeWaitDecision::Connected => {
        let stream = connection.expect("connected decision requires a pipe");
        // Keep the existing handshake, decoder, parser message match,
        // autosave interval, and connection-loss cleanup unchanged here.
    }
    PipeWaitDecision::Retry => tokio::time::sleep(PIPE_CONNECT_INTERVAL).await,
    PipeWaitDecision::ProcessExited => {
        state.on_connection_lost();
        update_equipment_connection(&app, false);
        emit_connection_state(&app, ConnectionState::Disconnected);
        let _ = app.emit_all("error-alert", "Game connection closed");
        tokio::time::sleep(PROCESS_SEARCH_INTERVAL).await;
        spawn_game_search(app.clone(), Duration::ZERO);
        return;
    }
    PipeWaitDecision::TimedOut => {
        update_equipment_connection(&app, false);
        emit_connection_state(&app, ConnectionState::Unsupported);
        return;
    }
}
```

The `Connected` branch contains the current contiguous `Ok(stream)` body without behavioral edits. This makes successful connection part of the same tested decision model without rewriting message dispatch.

At the existing normal pipe-close tail, replace recursive `tauri::async_runtime::spawn(check_and_perform_hook(app))` with:

```rust
tokio::time::sleep(PROCESS_SEARCH_INTERVAL).await;
spawn_game_search(app, Duration::ZERO);
```

Delete the old unconditional process-search recursion. Every search now enters through `spawn_game_search`.

- [ ] **Step 6: Add and register the retry command**

Add:

```rust
#[tauri::command]
fn retry_game_search(app: AppHandle, state: State<ConnectionStatus>) -> ConnectionState {
    let current = *state.0.lock().unwrap();
    if retry_allowed(current) && spawn_game_search(app.clone(), Duration::ZERO) {
        ConnectionState::Searching
    } else {
        *app.state::<ConnectionStatus>().0.lock().unwrap()
    }
}
```

Add `retry_game_search` immediately after `get_connection_state` in `tauri::generate_handler!`. Replace setup’s direct spawn with:

```rust
spawn_game_search(app.handle(), Duration::ZERO);
```

- [ ] **Step 7: Run backend regression tests and verify GREEN**

Run:

```powershell
cargo test --locked --package gbfr-logs
```

Expected: all `gbfr-logs` tests pass, including the 6 search-module tests and 2 new main tests.

- [ ] **Step 8: Commit backend orchestration**

```powershell
git add -- src-tauri/src/main.rs
git commit -m "feat: bound game discovery and expose retry"
```

### Task 3: Not-Found Header and Retry Action

**Files:**
- Modify: `src/types.ts`
- Modify: `src/pages/Logs.tsx`
- Modify: `src/pages/Logs.test.tsx`
- Modify: `src/pages/Settings.localization.test.ts`
- Modify: `src-tauri/lang/ko/ui.json`
- Modify: `src-tauri/lang/en/ui.json`

**Interfaces:**
- Consumes: backend `not-found` events and `retry_game_search` from Task 2.
- Produces: translated not-found status and one pending-safe retry button.

- [ ] **Step 1: Write failing frontend tests**

In `src/pages/Logs.test.tsx`, extend the hoisted mocks with `invoke: vi.fn()`, mock `@tauri-apps/api`, and add not-found translations to the translation mock. Reset `invoke` in `beforeEach` with explicit results so the real repeat-quest hook remains stable:

```tsx
mocks.invoke.mockImplementation(async (command: string) => {
  if (command === "get_repeat_quest_status") return { state: "off", reason: null };
  if (command === "retry_game_search") return "searching";
  throw new Error(`unexpected command: ${command}`);
});
```

Add:

```tsx
it("shows retry only when the game search is exhausted", () => {
  mocks.connectionState = "not-found";
  renderLayout();

  const header = screen.getByRole("banner");
  expect(within(header).getByText("게임을 찾지 못했습니다.")).toBeTruthy();
  expect(within(header).getByRole("button", { name: "게임 다시 찾기" })).toBeTruthy();
});

it.each(["searching", "connected", "disconnected", "unsupported"] as const)(
  "does not show retry while the state is %s",
  (state) => {
    mocks.connectionState = state;
    renderLayout();

    expect(screen.queryByRole("button", { name: "게임 다시 찾기" })).toBeNull();
  }
);

it("invokes one retry while the command is pending", async () => {
  const retry = deferred<void>();
  mocks.connectionState = "not-found";
  mocks.invoke.mockImplementation((command: string) =>
    command === "retry_game_search" ? retry.promise : Promise.resolve({ state: "off", reason: null })
  );
  renderLayout();
  const button = screen.getByRole("button", { name: "게임 다시 찾기" });

  fireEvent.click(button);
  fireEvent.click(button);

  expect(mocks.invoke).toHaveBeenCalledTimes(1);
  expect(mocks.invoke).toHaveBeenCalledWith("retry_game_search");
  expect((button as HTMLButtonElement).disabled).toBe(true);
  retry.resolve();
  await act(async () => retry.promise);
});
```

Use the same `deferred<T>()` promise helper shape as `useConnectionState.test.tsx`.

In `src/pages/Settings.localization.test.ts`, append `not-found` to both expected connection maps and add exact `game-search` maps:

```ts
const expectedEnglishGameSearch = {
  retry: "Retry",
  "retry-label": "Find the game again",
};

const expectedKoreanGameSearch = {
  retry: "재시도",
  "retry-label": "게임 다시 찾기",
};
```

Assert `english["game-search"]` and `korean["game-search"]` equal those objects.

- [ ] **Step 2: Run the frontend tests and verify RED**

Run:

```powershell
npm test -- --run src/pages/Logs.test.tsx src/pages/Settings.localization.test.ts
```

Expected: FAIL because `not-found` is not in the type/locales and no retry button exists.

- [ ] **Step 3: Append the frontend state and exact translations**

Change `src/types.ts` to:

```ts
export type ConnectionState = "searching" | "connected" | "disconnected" | "unsupported" | "not-found";
```

Add to Korean UI JSON:

```json
"connection": {
  "not-found": "게임을 찾지 못했습니다."
},
"game-search": {
  "retry": "재시도",
  "retry-label": "게임 다시 찾기"
}
```

Merge those keys into the existing objects rather than replacing existing translations. Add equivalent English values from Step 1.

- [ ] **Step 4: Implement the pending-safe header action**

Add `Button` and `invoke` imports plus local pending state in `Logs.tsx`:

```tsx
const [retryPending, setRetryPending] = useState(false);

const retryGameSearch = async () => {
  if (retryPending) return;
  setRetryPending(true);
  try {
    await invoke("retry_game_search");
  } finally {
    setRetryPending(false);
  }
};
```

Keep the existing left header group. Replace the single right-side status `Text` with:

```tsx
<Group gap="xs" wrap="nowrap">
  <Text size="sm" ta="right" truncate>
    {t(`ui.connection.${connectionState}`)}
  </Text>
  {connectionState === "not-found" && (
    <Button
      size="compact-xs"
      variant="subtle"
      aria-label={t("ui.game-search.retry-label")}
      disabled={retryPending}
      onClick={() => void retryGameSearch()}
    >
      {t("ui.game-search.retry")}
    </Button>
  )}
</Group>
```

Import `useState` alongside `useEffect`. Do not add CSS or alter sidebar/main scrolling.

- [ ] **Step 5: Run focused frontend regression tests and verify GREEN**

Run:

```powershell
npm test -- --run src/pages/Logs.test.tsx src/pages/Settings.localization.test.ts src/pages/useConnectionState.test.tsx src/pages/useCompactMeter.test.tsx src/pages/useRepeatQuest.test.tsx
```

Expected: all focused tests pass.

- [ ] **Step 6: Commit the frontend action**

```powershell
git add -- src/types.ts src/pages/Logs.tsx src/pages/Logs.test.tsx src/pages/Settings.localization.test.ts src-tauri/lang/ko/ui.json src-tauri/lang/en/ui.json
git commit -m "feat: add game search retry action"
```

### Task 4: Full Verification and Manual Retry Cycle

**Files:**
- Verify only; update source only if a new failing regression test demonstrates a defect.

**Interfaces:**
- Consumes: Tasks 1-3.
- Produces: automated and actual-window evidence for bounded search and retry.

- [ ] **Step 1: Run static checks**

```powershell
npm run format-check
npm run lint
npm run tsc
cargo fmt --all -- --check
```

Expected: every command exits 0.

- [ ] **Step 2: Run full automated regression and production build**

```powershell
npm test -- --run
npm run build
cargo test --workspace --all-targets --locked
```

Expected: all tests and the production frontend build pass. Existing non-fatal chunk-size and Rust dead-code warnings may remain.

- [ ] **Step 3: Verify the actual closed-game cycle**

With the game process closed, restart the development application so the new Rust backend is loaded. Confirm:

1. the header begins at `게임을 찾는 중입니다`;
2. after the tenth failed attempt it changes to `게임을 찾지 못했습니다. 재시도`;
3. no further process searches occur while left at `not-found`;
4. clicking `재시도` once returns the header to `게임을 찾는 중입니다`;
5. rapid additional clicks do not create another search run;
6. the second run returns to not-found after ten attempts;
7. settings and repeat quest do not duplicate the common not-found message;
8. the 800x600 header, sidebar, and main scroll regions remain readable.

- [ ] **Step 4: Optionally verify connection in a private/offline game session**

If the user has the game running during verification, confirm that a search run transitions to `게임에 연결되었습니다`. Record this only as connection-lifecycle evidence; do not claim full game 2.0.2 compatibility.

- [ ] **Step 5: Check final scope and working-tree preservation**

```powershell
git diff --check
git status --short
git log -6 --oneline
```

Expected: no whitespace errors; only the known unrelated user files and `logs.db` remain outside the feature commits.

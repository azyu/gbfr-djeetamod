# Bounded Game Search and Retry Design

## Goal

Replace indefinite game-process and hook-pipe polling with bounded attempts, an explicit not-found state, and a real retry action in the management header.

## Scope

- Search for `granblue_fantasy_relink.exe` at most ten times per search run.
- Stop searching after exhaustion and expose a `not-found` connection state.
- Let the user start a fresh bounded search from the management header.
- Prevent concurrent search runs and duplicate retry actions.
- Bound hook-pipe connection polling and leave that loop when the selected process exits.
- Preserve injection, handshake, parser, encounter, repeat-quest, and compact-meter semantics outside the connection lifecycle.
- Do not add automatic exponential backoff, settings, notifications, or persisted retry state.

## Public State and Header

Append `NotFound` to the Rust `ConnectionState` enum. Its serialized value is `not-found`. Append the same value to the frontend `ConnectionState` union.

The management header renders:

- Korean: `게임을 찾지 못했습니다.` followed by a compact `재시도` button.
- English: `The game could not be found.` followed by a compact `Retry` button.

The button appears only for `not-found`. Clicking it immediately changes the backend state to `searching`, hides the button, and begins a fresh search run. Existing labels remain unchanged for `searching`, `connected`, `disconnected`, and `unsupported`.

The retry button uses the translated accessible name `게임 다시 찾기` / `Find the game again`. No new icon, badge, animation, toast, or status color is added.

## Search Lifecycle

Define these fixed limits:

- process search attempts: 10;
- delay between failed process searches: 1 second;
- hook-pipe polling interval: 100 milliseconds;
- hook-pipe connection deadline: 10 seconds.

A process search run behaves as follows:

1. Acquire the single-flight search guard. If another search run owns it, return without spawning another task.
2. Emit `searching`.
3. Search immediately for the game process.
4. When absent, wait one second and try again, except after the tenth failed attempt.
5. After ten failures, emit `not-found`, release the guard, and stop.
6. When found, keep the owned process handle, perform the existing injection attempt, and proceed to bounded pipe connection.

The first attempt is immediate, so ten failed attempts take approximately nine seconds plus process-enumeration time. This is the precise meaning of the requested “about ten attempts.”

At application startup, start one search run. After a connected game or pipe closes, emit `disconnected`, wait one second, and automatically start one new bounded search run. If that run also exhausts its attempts, remain `not-found` until the user retries.

## Retry Command and Concurrency

Add a Tauri command named `retry_game_search`. It is valid only while the authoritative backend connection state is `not-found`.

The command:

1. atomically checks the current connection state and the single-flight guard;
2. if eligible, starts one search task and returns `searching`;
3. if a search is already running or the state is no longer `not-found`, performs no new work and returns the current state.

The search guard is process-local and non-persistent. It must release on normal exhaustion, process loss, pipe timeout, successful handoff to the parser, and task failure. Rapid button clicks can therefore create at most one search run.

## Bounded Pipe Connection

The found `OwnedProcess` handle is retained while waiting for the named pipe. The installed `dll-syringe` `Process` trait provides `is_alive()`, so no second name-based lookup or PID inference is required.

While connecting to the pipe:

- attempt a connection every 100 milliseconds;
- before each retry, check `target.is_alive()`;
- if the process exits, emit `disconnected`, release the current run, wait one second, and start a new bounded process search;
- if the process remains alive but no pipe connects within 10 seconds, emit `unsupported`, release the run, and stop;
- if the pipe connects, continue the existing handshake and parser flow.

An injection failure continues to try the existing pipe within the same 10-second deadline, preserving the current upgrade-compatibility behavior. A compatible existing pipe may still reach `connected`. A `HookStatus::Unsupported` message remains latched by the existing handshake invariant and is not changed by this design.

## Frontend Consumers

- `useConnectionState` accepts and publishes `not-found` like any other backend-observed state.
- `Logs` owns the retry command and a short pending flag to prevent duplicate UI actions before the backend event arrives.
- The compact meter remains visible only for `connected`; `not-found` therefore behaves like every other non-connected state.
- Repeat quest continues refreshing on every `connection-state` event. Its backend-observed unavailable state remains authoritative, and its duplicate `gameNotRunning` reason remains suppressed because the header owns the common condition.
- Settings remains free of connection-state presentation.

## Testability

Lifecycle decisions must be separated from Windows process and pipe I/O before changing behavior:

- a bounded-search counter decides `retry`, `found`, or `not-found` and proves the exact ten-attempt boundary;
- a single-flight guard proves overlap rejection and release on drop;
- a pipe-wait budget decides `retry`, `process-exited`, or `timed-out` from elapsed time and liveness;
- the Tauri-facing orchestration applies those tested decisions to real `OwnedProcess`, `RecvPipeStream`, state emission, and task spawning.

No test-only production API is added. Small internal decision types remain private to `src-tauri/src/main.rs` or a focused connection-lifecycle module if extraction materially improves readability.

## Verification

Automated tests must prove:

- attempts 1 through 9 request another process scan and attempt 10 produces `not-found`;
- finding the process on any attempt prevents `not-found`;
- only one search run can own the guard;
- retry is accepted only from `not-found` and starts one run;
- a dead process exits pipe waiting and schedules a fresh bounded search;
- a live process without a pipe reaches `unsupported` at 10 seconds;
- a pipe connection before the deadline continues the existing parser path;
- `not-found` serializes as `not-found` and is accepted by frontend types;
- the header shows the translated not-found text and retry button only for that state;
- clicking retry invokes `retry_game_search` once, hides or disables duplicate actions while pending, and returns to searching when observed;
- compact-meter and repeat-quest regression tests remain green.

Manual verification with the game closed must show ten attempts followed by `게임을 찾지 못했습니다. [재시도]`. Clicking `재시도` must return the header to `게임을 찾는 중입니다` and run one new ten-attempt cycle. A private/offline game session may then verify transition to `게임에 연결되었습니다`; this does not by itself establish full game 2.0.2 compatibility.

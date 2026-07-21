# Shared Game Connection Status Design

## Goal

Give the management window one authoritative game-connection indicator in its header. Remove duplicate connection messaging from individual pages and keep feature-specific errors local to the affected feature.

## Scope

- Add a reusable frontend connection-state hook backed by the existing Tauri `get_connection_state` command and `connection-state` event.
- Let the management layout subscribe once and render the current state at the right edge of the header.
- Remove the connection-state line from the settings page.
- Suppress the repeat-quest `gameNotRunning` reason because the header owns that application-wide condition.
- Preserve all repeat-quest reasons that describe feature-specific failures.
- Leave the compact meter's existing connection lifecycle unchanged because it runs in a separate WebView and uses the state to control meter visibility.
- Do not change process detection, injection, hook lifecycle, or backend connection-state variants.

## Header Layout

The management header has two horizontal groups separated by the available space:

- Left: the existing responsive menu controls followed by `Djeeta MOD`.
- Right: one localized connection-state label.

The label remains visible for every state:

| State | Korean label | Meaning |
| --- | --- | --- |
| `searching` | `게임을 찾는 중입니다` | The application is polling for the game process. |
| `connected` | `게임에 연결되었습니다` | The hook handshake has been accepted. |
| `disconnected` | `게임 실행 중이 아닙니다` | A previously connected game process or pipe has closed. |
| `unsupported` | `지원하지 않는 게임 버전입니다` | Injection, handshake, or version validation rejected the running game. |

Existing English localization receives equivalent labels. Text is right-aligned and must not overlap the left group at the supported 800x600 minimum management-window size. The implementation may use compact typography but must not introduce icons, badges, animation, or new status colors in this change.

## State Ownership and Data Flow

A new `useConnectionState` hook owns the management-window subscription:

1. Initialize to `searching` so the first render has a stable label.
2. Register the `connection-state` listener.
3. After listener registration, invoke `get_connection_state` to close the initial-event race.
4. Apply later event payloads to local React state.
5. Ignore late asynchronous results and unregister the listener on unmount.

`Logs` calls this hook once and passes no connection state through route components. The backend remains the source of truth; the frontend does not persist or infer connection state.

`useSettings` no longer subscribes to connection events or returns `connectionState`. `SettingsPage` removes its duplicate status text.

`useRepeatQuest` continues listening for connection changes because it must refresh backend-observed patch state. Its sidebar presentation hides only `gameNotRunning`; all other reasons remain visible below the switch.

## Error and Transition Behavior

- A failed initial `get_connection_state` call leaves the initial `searching` state rather than inventing a new public state.
- Rapid state transitions use the latest event received by the mounted management window.
- The existing latched `unsupported` backend invariant remains unchanged.
- Header state does not enable or disable controls directly. Existing feature hooks continue to determine their own availability.
- No toast is added for ordinary searching, connection, or disconnection transitions.

## Verification

Automated tests must prove:

- the shared hook reads the initial backend state, responds to events, and disposes safely;
- the header renders `Djeeta MOD` on the left and the localized state on the right;
- all four connection states map to their intended localization keys;
- settings no longer owns or renders connection state;
- repeat quest hides only `gameNotRunning` while preserving feature-specific reasons;
- existing navigation, repeat-quest, settings, and compact-meter behavior remains green.

Manual verification at the supported 800x600 management-window size must confirm that the title and right-aligned status remain readable for all four states and that the sidebar and main scrolling behavior is unchanged.

# Item acquisition warning notification design

## Goal

Add an opt-in Windows system notification after a battle when one or more
ordinary items increased and their resulting quantity is at least 900.

The feature must:

- be configurable from a sub-tab of the Item Analysis page;
- default to off and persist the user's choice across app restarts;
- inspect inventory once per completed battle instead of polling continuously;
- notify for every increase whose resulting quantity is at least 900, including
  increases such as 950 to 951;
- combine all qualifying items from one battle into one system notification;
- avoid false acquisition notifications when no valid baseline exists.

## User interface

The Item Analysis page will contain two sub-tabs:

1. `Inventory`: the existing description, refresh action, errors, and item table.
2. `Notification settings`: a persisted switch labeled
   `Notify me when an acquired item reaches 900 or more`.

Korean and English translation resources will contain the tab labels, switch
label and description, permission guidance, and notification copy. The Korean
switch copy will be:
`아이템 획득 시, 900개 이상일 경우 알림`.

The notification title will be `Djeeta MOD · 아이템 분석` in Korean and the
localized equivalent in English. The body will summarize the acquired items.
Windows owns presentation and notification-center history. The app will not
also show an in-app toast, force the window to the foreground, or change window
visibility.

When the user enables the switch, the app checks notification permission and
requests it when needed. The setting becomes enabled only after permission is
granted. If permission is denied or unavailable, the switch remains off and
the settings tab displays localized guidance.

## Architecture

### Persisted setting

A small Zustand persisted store will own only the boolean notification setting.
Its initial value is `false`. Runtime baseline data and pending timers will not
be persisted because quantities can change while the app is not running.

### Windows notification capability

Enable only Tauri's required notification feature and allowlist entry. The
frontend controller will use the Tauri notification permission and send APIs.
Permission is checked again when an enabled setting is restored at startup. If
permission is no longer available, the controller disables the persisted
setting and exposes the localized guidance state.

### Battle boundary

When the backend accepts the supported hook's `OnBattleEnd` message, it will
emit an app-wide `battle-ended` event in addition to the existing encounter
lifecycle behavior. This event is a notification trigger only; it must not
change encounter clearing, saving, or reward-boundary ordering.

The hook fires immediately before reward/result setup. The frontend therefore
debounces battle events and waits five seconds before taking one inventory
snapshot. A later battle event replaces an already pending timer so at most one
scan is scheduled.

### Full inventory snapshot

The existing Item Analysis response contains only items at or above the warning
threshold. That is sufficient for the table but cannot calculate an accurate
increase for a transition such as 899 to 900.

Add a dedicated read-only Tauri command that reuses the existing process,
version, region, stability, duplicate-record, and overlap protections but
returns all decoded ordinary-item quantities from the stable snapshot. The
existing `fetch_item_analysis` contract and table behavior remain unchanged.
The two commands share the same running guard, so they cannot scan the large
inventory region concurrently.

### Global notification controller

A hook mounted in the management layout will own:

- the most recent successful full inventory snapshot;
- whether that snapshot is a valid comparison baseline;
- the pending post-battle timer;
- the app-wide `battle-ended` listener.

When the setting becomes enabled, or when the app starts with the persisted
setting enabled, the controller immediately requests a full snapshot. A
successful result becomes the baseline without producing a notification.

Five seconds after a battle event, the controller requests another full
snapshot. If a valid baseline exists, a pure comparison function selects items
where:

```text
current quantity > previous quantity
and current quantity >= 900
```

For each selected item it retains the exact increase. After every successful
scan, the complete current snapshot replaces the baseline, including decreases
and unchanged values.

If several items qualify, one Windows notification summarizes them. The first
item includes its translated name, final quantity, and increase, followed by
the remaining count. A single qualifying item omits the remaining count. For
example:

```text
궁극의 증표 918 (+3) 외 2개
```

Turning the setting off cancels any pending timer and clears the runtime
baseline.

## Failure handling

- Denied or unavailable notification permission keeps the setting off and
  exposes localized guidance in the Notification settings tab.
- A failed initial scan leaves the controller without a valid baseline.
- A failed post-battle scan does not change the last valid baseline and does
  not show an error notification.
- The first successful scan after the controller has no baseline establishes
  a new baseline without showing an acquisition notification.
- An `ALREADY_RUNNING` result caused by a simultaneous manual refresh is
  treated like any other skipped scan; it does not create a false notification.
- Listener and timer cleanup occurs on unmount and when the setting is disabled.
- Unknown item-name records continue to use the existing item-ID fallback.

Inventory-read failures stay silent because the Item Analysis page already
exposes manual scan errors, while a background notification feature should not
create unrelated error noise.

## Testing

Development will follow test-first changes.

Frontend tests will prove:

- the persisted setting defaults to off and survives reload;
- the two sub-tabs and Korean/English copy are complete;
- enabling succeeds only with notification permission and denied permission
  leaves the switch off with guidance;
- a restored enabled setting is disabled if permission is no longer available;
- an initial successful snapshot establishes a baseline without a notification;
- 899 to 900, 900 to 901, and 950 to 951 qualify;
- unchanged values, decreases, and disabled settings do not notify;
- multiple qualifying items produce one translated Windows notification;
- battle events schedule a five-second delayed scan and repeated events are
  debounced;
- failed scans do not replace a valid baseline or create false notifications;
- disabling cancels a pending scan and clears the baseline.

Rust tests will prove:

- the full-snapshot response retains all ordinary items, including quantities
  below 900;
- the existing Item Analysis response still exposes only warning items;
- the battle-end message emits the new event without changing the parser's
  existing battle-end behavior;
- concurrent manual and notification scans remain rejected by the shared guard.

Security configuration tests will prove that only the required Tauri
notification capability was added. Frontend notification tests will mock
permission and send APIs so automated tests never create a real OS
notification.

After focused tests, run the project's required frontend formatting, lint,
TypeScript, test, and build commands, plus the release hook build and full Rust
workspace tests.

## Out of scope

- continuous or interval-based inventory polling;
- in-app toast duplication;
- changing the 900 threshold in the UI;
- separate notifications for every item;
- app-managed notification history beyond Windows Notification Center;
- changing game memory or the reward hook's established ordering;
- claiming game compatibility from automated tests alone.

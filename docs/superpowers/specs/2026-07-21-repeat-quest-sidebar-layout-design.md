# Repeat Quest Sidebar Layout Design

## Goal

Keep `무한 퀘스트 반복` visible in the management window regardless of the current page or window height, keep `설정` anchored at the bottom of the sidebar, and make oversized page content vertically scrollable.

## Root Cause

The shared `html`, `body`, and `#root` elements use `overflow: hidden`, while the management `AppShell.Main` does not establish its own bounded vertical scroll area. Settings content therefore extends beyond the visible window and is clipped. The repeat-quest switch was appended at the bottom of that content, making it unreachable at the default 800×600 management-window size.

## Layout

- Remove the repeat-quest fieldset from `SettingsPage`; the feature must have one UI owner.
- Add a dedicated repeat-quest control directly below the existing `데미지 미터` navigation row.
- Keep the normal navigation items in a growing, vertically scrollable sidebar section.
- Keep the `설정` navigation item in a separate non-growing section at the bottom of the sidebar.
- Give `AppShell.Main` a viewport-bounded vertical scroll area so settings, logs, and equipment pages remain reachable in short windows.
- Continue using the existing global scrollbar styling; do not add a second visual scrollbar implementation.

## State and Data Flow

The sidebar owns `useRepeatQuest`. The hook remains non-persistent and continues to use only:

- `get_repeat_quest_status` for the initial observed state;
- `set_repeat_quest_enabled` for user changes;
- the existing `connection-state` event to refresh status when the game connects, disconnects, or becomes unsupported.

The switch is checked only when the backend reports `on`. It is disabled while a command is pending or when the backend reports `unavailable`. No ON state, process address, or process identifier is stored in frontend persistence.

## Error Presentation

When the backend returns a reason, show the existing translated reason text directly below the sidebar control in compact text. A failed request preserves the last observed ON/OFF state and reports the existing `internal` reason. Moving the control does not change backend patching, permissions, startup restoration, or exit restoration.

## Verification

Automated tests must prove:

- the repeat-quest switch renders immediately below the damage-meter row and no longer appears in `SettingsPage`;
- the switch still reflects backend state, locks while pending, and sends `{ enabled }`;
- connection-state events trigger a status refresh;
- the settings navigation remains outside the growing scroll section;
- the management main region has bounded vertical overflow;
- existing navigation, localization, and repeat-quest tests remain green.

Manual verification uses the default 800×600 window and a shorter resized window. In both cases the repeat-quest control and bottom settings item must remain reachable, while long settings content scrolls vertically.

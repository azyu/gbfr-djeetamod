# Close Button Action Setting Design

**Date:** 2026-07-23

## Goal

Let users choose what the management window's top-right close button does while
preserving the current minimize-to-tray behavior as the default.

## User Experience

Add a `General Settings` fieldset above `Meter Settings`. Move the existing
language selector into this fieldset and add one controlled select labeled
`Close button action`.

The select has two explicit choices:

- `Minimize to tray` (default): hide the management window while Djeeta MOD
  continues running in the system tray.
- `Quit application`: save window state and terminate Djeeta MOD.

Supporting text explains that the setting applies to the management window's X
button. The tray menu's existing `Quit` item always terminates the application
regardless of this setting.

A select is preferred over a checkbox because both outcomes remain explicit,
and over radio buttons because it uses less vertical space in the existing
settings page.

## State and Data Flow

Add a string-valued close action to the existing persisted settings store. Its
default is `minimize-to-tray`, so existing users whose stored state lacks the new
field keep the current behavior without a migration.

At application startup and whenever the setting changes, the frontend invokes a
small Tauri command that updates an application-wide Rust state value. The Rust
value also defaults to minimize-to-tray, which keeps the close behavior safe and
consistent before the frontend has finished mounting.

The existing `CloseRequested` handler reads that state for the `logs`
management window:

- minimize-to-tray hides the window and prevents the close;
- quit saves all window state and exits the application.

Close requests from other windows retain their current hide-and-prevent
behavior. This keeps the option scoped to the visible Windows close button the
user is configuring and avoids changing unrelated window lifecycle behavior.

## Components

- `useMeterSettingsStore` persists the new close action alongside existing
  settings. The store keeps its current name and storage key to avoid an
  unrelated migration or refactor.
- `useSettings` exposes the current value and a change handler. The handler
  updates persisted state and immediately synchronizes the Rust state.
- `SettingsPage` renders the new general-settings fieldset, select, and
  explanatory copy using localization keys.
- The application startup path synchronizes the persisted selection to Rust
  even when the settings page is never opened.
- The Tauri backend owns the authoritative close decision once a native
  `CloseRequested` event occurs.

## Error Handling

If startup synchronization fails, Rust retains the safe default and the X
button minimizes to the tray. A failed setting-change invocation is logged, but
the persisted choice remains and is retried on the next application startup.
No confirmation dialog is added because the user has explicitly selected the
close behavior and Djeeta MOD has no unsaved editor state.

## Testing

- Add a focused store or settings-hook regression test proving the default is
  minimize-to-tray and both select values persist.
- Test that startup and setting changes invoke the backend with the matching
  close-to-tray value.
- Unit-test the Rust close-decision helper for the management window's two
  branches and for an unrelated window.
- Extend localization coverage for the general-settings label, select label,
  description, and both choices in Korean and English.
- Run the focused regression tests first, then the required frontend format,
  lint, type-check, complete test, and build commands. Because the lifecycle
  handler changes in Rust, also run the required release hook build and full
  locked Rust test suite.

## Out of Scope

- Changing the tray menu's `Quit` command.
- Adding a close confirmation dialog or a one-time onboarding prompt.
- Changing the compact meter window's custom title bar.
- Refactoring or renaming the existing settings store.

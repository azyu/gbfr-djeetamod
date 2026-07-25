# Conflux Minimal Autodrive Implementation Plan

**Goal:** Use the game's own Conflux unattended progression while Djeeta MOD
shortens its inactivity timer, selects the final reward, and re-enters the
currently focused depth.

**Architecture:** Keep timer patching, reward selection, and re-entry actions
inside the version-pinned injected hook. Expose one fail-closed control channel
to the Tauri page. Promote a native mutation only after an independent
observation proves a unique boundary and reversible behavior.

## Constraints

- Target only executable SHA-256
  `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F`.
- Test only in an offline/private session.
- Do not import, load, redistribute, decompile, or copy implementation details
  from downloaded mods.
- Do not synthesize keyboard, mouse, or player movement.
- Do not touch `logs.db`.
- Do not stage or commit without explicit approval.

## Task 1: Independently identify the inactivity timer

- [ ] Add a read-only timer candidate sampling mode to
      `src-tauri/examples/probe_conflux_ui.rs`.
- [ ] Add focused tests for candidate filtering, stable original values, and
      float/double interpretation.
- [ ] Observe the same Conflux screen with the game option OFF and ON and
      identify the countdown/duration boundary.
- [ ] Derive and document a unique pinned-executable signature without using
      the downloaded DLL.
- [ ] Verify that the candidate does not control fade or transitions.

## Task 2: Implement the minimal pure policy

- [ ] Replace obsolete route/Power RED tests with timer lifecycle and reward
      fallback tests.
- [ ] Reduce protocol stages to timer armed, reward selection, result return,
      Tredame gate, party formation, and depth confirmation.
- [ ] Implement OFF-by-default, enable/apply, disable/restore,
      disconnect/restore, and fail-closed transitions.
- [ ] Run focused protocol and hook tests.

## Task 3: Implement validated native boundaries

- [ ] Implement the reversible timer patch with original-value ownership.
- [ ] Implement only the verified reward and re-entry callbacks.
- [ ] Reject non-unique signatures, unexpected original values, invalid active
      states, duplicate actions, and transition timeouts.
- [ ] Restore timer values before every terminal OFF/unavailable path.

## Task 4: Add the Tauri control client and page

- [ ] Add the `극돈공소` sidebar route and page.
- [ ] Add the process-local `자동 실행` switch and persisted floor-five reward
      preference.
- [ ] Show authoritative stage/reason state and the requirement that the game
      unattended option be ON.
- [ ] Add frontend and backend regression tests.

## Task 5: Verify

- [ ] Run focused tests, formatting, lint, TypeScript, full tests, frontend
      build, release hook build, and Rust workspace tests.
- [ ] Start a fresh offline/private floor-three run with the game option ON.
- [ ] Confirm about-two-second game-owned progression, original-delay restore
      on OFF, unchanged transitions, reward fallback, Tredame return, and
      re-entry with no manual input after enable.
- [ ] Record remaining unsupported boundaries as TODO rather than guessing.

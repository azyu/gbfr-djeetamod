# Conflux Minimal Automation Implementation Plan

**Goal:** Use the game's own unattended Conflux progression while Djeeta MOD
shortens the wait. Keep final-reward selection and re-entry as follow-up work.

## Constraints

- Target only executable SHA-256
  `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F`.
- Test only in an offline/private session.
- Do not copy implementation details from downloaded mods.
- Do not synthesize input or player movement.
- Do not touch `logs.db`.
- Do not stage or commit without explicit approval.

## Timer Reduction

- [x] Independently identify the Conflux timer manager, mode, original
      configuration, notice threshold, initial duration, and current countdown.
- [x] Add a read-only timer-manager probe and focused tests.
- [x] Implement exact-value classification, 1/2-second application, active
      countdown clamping, readback verification, rollback, and restoration in
      the Tauri backend.
- [x] Verify a reversible live write round trip in the offline/private session.
- [x] Keep the injected hook and shared protocol unchanged.

## UI and Lifecycle

- [x] Add the `극돈공소` sidebar route and page.
- [x] Add an authoritative OFF/ON/unavailable switch and explain the required
      game option.
- [x] Restore on startup, OFF, normal exit, and update preparation.
- [x] Show reward selection and re-entry as TODO.
- [x] Add focused frontend, backend, and write-isolation regression tests.

## Remaining Verification

- [ ] Run format, lint, TypeScript, full frontend tests/build, focused Rust
      tests, release hook build, and Rust workspace tests.
- [ ] In a fresh offline/private floor-three run, enable both the game option
      and Djeeta MOD switch.
- [ ] Observe about-two-second game-owned route/Power/OK progression.
- [ ] Disable the switch and verify original timer values are restored.
- [ ] Confirm fade and screen transitions are unchanged.

## Follow-up TODO

- [ ] Map and validate final-reward rows, IDs, selection, and fallback.
- [ ] Map and validate TOTAL RESULTS dismissal and return destination.
- [ ] Map and validate Tredame gate, current-party confirmation, and depth
      re-entry.

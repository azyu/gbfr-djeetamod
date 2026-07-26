# Conflux Minimal Automation Design

**Date:** 2026-07-25
**Revised:** 2026-07-26

## Decision

Granblue Fantasy: Relink's `극돈공소 무조작 시 설정` owns all in-run route,
Power, monk, and single-OK progression. Djeeta MOD owns only:

1. shortening the game's unattended-selection delay while the user enables it;
2. later, selecting the configured final reward;
3. later, returning to Tredame Palace and re-entering Conflux.

This implementation delivers item 1. Items 2 and 3 remain explicit TODOs.

## Current Product Behavior

The `극돈공소` page contains an OFF-by-default `자동 실행` switch. Enabling it
changes the independently identified Conflux timer configuration from the
verified 2.0.2 values to:

- auto-progress delay: 2 seconds;
- notice delay: 1 second.

Fade and screen-transition timings are untouched. Disabling the switch restores
the verified original timer configuration. Startup, normal application exit,
and update installation also attempt restoration. The switch is not persisted.

The game option `극돈공소 무조작 시 설정` must be ON. Djeeta MOD does not
toggle that option and does not issue route, Power, monk, dialog, keyboard,
mouse, or player-movement actions.

## Implementation Boundary

The timer is modified by the Tauri backend through a narrow external-process
data write. It is not implemented in the injected hook and adds no protocol
variant. Before every mutation the backend:

- finds the exact game process;
- verifies executable SHA-256
  `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F`;
- resolves the independently identified timer-manager pointer;
- verifies the complete original/shortened configuration;
- requires Endless mode before enabling;
- verifies the write and rolls back on failure.

The downloaded `GBFR Fast Conflux` archive is behavior reference only. Djeeta
MOD does not load, redistribute, decompile, or copy its code, addresses,
signatures, or constants.

## Safety Boundaries

- Timer writes target writable manager data only; code pages, fade timing, and
  transition timing are not modified.
- Unknown or mixed timer values fail closed.
- OFF and update preparation restore the original values even when the current
  app session did not perform the earlier write.
- Normal exit restores only when the app may have enabled the timer.
- A process crash cannot run exit cleanup. A later Djeeta MOD startup or game
  restart restores the safe baseline.
- Automated tests and builds do not establish game compatibility.

## Observable Success Criteria

1. With the game option ON and Djeeta MOD `자동 실행` ON, game-owned selections
   and single-OK screens advance in about two seconds.
2. Turning `자동 실행` OFF restores the original 60/30-second configuration
   without restarting the game.
3. Fade and screen transitions retain their original timing.
4. No Djeeta MOD route, Power, monk, or generic dialog action is required.

## Follow-up TODOs

- final-reward catalog and configured selection with first-selectable fallback;
- TOTAL RESULTS acknowledgement and Tredame return;
- gate interaction, current-party confirmation, and depth re-entry;
- floor-three full-cycle validation after those boundaries are implemented.

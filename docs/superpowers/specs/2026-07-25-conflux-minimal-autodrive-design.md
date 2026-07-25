# Conflux Minimal Autodrive Design

**Date:** 2026-07-25

## Decision

Djeeta MOD will leave all in-run choices and acknowledgements to Granblue
Fantasy: Relink's `극돈공소 무조작 시 설정`. The mod owns only three gaps:

1. reduce the Conflux inactivity auto-progress delay from 60 seconds to 2
   seconds while automatic execution is enabled;
2. select the configured final reward;
3. return to Tredame Palace and re-enter the currently selected depth.

The earlier route, Power, monk, and dialog-specific automation design remains a
follow-up reference only. It is not part of this implementation.

## Product Behavior

The existing `극돈공소` page keeps:

- an `자동 실행` switch;
- a non-clearable floor-five reward dropdown.

Automatic execution starts OFF for every game process. Enabling it applies the
2-second inactivity delay and 1-second notice delay, then arms final-reward and
re-entry handling. Disabling it, losing the control connection, or failing a
version/capability check restores the original game timer values immediately.
The timer setting is not persisted.

The game option `극돈공소 무조작 시 설정` must be ON. The mod does not toggle
that option and does not choose routes, Power, monk purchases, or generic OK
dialogs.

On floor five, the reward handler selects the first selectable row matching the
persisted internal reward ID. If it is absent, it selects the first selectable
row. Floor three is validation-only and always selects the first selectable
row. Other floor reward catalogs remain TODO.

After the result flow returns to Tredame Palace, the re-entry handler activates
the Conflux gate, keeps the current party, and confirms the depth already
focused by the game. It does not synthesize keyboard, mouse, or player movement.

## Independent Implementation Boundary

The downloaded `GBFR Fast Conflux` archive is used only as behavior
documentation. Its public configuration states:

- auto-progress delay: 2 seconds;
- notice delay: 1 second;
- fade and screen-transition timings remain unchanged;
- the feature starts OFF.

Djeeta MOD will not load, redistribute, decompile, or copy code, signatures,
addresses, or constants from the downloaded DLL. Timer storage and mutation
boundaries must be found independently in the pinned 2.0.2 executable and
validated in an offline/private session.

## Safety and Validation

- Pin the executable SHA-256 before enabling any patch.
- Require unique signatures and verified original values.
- Save original timer values before the first mutation and restore them on
  every OFF/error/disconnect path.
- Apply timer changes only from the injected hook; the desktop app must not
  write game memory.
- Do not patch fade or transition timing.
- Do not invoke a reward or re-entry callback until its active-state,
  argument, thread, and successor boundaries are independently verified.
- Automated tests and builds do not establish game compatibility.

## Observable Success Criteria

1. With the game option ON and Djeeta MOD automation ON, ordinary Conflux
   selections and single-OK screens advance in about two seconds without mod
   route/Power/dialog actions.
2. Turning automation OFF restores the normal delay without restarting the
   game.
3. Fade and screen transitions retain their original timing.
4. The configured floor-five reward is selected, with documented fallback.
5. A complete floor-three validation cycle returns to Tredame and re-enters
   the depth focused by the game without manual input after enabling.

## Follow-up TODOs

- per-floor reward catalogs and preferences outside floor five;
- explicit route or Power policies;
- monk CP-spending policy;
- allowlisted dialog-specific acceleration;
- explicit re-entry depth selection if the game does not preserve focus.

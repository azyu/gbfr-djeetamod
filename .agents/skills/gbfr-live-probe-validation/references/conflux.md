# Conflux Live Evidence

## Contract

- Require the pinned executable hash from
  `docs/testing/game-2.0.2-conflux-auto-reentry-probe.md`.
- Require the user to confirm an offline or private session.
- Use `DJEETA_CONFLUX_UI_PROBE=1` for bounded UI-object capture.
- Use `DJEETA_CONFLUX_TIMER_PROBE=1` only on an unattended-choice screen.
- Run `cargo run --locked --package gbfr-logs --example probe_conflux_ui`.
- Remove the task-specific environment variable after the process exits.
- Do not invoke game callbacks, synthesize input, or request write access.

## Stable Capture

1. Keep the requested game screen open.
2. Run only the selected read-only probe.
3. Record the process hash, readable-region summary, fixed target counts, and
   bounded fingerprints.
4. If visual labels or row ordering matter, use Computer Use to capture the
   unique returned game window without clicking or typing.
5. Save evidence under `docs/testing/evidence/` as
   `conflux-<floor-or-stage>-<screen>-YYYY-MM-DD.<ext>`.
6. Inspect the image for unnecessary personal data before referencing it.
7. Update only the matching evidence-table row.

## Evidence Shape

Record:

- floor or fixed stage;
- visible row or choice count;
- relevant controller families and counts;
- positive or hidden/stale negative;
- `PASS`, `MISMATCH`, or `DEFERRED` with one reason;
- screenshot link when visual names or order are evidence;
- remaining field, callback, or successor TODO.

Do not record:

- raw addresses or memory dumps;
- full process output when a bounded summary suffices;
- reward IDs before the catalog contract permits them;
- PID in committed files;
- claims that a structural `PASS` authorizes game actions.

## Promotion Boundary

A screenshot proves visible labels and order. Object counts prove allocation,
not active state. A bounded fingerprint proves a state difference, not semantic
ownership. Promote an action only after the evidence document's positive,
negative, ABI, unique-signature, and successor gates all pass.

## Completion

Report the game hash, screen captured, restart count, read-only status,
`PASS`/`MISMATCH`, evidence files changed, environment cleanup, and remaining
manual checks.

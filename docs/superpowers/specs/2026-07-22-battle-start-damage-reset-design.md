# Battle-Start Damage Reset Design

**Date:** 2026-07-22

## Goal

Clear every damage-derived value as soon as quest entry or Play Again loading
begins, before the first accepted hit of the next battle. The first accepted hit
continues to define the encounter start time.

## Boundary and Compatibility

The reset must originate from a Granblue Fantasy: Relink 2.0.2 loading boundary,
not from player-identity refreshes or inactivity. The existing auxiliary area
and quest hooks are disabled because their signatures and memory layouts are not
verified for 2.0.2; they must not be re-enabled without live validation in an
offline or private session.

Live validation must establish that the selected boundary fires once for normal
quest entry and once for Play Again loading, and does not fire during battle,
fall recovery, boss mechanics, or result presentation. Until that evidence is
recorded, the implementation remains compatibility-unverified.

## Wire Protocol

Append `OnBattleStart` to the end of the current bincode `Message` enum. Existing
variants retain their indices. The hook sends the new message immediately before
the validated loading operation begins. Legacy decoding remains unchanged.

## Parser Behavior

`Parser::on_battle_start_event` performs a non-persisting reset:

- clear total damage, DPS, stun totals, party damage/skill breakdowns, targets,
  timestamps, and the raw encounter event log;
- set both parser and derived status to `Waiting`;
- emit the cleared party and encounter state immediately so both meter views
  become empty before the first hit;
- preserve no completed encounter as part of this reset; persistence remains the
  responsibility of the verified reward boundary;
- remain idempotent if the loading boundary produces a duplicate notification.

Battle-scoped player identity caches are reset at the same boundary so reused
actor allocations cannot inherit the previous party mapping. New identity data
may then populate during loading. The first accepted damage event uses the
existing path to start a fresh `InProgress` encounter and is included in its
totals.

## Alternatives Rejected

- Player-identity refresh is already verified, but it can occur for party/lobby
  changes and is not an unambiguous battle boundary. Using it could erase an
  active encounter.
- The old `OnAreaEnter` and `OnLoadQuest` hooks are unverified on 2.0.2 and cannot
  be enabled safely based only on historical signatures.
- Clearing on every first-looking hit cannot distinguish a new battle while the
  parser still considers the prior battle active and would weaken the invariant
  that inactivity never splits a live encounter.

## Testing and Evidence

- Prove the new protocol variant is appended after every existing variant.
- Reproduce stale derived and raw damage state, invoke the start handler, and
  assert that every damage-derived collection and total is empty without saving.
- Invoke the start handler twice and assert the second reset is harmless.
- Apply the first accepted hit after reset and assert that only that hit appears
  in the new encounter.
- Unit-test hook notification ordering around the selected loading operation.
- Run the focused parser, hook, and protocol tests, then the required frontend
  and full Rust verification suites.
- Record live offline/private evidence for normal entry, Play Again, and the
  listed negative cases before claiming game 2.0.2 compatibility.

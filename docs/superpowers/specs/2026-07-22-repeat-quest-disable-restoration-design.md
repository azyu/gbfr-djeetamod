# Repeat Quest Disable Restoration Design

**Date:** 2026-07-22

## Problem

The repeat-quest backend discovers both patch sites with signatures that include
the original target instructions. Enabling changes those instructions, so a
later disable operation cannot rediscover either site and never reaches the
existing restoration writes.

## Design

Signature discovery continues to verify the pinned executable and the complete
stable instruction context, but excludes each three-byte patch target from the
signature comparison. After locating the unique surrounding context, the
existing byte classifier still requires each target to be exactly original or
patched; unknown bytes remain unavailable and are never overwritten.

Disabling can therefore rediscover the same sites after enabling changed the
target instructions. The same discovery also works during startup recovery
after an abnormal Djeeta MOD termination, where no in-memory patch address can
survive. Restoration continues to write the known original instructions only
when the observed target is the known patched value, followed by read-back.

## Alternatives Rejected

- Retaining only the successful enable transaction fixes explicit OFF in the
  same app process, but cannot support the existing startup recovery contract
  after an abnormal Djeeta MOD termination.
- Computing addresses from fixed executable offsets avoids the immediate bug
  but loses the current live-module stable-context validation.
- Restoring game data values is not required to fix the observed failure because
  the current code never reaches instruction restoration at all. Runtime state
  can be investigated separately only if functional repetition remains after
  the code restoration succeeds.

## Testing

Add a regression test that discovers sites in an original text fixture, changes
both target instructions to their patched values, rediscovers the same sites,
and restores the originals. Retain the existing unknown-byte refusal coverage.
Run the focused repeat-quest tests followed by the repository's required
frontend and Rust verification commands.

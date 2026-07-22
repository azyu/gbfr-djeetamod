---
name: gbfr-live-probe-validation
description: Use when validating Djeeta MOD debug probes against Granblue Fantasy Relink in a live offline or private game session, including external equipment reader, inventory, roster, controlled-change, restart, MATCH/MISMATCH, or smoke-test evidence work.
---

# Validate GBFR Live Probes

## Overview

Treat the game UI or verified hook snapshot as the control, accept only observed evidence, and keep probe success separate from full game compatibility.

## Select the Contract

| Probe | Gate | Evidence document |
|---|---|---|
| External equipment reader or roster | Debug build and `DJEETA_EXTERNAL_READER_PROBE=1` | `docs/testing/game-2.0.2-equipment-layout.md` |
| Inventory scanner | Debug build and `DJEETA_INVENTORY_PROBE=1` | `docs/testing/game-2.0.2-inventory-probe.md` |
| Product behavior | Packaged build | `docs/testing/game-2.0.2-smoke-test.md` |

Read the selected evidence document and the relevant `docs/superpowers/specs` or `docs/research` file before proposing steps. Use the document's current acceptance criteria; do not silently strengthen or weaken them.

## Workflow

1. Confirm the target game build and executable hash from the selected evidence document. Require an offline or private session.
2. Confirm the probe is debug-only, opt-in, and read-only. Do not enable it in a release build or request memory-write rights.
3. Start only the required debug app. Do not launch, stop, or control the game without the user's explicit instruction.
4. Capture a stable baseline. Record PID, executable hash, bounded summary counts, status, and digest only when the contract calls for them.
5. Perform the contract's controlled change and restoration. Transitional `UnstableRead` or missing-hook states may be deferred; a stable `MISMATCH`, wrong hash, invalid signature count, or boundary violation is a failure and stops promotion.
6. Perform only the required restart count. Require new-process rediscovery and fresh hook-session comparison; do not repeat controlled changes on every restart unless the evidence contract requires it.
7. Verify opt-out or release rejection when required. End the debug process and remove the task-specific environment variable.
8. Update only rows personally observed during this session. Leave unobserved boxes unchecked and label failed gates accurately.

## Evidence Boundaries

- Never read, modify, stage, or commit `logs.db`.
- Do not record raw memory dumps, full inventory contents, personal player data, or unnecessary absolute addresses.
- Prefer counts, character keys, short digests, executable hashes, PID, and PASS/FAIL rationale.
- Probe failure must not change the damage-meter connection state or establish `HookStatus::Unsupported` unless the product contract explicitly requires it.
- Automated tests, packaging, or one probe PASS do not establish Granblue Fantasy Relink 2.0.2 compatibility.

## Completion Report

Report the tested probe, game build/hash, observed cases, restart count, PASS/FAIL gate, evidence document changed, cleanup performed, and remaining manual checks. If evidence was committed, stage only the intended documentation and code changes.

## Common Mistakes

- Treating `ROSTER PROBE CANDIDATE` as proof of unlocked characters.
- Treating transitional `DEFERRED` output as either PASS or permanent failure.
- Reusing a prior PID or stale hook snapshot as current evidence.
- Marking an entire smoke-test section complete from a narrower probe result.

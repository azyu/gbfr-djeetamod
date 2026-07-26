---
name: gbfr-reference-assisted-native-analysis
description: Use when Djeeta MOD native-boundary analysis is blocked and a user-supplied managed GBFR mod, PDB, IL, masked signature, RVA, vtable, callback, or field offset may provide hypotheses that must be independently relocated to a pinned game build.
---

# GBFR Reference-Assisted Native Analysis

## Overview

Treat reference mods as untrusted hints, never implementation authority. Promote
only evidence independently recovered from the pinned game executable and a
live offline/private positive and negative.

## Analysis Ladder

1. Read the applicable `AGENTS.md`, research ledger, live evidence contract,
   and approved design.
2. Hash the target executable and reference archive. Stop on any mismatch.
3. Perform one bounded independent pass for each required boundary.
4. Use the reference only for items that remain unresolved after that pass.
5. Extract only to a fresh directory under the system temporary root.
6. Do not execute, inject, load, redistribute, or commit reference binaries,
   PDBs, reconstructed sources, dependencies, or raw IL.
7. Inspect only relevant symbols, callers, fields, and static initializers.
8. Convert every finding into a named hypothesis.
9. Relocate masked signatures in executable PE sections and require exactly one
   match.
10. Corroborate the ABI, exact object type, bounded fields, active state,
    accept/cancel distinction, positive, hidden/stale negative, and successor.
11. Record only bounded findings in the repository.
12. Verify the resolved temporary path is a child of the temporary root, then
    delete that exact directory.

Read [`references/commands.md`](references/commands.md) for the command sequence
and evidence record.

## Deterministic Tools

- Run `scripts/extract_ildasm_signatures.py --help` to convert nullable-byte
  arrays in an `ildasm` static constructor into masked signatures.
- Run `scripts/scan_pe_signatures.py --help` to scan executable PE sections.
  Use `--require-unique` for a promotion gate.
- Keep script output in the task-specific temporary directory until its bounded
  summary is accepted into a research document.

## Promotion Result

Use exactly one result:

- `STATIC PASS`: independently relocated and fully corroborated.
- `CANDIDATE`: concrete evidence exists but a required gate is missing.
- `MISMATCH`: observed evidence contradicts the hypothesis.
- `REJECTED`: zero/multiple matches, wrong build, unsafe ABI, or no independent
  corroboration.

Reference agreement alone is never `STATIC PASS`.

## Common Mistakes

- Broadly decompiling the entire archive instead of the failed boundary.
- Copying reference offsets directly into production code.
- Treating one unique signature as proof of semantic ownership.
- Logging raw addresses, row data, account data, or full memory dumps.
- Storing generated IL inside the repository.
- Combining path discovery and destructive cleanup across shells.
- Claiming game compatibility from static analysis or one live observation.

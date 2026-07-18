# Development Safety and Packaging Automation Design

## Goal

Turn the main lessons from the trait-cap development session into repeatable safeguards: one cross-language equipment-analysis contract, a recoverable React failure path, one local packaging command, and a smoke-test checklist that matches current window behavior.

## Scope

This change includes:

- A shared JSON fixture for the Tauri equipment-analysis response.
- Rust serialization tests and React rendering/normalization tests against that fixture.
- A top-level React error boundary that replaces a blank window with a recovery message.
- A PowerShell packaging script that runs every required gate, synchronizes `hook.dll`, builds the MSI, verifies hashes, and updates the two artifact-hash records.
- Corrections and additions to the game 2.0.2 manual smoke-test checklist.

It does not include npm dependency upgrades, broad Rust dead-code cleanup, automatic DLL injection instructions, or automatic completion of manual game checks.

## Equipment Analysis Contract

Create one fixture under `src/fixtures/` using the exact JSON shape emitted by Tauri. It contains a connected response, one character, Damage Cap at level 70 with a verified cap of 65, a five-level overflow, and contributing primary and secondary sigil traits.

Rust reads the same file during a backend test, constructs the corresponding response, serializes it with `serde_json`, and compares the complete JSON value. This protects camelCase field names, enum string values, optional values, and nested source fields.

React imports the fixture as unknown data and passes it through a small handwritten normalizer before storing it. The normalizer accepts only the fields needed by the current UI, drops malformed collection entries, and represents missing optional numeric values as absent rather than calling methods on `undefined`. It introduces no schema dependency.

The equipment page test renders the normalized fixture and verifies `70 / 65`, `5 초과`, and the contributing sigil details. A malformed fixture test proves that incomplete source data cannot crash rendering.

## React Failure Isolation

Add one reusable top-level class error boundary around the router content. Unexpected render errors show a compact Korean recovery view with the Djeeta MOD name, an explanation that the screen could not be displayed, and a button that reloads the current application window.

The boundary is a last-resort safeguard, not a substitute for normalization. Its test renders a component that throws and verifies the fallback and reload action without allowing the exception to blank the entire test document.

## Packaging Script

Add `scripts/package.ps1`. The script resolves every path relative to its own directory and uses terminating error behavior. It performs these steps in order:

1. Confirm the script is running on Windows from the expected repository layout.
2. Confirm `granblue_fantasy_relink.exe` is not running before doing expensive work.
3. Resolve `node`, `npm`, and Cargo. Node 20 is the supported version; a newer locally installed Node may continue with a prominent warning so the current workstation can still package, while versions below 20 fail.
4. Run `npm ci`, formatting, lint, type-check, frontend tests, and the frontend build.
5. Run the locked release hook build and locked workspace tests.
6. Copy `target/release/hook.dll` to `src-tauri/hook.dll`.
7. Build one MSI through Tauri with Cargo available on `PATH`.
8. Require SHA-256 equality between the release and bundled hook DLLs.
9. Replace the existing MSI and hook SHA-256 values in `README.md` and `docs/testing/game-2.0.2-smoke-test.md`.
10. Run `git diff --check` and print the MSI path, MSI hash, hook hash, and documentation files changed.

The script never stops the game, deletes files, stages changes, commits, pushes, or marks manual smoke checks complete. A running game is an actionable failure because it locks the bundled hook.

PowerShell-focused tests invoke helper functions without running a real build. To keep the script testable, command-independent logic for version validation, hash replacement, and process preflight is placed in `scripts/PackageHelpers.psm1`; `package.ps1` remains the orchestration entry point. `scripts/tests/PackageHelpers.Tests.ps1` uses built-in PowerShell assertions and exit codes, so Pester is not required.

## Manual Smoke-Test Update

Replace the obsolete click-through and resize scenario with current behavior:

- The sidebar Damage Meter switch shows and hides the meter.
- The fixed-size meter moves by dragging the header and has no scrollbar.
- Only the management window appears on the taskbar.
- The meter stays always on top; management Always on Top defaults off.
- Opening a character in the equipment screen publishes its traits, and the verified Narmaya sample shows Damage Cap `70 / 65` and `5 초과`.

Keep every result checkbox and actual-result field manual. The document continues to forbid a game 2.0.2 compatibility claim until all required scenarios have recorded results.

## Error Handling

- Invalid contract entries are ignored or displayed with an unavailable marker; they do not abort the whole response.
- Tauri invocation failures leave the equipment page in its current waiting/error state and do not poison existing valid state.
- Every external command failure stops packaging immediately and reports the failed command.
- Missing tools, an unsupported old Node version, a running game, a missing hook, a missing MSI, unequal hook hashes, or an unexpected hash-document format are hard failures.

## Verification

- Demonstrate RED then GREEN for the shared fixture contract, malformed React payload, error boundary, and packaging helper tests.
- Run the PowerShell helper tests.
- Run `npm ci`, format check, lint, type-check, all frontend tests, and the production build.
- Run the locked release hook build and all locked Rust workspace tests.
- Execute `scripts/package.ps1` locally and confirm it creates an MSI, synchronizes hook hashes, and records current hashes.
- Run `git diff --check` and verify `logs.db` remains untouched and untracked.

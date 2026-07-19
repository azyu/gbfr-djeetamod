# NSIS Per-User Installer Design

## Goal

Replace the per-machine MSI distribution with one NSIS installer that installs Djeeta MOD for the current Windows user without requiring administrator elevation. Keep the application process at the same normal integrity level as a Steam-launched Granblue Fantasy: Relink process.

## Scope and constraints

- Produce NSIS only; do not keep a parallel MSI release path.
- Configure Tauri's NSIS installer with `installMode: "currentUser"`.
- Keep the application manifest at `asInvoker`.
- Preserve automatic `hook.dll` injection and all current meter, combat-log, and equipment-analysis behavior.
- Do not add Defender exclusions, restore quarantined files, disable Windows security features, or claim that changing installer scope eliminates behavior-based detection.
- The NSIS installer, application executable, and hook remain unsigned until a separate signing decision is made.
- Existing MSI installations are not migrated automatically. The user guide must instruct users to uninstall the old `C:\Program Files\Djeeta MOD` installation before running the NSIS installer.

## Packaging configuration

Change `src-tauri/tauri.conf.json` so the configured bundle target is `nsis`, and add `windows.nsis.installMode` with the exact value `currentUser`. Retain the existing WebView2 bootstrapper mode and the previously hardened WebView arguments.

The current-user NSIS mode owns installer metadata under the current user's registry hive and chooses a location that does not require administrator access. The project must not depend on one hard-coded absolute user profile path.

The disabled updater configuration is not part of this change. Its `basicUi` value controls updater presentation, not installation scope.

## Automated safeguards

Extend the existing security configuration test to read the real Tauri configuration and require all of the following:

- `bundle.targets` contains only `nsis`;
- `bundle.windows.nsis.installMode` is `currentUser`;
- no MSI bundle target remains;
- the application manifest remains `asInvoker` and does not require administrator elevation;
- the WebView arguments do not disable SmartScreen protection.

Convert the PowerShell packaging helper tests before changing the implementation. Replace MSI-specific artifact selection with NSIS setup-executable selection. Selection must require exactly one fresh setup executable matching the configured product name, version, and x64 target; it must not mistake the unpackaged application executable for the installer.

Replace MSI-specific hash rewriting with installer-generic rewriting that updates exactly one NSIS installer hash and one `hook.dll` hash in each active release document. Invalid hashes, missing labels, duplicate labels, stale artifacts, or multiple matching installers remain hard failures.

## Packaging workflow

Keep `scripts/package.ps1` as the single packaging orchestrator, but change its user-facing command from `package:msi` to `package:nsis`. The workflow remains:

1. Refuse to package while the game is running.
2. Validate the Node toolchain and run the PowerShell helper tests.
3. Run formatting, linting, TypeScript checks, frontend tests, and the frontend production build.
4. Build the release hook and run every Rust workspace test.
5. Copy the release hook into the Tauri resource location.
6. Build only the NSIS bundle for the configured application binary.
7. Require one fresh matching installer under `target/release/bundle/nsis`.
8. Require SHA-256 equality between the release and bundled hook DLLs.
9. Calculate the NSIS installer SHA-256 and update the active release records.
10. Run `git diff --check` and print `InstallerPath`, `InstallerSHA256`, `HookSHA256`, and `HookHashesEqual`.

Update `AGENTS.md` so its required verification and final artifact checks describe NSIS rather than MSI.

## User documentation and migration

Update the Korean and English README instructions to describe the NSIS setup executable. Add a visible migration note telling users with the previous MSI build to uninstall it through Windows Installed Apps before installing the per-user build. Do not implement registry probing, silent MSI removal, file copying, or automatic cleanup of `C:\Program Files\Djeeta MOD`.

Update `docs/testing/game-2.0.2-smoke-test.md` to record the NSIS installer hash and use the new installer for manual installation steps. Historical specifications and implementation plans remain historical and are not rewritten.

## Verification and acceptance

Implementation follows test-first changes for the Tauri security contract and PowerShell artifact helpers. Completion requires:

- focused security and PowerShell helper tests pass;
- `npm ci`, formatting, linting, TypeScript checks, all frontend tests, and the production build pass;
- the release hook build and every Rust workspace test pass;
- Tauri produces exactly one current-user NSIS setup executable;
- release and bundled `hook.dll` hashes are equal;
- README and the active smoke-test document contain the final NSIS and hook hashes;
- `git diff --check` passes;
- unrelated `logs.db` remains untouched.

The new installer is ready for a separately approved Defender execution test after automated verification. Granblue Fantasy: Relink 2.0.2 compatibility remains unverified until the existing in-game smoke-test checklist is completed in an offline or private session.

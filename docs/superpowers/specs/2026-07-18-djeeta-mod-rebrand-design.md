# Djeeta MOD Rebrand Design

## Goal

Rename the distributable product to `Djeeta MOD` while preserving upstream credit, the existing damage-meter behavior, and the unverified-test-build disclosure for game 2.0.2.

## Product identity

- User-facing product name: `Djeeta MOD`
- npm package name: `djeeta-mod`
- Tauri bundle identifier: `com.azyu.djeeta-mod`
- MSI, window titles, settings/log headings, Rust descriptions, Windows DLL metadata, and Korean README use the new name.
- The MIT license and credit for False Spring and onelittlechildawa remain unchanged.

Changing the bundle identifier is intentional because this project has not been released as a stable product. The new identity may use a separate application-data and installer identity from the earlier test build.

## Documentation

`README.md` remains the Korean end-user guide. It explains installation, the compact overlay, performance expectations, DLL-injection risk, unverified game compatibility, source builds, hashes, and upstream credit.

Create root-level `AGENTS.md` as the reusable maintainer handoff. It records:

- project purpose and supported target;
- architecture and important paths;
- exact Rust/Node/Visual Studio requirements;
- required build, test, packaging, and hash commands;
- reward-boundary and connection-handshake invariants;
- Korean-default and compact-overlay requirements;
- manual game smoke-test requirement;
- preservation of upstream license and credit.

## Performance position

The mod does not modify the game's graphics settings or rendering quality. It does add CPU and memory work through the injected damage hook, named-pipe parsing, and an external transparent WebView. The compact UI publishes at 250 ms intervals and the WebView runs with GPU acceleration disabled, so expected GPU impact is small. No claim of zero performance impact is made until an in-game comparison is recorded.

No performance optimization or hook behavior change is included in this rebrand.

## Verification and packaging

1. Confirm all old distributable names are removed from manifests and visible UI, except historical upstream-credit text.
2. Run formatting, linting, type checking, frontend tests, and the complete locked Rust test suite.
3. Build the release hook and MSI from the final source.
4. Verify `target/release/hook.dll` and `src-tauri/hook.dll` have equal SHA-256 values.
5. Replace the README and smoke-test hashes with the newly built artifacts.
6. Re-run manifest, hash, and working-tree checks.

The MSI remains labeled as an unverified 2.0.2 test build until every item in `docs/testing/game-2.0.2-smoke-test.md` is completed in an offline or private game session.

## Commit structure

- Design document: `docs: define Djeeta MOD rebrand`
- Product rename, README, `AGENTS.md`, rebuilt metadata and hash updates: `chore: rebrand release as Djeeta MOD`

Existing implementation commits remain unchanged because they are already separated by lifecycle, resilience, model, UI, localization, and release preparation concerns.

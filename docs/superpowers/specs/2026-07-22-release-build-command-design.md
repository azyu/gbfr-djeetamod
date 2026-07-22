# Release Build Command Design

## Goal

Provide a double-clickable Windows command file for the local maintainer to run the existing signed release build without manually preparing updater environment variables. The canonical build remains `npm.cmd run package:nsis`; the new entry point only prepares credentials securely and delegates to it.

## Files and responsibilities

- `build-release.cmd` is the repository-root entry point. It resolves paths relative to itself, invokes the PowerShell wrapper, preserves its exit code, and pauses so a double-clicked window remains readable.
- `scripts/build-release.ps1` owns credential preparation. It resolves the repository root and `%USERPROFILE%\.djeeta-mod\updater.key`, verifies that the key file exists, prompts for the password as a `SecureString`, exposes both values only through the current child process environment, and invokes `npm.cmd run package:nsis` from the repository root.
- `scripts/package.ps1` remains the only implementation of version agreement, verification, Tauri compilation, updater signing, artifact selection, manifest generation, hashing, and documentation updates.

## Data flow

1. The maintainer double-clicks `build-release.cmd` or runs it from a terminal.
2. The command file starts `scripts/build-release.ps1` with Windows PowerShell.
3. The wrapper reads the private key from `%USERPROFILE%\.djeeta-mod\updater.key` without printing it.
4. The wrapper requests the key password with a masked prompt and converts it only for the duration required by Tauri.
5. The wrapper calls `npm.cmd run package:nsis` without a requested version. `scripts/package.ps1` uses the package version and verifies that Cargo and Tauri versions match it.
6. A `finally` block clears `TAURI_PRIVATE_KEY` and `TAURI_KEY_PASSWORD` and releases the unmanaged password buffer on success or failure.
7. The command file returns the build exit code after pausing for inspection.

## Security and failure behavior

- Neither the private key nor password is accepted as a command-line argument, committed, printed, or stored in a generated file.
- A missing key fails before starting the expensive build and identifies the expected path.
- The game-running, version-agreement, signing, test, artifact, and hash checks remain in the canonical packager.
- Cleanup runs even when npm, Cargo, Tauri, or signing fails.
- The wrapper uses the current user's profile instead of hardcoding `C:\Users\azyu`, while resolving to the approved location on this machine.

## Verification

- A static regression test requires the command file to invoke only the PowerShell wrapper and preserve its exit code.
- The same test requires the wrapper to use `Read-Host -AsSecureString`, the expected per-user key path, `try`/`finally`, unmanaged-buffer zeroing, environment-variable cleanup, and `npm.cmd run package:nsis`.
- The test rejects password literals and direct password command-line arguments.
- PowerShell parser validation checks the wrapper syntax.
- Existing package-helper, security-configuration, frontend, Rust, and release-build verification remain authoritative.

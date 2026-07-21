# Djeeta MOD Maintainer Guide

## Product contract

- Public name: `Djeeta MOD`
- Package name: `djeeta-mod`
- Tauri identifier: `com.azyu.djeeta-mod`
- Version: `0.1.1`
- Target: Granblue Fantasy: Relink Endless Ragnarok 2.0.2 on Windows x64
- Default language: Korean
- Compatibility remains unverified until `docs/testing/game-2.0.2-smoke-test.md` is completed.

## Architecture

- `src-hook/`: injected Rust DLL; captures player identity, damage, and the reward boundary.
- `protocol/`: append-only bincode wire messages shared by the DLL and Tauri backend.
- `src-tauri/`: named-pipe client, encounter parser, persistence, Tauri commands, and Windows packaging.
- `src/`: React compact overlay and logs/settings UI.

## Behavioral invariants

- First accepted hit starts an encounter.
- All targets in one battle contribute to the same party totals.
- Inactivity must not split or hide a live encounter.
- The meter clears immediately before the reward UI.
- `HookStatus::Unsupported` is latched; later gameplay frames must not mark the connection ready.
- Unknown enemies are ignored unless a verified player identity owns the actor.
- The overlay shows at most four rows: Korean character name, cumulative damage/bar, and DPS.
- All four overlay rows must fit inside the 330x145 meter without clipping; keep the 11px row text and 12px header text unless the user requests a typography change.
- Presentation publishes every 250ms and bars transition over 150ms.
- 1920x1080 reset geometry is 330x145 at x45/y470.
- The compact meter stays always-on-top, is omitted from the taskbar, and starts with click-through disabled so its header can be dragged.

## Toolchain

- Node.js 20
- Visual Studio 2022 C++ Build Tools and Windows SDK
- rustup toolchain from `rust-toolchain.toml` (`nightly-2024-05-04`)
- WebView2 and NSIS as used by Tauri 1

Load the Visual Studio developer environment before Rust builds when the shell does not already expose MSVC.

On Windows PowerShell, invoke Node package binaries as `npm.cmd` and `npx.cmd`. When inspecting Korean text, use UTF-8-capable output or verify the file bytes/diff directly; mojibake in terminal output is not evidence that the file is corrupted.

## Required verification

For ordinary frontend changes, run the narrow regression test first, then `npm.cmd run format-check`, `npm.cmd run lint`, `npm.cmd run tsc`, `npm.cmd test -- --run`, and `npm.cmd run build`.

For Rust, hook, or protocol changes, also run `cargo build --release --locked --package hook` and `cargo test --workspace --all-targets --locked` after the focused regression test.

For a release build, use `npm.cmd run package:nsis` as the single canonical entry point. Do not manually duplicate its install, frontend, Rust, Tauri, hook-copy, or hash-update steps.

Before packaging, check for an exact `Djeeta MOD` process and stop it only when needed to release locked build files. Do not stop the game process merely to package the application.

After packaging, independently verify that the NSIS installer exists, SHA-256 hashes of `target/release/hook.dll` and `src-tauri/hook.dll` are equal, and the installer and hook hashes appear in both `README.md` and `docs/testing/game-2.0.2-smoke-test.md`. Commit those two generated hash-document changes together.

Node.js 20 is the supported toolchain. If packaging succeeds on another Node.js version, report it as a non-standard environment rather than implying that version is supported.

## Change discipline

- Work on `master` only when the user explicitly requests it.
- Execute implementation plans inline in the current session by default. Use subagents or a separate worktree only when the user explicitly requests them.
- For an unambiguous change limited to CSS, copy, static configuration, assets, or maintainer documentation, skip separate design/specification and implementation-plan documents unless the user requests them or an unresolved product choice remains. Implement the smallest change directly, add or update a regression test when behavior changes, verify it, and use one implementation commit instead of intermediate planning commits.
- Use tests before changing lifecycle, hook, parser, handshake, geometry, or throttling behavior.
- Append protocol variants; never reorder existing bincode variants.
- Treat `logs.db` as user data: do not read, modify, delete, stage, or commit it unless the user explicitly asks to inspect that database.
- Preserve `LICENSE` and upstream credit for False Spring and onelittlechildawa.
- Automated tests and successful packaging do not establish game compatibility. Do not claim game 2.0.2 compatibility before the manual smoke-test checklist passes in an offline or private session.

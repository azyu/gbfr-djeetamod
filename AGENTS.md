# Djeeta MOD Maintainer Guide

## Product contract

- Public name: `Djeeta MOD`
- Package name: `djeeta-mod`
- Tauri identifier: `com.azyu.djeeta-mod`
- Version: `0.1.0`
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
- Presentation publishes every 250ms and bars transition over 150ms.
- 1920x1080 reset geometry is 330x145 at x45/y470.
- Normal mode is click-through.

## Toolchain

- Node.js 20
- Visual Studio 2022 C++ Build Tools and Windows SDK
- rustup toolchain from `rust-toolchain.toml` (`nightly-2024-05-04`)
- WebView2 and WiX as used by Tauri 1

Load the Visual Studio developer environment before Rust builds when the shell does not already expose MSVC.

## Required verification

```powershell
npm ci
npm run format-check
npm run lint
npm run tsc
npm test -- --run
npm run build
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
npm run tauri build -- --bundles msi
```

After packaging, require SHA-256 equality between `target/release/hook.dll` and `src-tauri/hook.dll`, then record the MSI and hook hashes in `README.md` and `docs/testing/game-2.0.2-smoke-test.md`.

## Change discipline

- Work on `master` only when the user explicitly requests it.
- Execute implementation plans inline in the current session by default. Use subagents or a separate worktree only when the user explicitly requests them.
- Use tests before changing lifecycle, hook, parser, handshake, geometry, or throttling behavior.
- Append protocol variants; never reorder existing bincode variants.
- Preserve `LICENSE` and upstream credit for False Spring and onelittlechildawa.
- Do not claim game 2.0.2 compatibility before the manual smoke-test checklist passes in an offline or private session.

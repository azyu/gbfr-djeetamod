# Djeeta MOD Maintainer Guide

## Product contract

- Public name: `Djeeta MOD`
- Package name: `djeeta-mod`
- Tauri identifier: `com.azyu.djeeta-mod`
- Version: `0.1.2`
- Target: Granblue Fantasy: Relink Endless Ragnarok 2.0.2 on Windows x64
- Default language: Korean
- Compatibility remains unverified until `docs/testing/game-2.0.2-smoke-test.md` is completed.

## Architecture

- `src-hook/`: injected Rust DLL; captures player identity, damage, and the reward boundary.
- `protocol/`: append-only bincode wire messages shared by the DLL and Tauri backend.
- `src-tauri/`: named-pipe client, encounter parser, persistence, Tauri commands, and Windows packaging.
- `src/`: React compact overlay and logs/settings UI.

## Maintainer references

- Relink file extraction, tables, IDs, and reverse-engineering index: [`docs/research/2026-07-24-relink-modding-reference.md`](docs/research/2026-07-24-relink-modding-reference.md)

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

## Release and updater operations

- Keep maintainer-only Rust tools out of `src-tauri/src/bin/`. Tauri 1 may still collect files there as bundle binaries even when Cargo declares them as examples; place them under `src-tauri/examples/` or another non-bin path.
- A warm `target/` directory can hide a missing bundle binary. Reproduce unexplained NSIS `os error 2` failures with an isolated `CARGO_TARGET_DIR` and, when Tauri tool downloads may affect the result, an isolated `LOCALAPPDATA` before treating the problem as CI-only.
- Store `TAURI_PRIVATE_KEY` as the exact one-line contents of the generated updater key file. When updating the GitHub environment secret, pass the file through standard input and never print the key or place it directly in a logged command line.
- GitHub normalizes spaces in uploaded release asset filenames to periods. Generate updater manifest URLs and remote asset comparisons with `ConvertTo-GitHubReleaseAssetName`; do not assume the local Tauri filename is the remote filename.
- Do not use the single-release-by-tag API to inspect a newly created Draft Release. Query the authenticated releases list and use a bounded retry because a successful `gh release create` may not be immediately visible.
- A failed Release workflow may already have pushed a hash-document commit and tag or created a Draft with uploaded assets. Inspect remote `main`, the exact tag, the authenticated release list, and asset state before rerunning; never blindly rerun the workflow.
- If rebuilding is necessary, remove only the confirmed Draft release ID and exact version tag, then verify both are absent before dispatching again. Never delete a published release as cleanup for a failed workflow.
- If failure occurs after asset upload, preserve and independently validate the existing Draft before choosing to rebuild. Do not spend another full release build merely to obtain a green workflow unless the Draft is invalid or the user explicitly requests a rerun.
- Independently verify every release after the workflow: it remains a Draft unless publication was requested, exactly four assets exist, every available remote digest matches a fresh download, and `latest.json` contains the uploaded updater archive URL and the exact `.sig` contents.
- Distinguish an ordinary local frontend/Rust build from the canonical signed NSIS package. Do not report local signed packaging as successful unless `npm.cmd run package:nsis` completed with the updater key and password; a successful unsigned Tauri or frontend build is not equivalent.
- Publish a Draft Release only on an explicit user request. After publishing, verify that it is no longer a draft, has a publication timestamp, remains the latest release, and still has four uploaded assets.
- GitHub Actions may force action runtimes onto Node.js 24 even when the workflow configures Node.js 20. Treat and report that as a non-standard environment warning while separately reporting the actual build result.

## Change discipline

- Work on `master` only when the user explicitly requests it.
- Execute implementation plans inline in the current session by default. Use subagents or a separate worktree only when the user explicitly requests them.
- For an unambiguous change limited to CSS, copy, static configuration, assets, or maintainer documentation, skip separate design/specification and implementation-plan documents unless the user requests them or an unresolved product choice remains. Implement the smallest change directly, add or update a regression test when behavior changes, verify it, and use one implementation commit instead of intermediate planning commits.
- Use tests before changing lifecycle, hook, parser, handshake, geometry, or throttling behavior.
- Append protocol variants; never reorder existing bincode variants.
- Treat `logs.db` as user data: do not read, modify, delete, stage, or commit it unless the user explicitly asks to inspect that database.
- Preserve `LICENSE` and upstream credit for False Spring and onelittlechildawa.
- Automated tests and successful packaging do not establish game compatibility. Do not claim game 2.0.2 compatibility before the manual smoke-test checklist passes in an offline or private session.

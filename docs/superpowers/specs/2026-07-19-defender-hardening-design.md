# Defender Hardening Design

## Goal

Reduce avoidable Windows Defender risk signals while preserving the current game connection behavior and keeping the first experiment small enough to identify whether the security changes affect detection.

## Current evidence

- The release application embeds a manifest with `requireAdministrator` even though Granblue Fantasy: Relink runs with Steam's normal user permissions.
- Both Tauri windows launch WebView2 with `msSmartScreenProtection` explicitly disabled.
- The application automatically injects `hook.dll` into the game through `dll-syringe` when it finds the game process.
- The MSI, application executable, and hook DLL are currently unsigned.
- Defender quarantined the installed application executable as `Behavior:Win32/DefenseEvasion.A!ml` after the game started.

Defender does not expose which individual signal caused the classification. This design therefore removes the unnecessary privilege and security-disable signals while leaving injection timing unchanged for the first comparison.

## Changes

### Process privilege

Change the release application manifest from `requireAdministrator` to `asInvoker`. The application and a normally launched Steam game will then run at the same integrity level. If injection fails under this condition, that result is evidence for a separate follow-up design; this change does not add an elevated helper.

### WebView2 security

Remove `msSmartScreenProtection` from `additionalBrowserArgs` for both Tauri windows. Preserve the existing `msWebOOUI`, `msPdfOOUI`, and `--disable-gpu` arguments because changing unrelated WebView behavior would make the Defender comparison less controlled.

### Injection behavior

Keep automatic game detection and `dll-syringe` injection unchanged. A manual connect button may improve transparency but would not remove the injection behavior, so it is outside this first experiment.

## Regression protection

Add a focused test that reads the release manifest and Tauri configuration as real files and asserts:

- the requested execution level is `asInvoker`;
- `requireAdministrator` is absent;
- neither window disables `msSmartScreenProtection`;
- the remaining approved WebView arguments are unchanged.

The test must fail against the current configuration before production files are changed.

## Verification

Run the focused test, the full project format/lint/type/test/build suite, the release hook build, Rust workspace tests, and MSI packaging. Confirm release and bundled hook SHA-256 equality and refresh the documented MSI hash.

Do not automatically restore quarantined files, add Defender exclusions, disable behavior monitoring, or submit files externally. After packaging, the new MSI and executable can be scanned or executed in a separate user-approved Defender test. Granblue Fantasy: Relink 2.0.2 compatibility remains unverified until the in-game smoke-test checklist is completed.

## Deferred work

- Trusted Authenticode signing requires a public-trust signing identity or certificate and is not simulated with a self-signed public release.
- Microsoft false-positive submission is an external action and will use the newly hardened binaries only after explicit approval.
- An explicit connect button or separate injection helper requires a new design if the minimal hardening does not produce an acceptable result.

# Release Workflow Security Design

## Goal

Prevent workflow-dispatch input from becoming PowerShell code and keep the Tauri updater private key unavailable to dependency installation, tests, compilation, and release publication.

## Scope

This change covers only the signed NSIS release path:

- `.github/workflows/release.yaml`
- `scripts/package.ps1`
- a new signing/finalization script
- package scripts and focused regression tests

Node, Rust, Tauri, Vite, and Vitest version upgrades remain separate follow-up security stages.

## Design

### 1. Treat dispatch inputs as data

The workflow passes `inputs.version` and `inputs.publish` through step-scoped environment variables. PowerShell reads `$env:RELEASE_VERSION` and `$env:PUBLISH_RELEASE`; GitHub expressions are never interpolated into the inline PowerShell source.

The existing stable `X.Y.Z` validation remains the authoritative version check.

### 2. Separate preparation, signing, and publication

The release job has three trust phases:

1. **Prepare:** install dependencies, run required verification, build the hook and application, create the NSIS installer, and create the updater ZIP. This phase receives neither updater signing secrets nor Git push credentials.
2. **Sign:** expose `TAURI_PRIVATE_KEY` and `TAURI_KEY_PASSWORD` only to a script that validates the prepared artifacts, signs the updater ZIP with `tauri signer sign`, and creates the final updater manifest and package summary.
3. **Publish:** remove signing secrets from scope, authenticate Git and GitHub only after untrusted dependency/build execution has finished, then commit hashes, push, tag, create the draft release, upload assets, and verify the remote release.

`actions/checkout` uses `persist-credentials: false`. Git credentials are configured immediately before the publish operations.

### 3. Preserve the canonical packaging entry point

`npm.cmd run package:nsis` remains the canonical preparation command. It no longer requires signing secrets and produces:

- the current-build NSIS installer;
- `target/release/bundle/nsis/Djeeta MOD_<version>_x64-setup.nsis.zip`;
- a preparation summary containing exact artifact paths, version, timestamps, and hashes needed by the signing phase.

A new `npm.cmd run package:sign` command:

- requires both updater signing environment variables;
- rejects missing, stale, wrong-product, or wrong-version preparation artifacts;
- signs only the prepared updater ZIP;
- writes `latest.json` with the exact generated signature;
- verifies installer, hook, archive, and bundled-hook hashes;
- updates the two release hash documents;
- writes the final `package-summary.json` consumed by the publish phase.

The updater ZIP contains exactly one entry: the generated NSIS installer with its original filename and bytes. Tauri 1 extracts it as a standard ZIP archive; the ZIP compression method is not a security boundary.

### 4. Failure behavior

Every phase fails closed:

- malformed dispatch input fails before packaging;
- preparation fails if the game is running or verification/build steps fail;
- signing fails if secrets or prepared artifacts are missing, stale, or inconsistent;
- publication fails if the final summary, hashes, tag, draft state, asset set, downloaded manifest, or signature differs from the local result.

A failed preparation never sees release credentials. A failed signing phase cannot push or publish. A failed publication phase no longer has the updater private key.

## Testing

Focused tests will prove that:

- workflow-dispatch expressions do not appear inside PowerShell source;
- signing secrets are absent from job-level environment and present only on the signing step;
- checkout does not persist Git credentials;
- preparation and publication steps do not receive signing secrets;
- the signing step does not run dependency installation, tests, Cargo, or Tauri build;
- the updater ZIP contains exactly the intended installer;
- the signing phase rejects missing or inconsistent preparation state;
- existing draft-release and remote-asset verification remains intact.

After focused tests pass, run the project-required frontend verification:

1. `npm.cmd run format-check`
2. `npm.cmd run lint`
3. `npm.cmd run tsc`
4. `npm.cmd test -- --run`
5. `npm.cmd run build`

Packaging itself is not executed without the updater private key. The PowerShell helper tests and static workflow regression tests verify the release path locally.

## Security boundaries and limitations

This design isolates the updater minisign key and Git publication credentials within one job. A future separate signing job or external signing service would provide stronger runner isolation but is intentionally outside this stage.

Windows Authenticode signing is also outside this stage; the Tauri updater signature protects update archives but does not authenticate the initial installer to Windows.

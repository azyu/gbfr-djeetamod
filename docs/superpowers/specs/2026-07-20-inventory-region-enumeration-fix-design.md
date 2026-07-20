# Inventory Region Enumeration Fix Design

## Context

The debug-only inventory probe currently enumerates remote memory until the hard-coded address `0x0000_7FFF_FFFF_FFFF`. On the validation machine, `GetNativeSystemInfo` reports `0x0000_7FFF_FFFE_FFFF` as the maximum application address. After the final valid region, `VirtualQueryEx` therefore receives an out-of-range address, returns zero with `ERROR_INVALID_PARAMETER`, and the probe reports `INTERNAL stage=region-enumeration`.

The probe must continue to fail closed for genuine query failures. Treating every `ERROR_INVALID_PARAMETER` as successful completion would hide an unexpected failure within the valid address range.

## Selected Approach

Use `GetNativeSystemInfo` to obtain the process-independent native application address range before enumeration.

- Start at `lpMinimumApplicationAddress` instead of address zero.
- Query while the next address is at or below `lpMaximumApplicationAddress`.
- Stop successfully once advancing past the reported maximum address.
- Keep zero-byte `VirtualQueryEx` results and zero-sized regions as errors while the address remains within that range.
- Preserve `PROCESS_QUERY_INFORMATION | PROCESS_VM_READ`; no elevation or write capability is added.

The address-bound decision will be a pure helper so boundary behavior can be tested without invoking Windows APIs.

## Alternatives Considered

### Treat terminal `ERROR_INVALID_PARAMETER` as success

This is smaller, but it cannot reliably distinguish the expected end of the address space from a premature invalid query. It weakens the fail-closed behavior added during review.

### Keep a hard-coded architecture limit

Changing the constant to the value observed on one machine would fix this run but retain the same environmental assumption. Windows already exposes the correct native range, so duplicating it is unnecessary.

## Data Flow and Error Handling

1. Open the game process with the existing read-only rights.
2. Read the native minimum and maximum application addresses.
3. Enumerate committed, readable, private regions within those bounds.
4. Return an error immediately for a failed query inside the valid range.
5. Pass the complete region list to the existing bounded inventory scanner.

No raw addresses or Windows error details are exposed to the frontend. Backend diagnostic logs retain only the existing stable public stage and status vocabulary.

## Verification

- Add a Rust unit test covering addresses below, equal to, and above the native maximum.
- Run the focused memory and inventory probe tests.
- Run the full Rust workspace and frontend test suites.
- Restart the debug app with `DJEETA_INVENTORY_PROBE=1` while the pinned game process remains open.
- Capture once from the unfiltered sigil inventory and record whether the result reaches `STABLE`, `UNAVAILABLE`, `AMBIGUOUS`, or `LIMIT_EXCEEDED` instead of `INTERNAL stage=region-enumeration`.
- Compare candidate and occupied counts with the in-game count only if the result is `STABLE`.

## Out of Scope

- Scanner performance optimization if the run reaches the 60-second limit.
- Suppressing the existing injected hook; that is stage 2.
- Renaming or reorganizing the Equipment Analysis UI; that is stage 3.
- Claiming full inventory compatibility before the manual checklist is complete.

## Success Criteria

- Enumeration uses the native Windows application address range.
- Premature `VirtualQueryEx` failures still fail closed.
- Automated tests pass.
- The actual-game capture no longer fails at `region-enumeration`.

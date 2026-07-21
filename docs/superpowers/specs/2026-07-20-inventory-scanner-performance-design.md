# Inventory Scanner Performance Design

## Context

The debug-only, read-only inventory probe now enumerates the correct native Windows application address range, but its first actual-game scan reached `LIMIT_EXCEEDED` after examining 1,727 regions and requesting 142,938,602 bytes over 61.499 seconds.

The remote reads are already grouped into 4 MiB chunks. The dominant cost is the local candidate loop: it attempts a complete `0x24` inventory-record decode at every 4-byte-aligned position. For the observed scan this is approximately 35.7 million complete decode attempts in an unoptimized debug build.

GBFRER Helper uses a useful candidate-first idea: search for known sigil or trait byte sequences, then expand only matching locations into plausible record runs. This design adopts only that search strategy. It does not adopt GBFRER Helper's administrator requirement, temporary code hook, remote allocation, process-memory writes, largest-run assumption, or early termination.

Reference: <https://github.com/didigns/GBFRER_Helper/blob/main/extract_sigils.py>

## Goals

- Complete an actual-game debug scan and candidate validation within 10 seconds.
- Preserve a complete search of every enumerated readable private region.
- Preserve fail-closed `UNAVAILABLE`, `AMBIGUOUS`, `UNSTABLE`, and `LIMIT_EXCEEDED` outcomes.
- Keep the process handle at `PROCESS_QUERY_INFORMATION | PROCESS_VM_READ` and the application at `asInvoker`.
- Avoid decoding arbitrary 4-byte-aligned positions as complete inventory records.

## Selected Approach

Use a two-phase known-ID search followed by complete validation of every discovered run.

### Phase 1: Discover occupied-record anchors

1. Expose a read-only iterator over the inventory catalog's known, non-empty sigil IDs.
2. Convert each ID to its 4-byte little-endian representation.
3. Build one `aho-corasick` multi-pattern matcher for those byte sequences.
4. Read every enumerated readable private region in 8 MiB chunks, with a 3-byte overlap so raw four-byte matching remains complete at chunk edges. Reuse one read buffer across all regions, and keep valid anchors subject to the separate 4-byte alignment check.
5. For each match, subtract the verified sigil field offset `0x10` to obtain a possible record address.
6. Reject underflow, record ranges that are not fully inside the source region, and addresses that are not 4-byte aligned.
7. Sort and deduplicate the possible record addresses.

The empty IDs `0` and `EMPTY_HASH` are excluded from the matcher. Empty memory therefore does not create anchors. A real qualifying inventory has at least six occupied records, so it necessarily contains a known non-empty sigil ID under the pinned 2.0.2 catalog contract.

### Phase 2: Validate and expand every anchor

For each deduplicated anchor:

1. Skip it only if it falls on the same `0x24` record phase inside a previously validated run. An overlapping anchor on a different phase is validated independently.
2. Read a 64 KiB window around the anchor, clamped to the containing enumerated region.
3. Fully decode the anchor with the existing shared inventory decoder.
4. Walk backward and forward at the verified `0x24` stride, decoding every record. When the next record is outside the current window, read the next adjacent 64 KiB window and continue.
5. Stop a direction at the first invalid record, the region boundary, an unavailable read, or the shared deadline.
6. Define the candidate from its first occupied record through its last occupied record, preserving internal empty records while excluding leading and trailing empty storage.
7. Accept the run only when that candidate still satisfies the existing minimum of 13 records and six occupied records.
8. Merge duplicate reports of the same address range.

After all anchors have been checked, zero accepted runs produces `UNAVAILABLE`, one produces the existing stable-read path, and more than one produces `AMBIGUOUS`. The scanner must not select the largest run or return after the first valid run.

## Runtime Limit

Remove the 16 GiB byte limit and the 60-second duration limit. Replace them with one 10-second deadline covering both discovery and candidate validation.

- Check the deadline before each remote chunk read.
- Check it while expanding candidate records in either direction.
- If it expires, discard all partial results and return `LIMIT_EXCEEDED` immediately.
- Continue to count requested bytes for diagnostics, but do not use the count as a termination condition.

This is a responsiveness guarantee, not a compatibility shortcut. Raising the deadline or accepting a partial candidate is not an allowed fallback.

## Components and Interfaces

### `equipment-core`

`InventoryCatalog` gains a read-only way to iterate its known non-empty sigil IDs. The existing decoder and its validation rules remain the authority for deciding whether a complete record is valid.

### Tauri inventory probe

The existing `scan_inventory` orchestration is split into small helpers with distinct responsibilities:

- build the known-ID matcher;
- discover aligned anchor addresses in one byte buffer;
- expand one anchor through adjacent buffered reads;
- normalize validated runs and classify the final outcome.

The production scan still receives only a `MemoryReader`, enumerated `MemoryRegion` values, the pinned catalog, and a duration limit. It does not receive or persist raw addresses from an earlier process session.

### Dependency

Add `aho-corasick` as a direct backend dependency and build its automaton as a DFA. Because the probe exists only in debug builds, compile `aho-corasick` and the `gbfr-logs` backend at dev `opt-level = 3`; debug assertions and the exact environment opt-in remain enabled. The 144 MiB synthetic scan must remain comfortably below the 10-second deadline.

## Data and Logging

Keep the current stable public status vocabulary. The backend diagnostic summary records only:

- enumerated region count;
- requested byte count;
- discovered anchor count;
- fully validated run count;
- elapsed milliseconds;
- final status.

Do not expose raw record bytes, complete inventory contents, player names, trait lists, or a reusable address cache. The existing final candidate diagnostic may retain its development-only address, count, occupied count, and digest behavior.

## Error Handling

- A discovery chunk that becomes unreadable is skipped, matching the existing race handling.
- An unavailable read while expanding an anchor rejects that anchor rather than accepting a partial run.
- Address arithmetic remains checked; overflow is `INTERNAL` at the existing scan stage.
- Matcher construction failure is `INTERNAL` and occurs before remote scanning.
- Deadline expiry is always `LIMIT_EXCEEDED`, never `UNAVAILABLE` or a partial success.
- A second stable read is still required after a unique run is found; changed bytes remain `UNSTABLE`.

## Testing

Use test-driven development and avoid wall-clock performance assertions in unit tests.

### Catalog tests

- Known non-empty sigil IDs are exposed.
- `0` and `EMPTY_HASH` are not returned as search patterns.

### Anchor-discovery tests

- A known sigil ID at record offset `0x10` produces the correct anchor.
- Unknown values and unaligned matches do not produce anchors.
- A record whose sigil field begins exactly at a chunk boundary is found exactly once.
- An overlapping raw-byte match is deduplicated and cannot bypass record alignment.
- Repeated matches deduplicate deterministically.

### Candidate-validation tests

- A qualifying run excludes leading and trailing empty records while preserving empty records between its first and last occupied records, matching the current scanner.
- A 12-record equipment snapshot remains excluded.
- Two distinct qualifying runs remain `AMBIGUOUS`.
- Multiple occupied anchors inside one run produce one candidate.
- A read that becomes unavailable rejects only the affected candidate.
- A changed second snapshot remains `UNSTABLE`.

### Work-bounding tests

Add diagnostic counters to the scan result and use a 144 MiB decoy buffer containing one valid run. Assert that the scan completes before the 10-second deadline and complete record validation is limited to discovered-anchor neighborhoods instead of every 4-byte position. Test the pure deadline decision separately rather than sleeping.

### Verification

- Run focused `equipment-core` and inventory-probe tests.
- Run the frontend security assertions to prove no write, operation, allocation, or remote-thread rights were added.
- Run the full frontend and Rust regression suites.
- With the pinned game already running and `DJEETA_INVENTORY_PROBE=1`, capture once from the unfiltered sigil inventory.
- Require completion within 10 seconds. Record `STABLE` counts only after comparing them with the in-game inventory count.

## Alternatives Considered

### Reorder or simplify the existing full decoder

Checking levels before hash-table lookups would reduce work per position, but it would still visit approximately 35.7 million positions and would leave performance dependent on debug optimizer behavior.

### Search a short priority list and stop at the first large run

This follows GBFRER Helper more closely and can return early, but it can miss inventories without those priority sigils and cannot prove that a second qualifying run is absent.

### Find a candidate, then run the old exhaustive decoder for confirmation

Both paths use the same pinned catalog. Repeating the exhaustive decode adds no independent correctness evidence and recreates the 60-second failure.

### Remove all runtime limits

An unbounded scan can appear hung after an environmental or algorithmic regression. A single 10-second deadline gives the requested responsiveness while preserving an explicit failure state.

## Out of Scope

- Disabling the existing injected gameplay hook; that remains the next stage after scanner validation.
- Displaying the owned inventory in the product UI.
- Changing the equipped-sigil analysis presentation.
- Caching addresses across game processes or restarts.
- Administrator elevation, process writes, code patches, remote allocation, or injected capture hooks.
- Claiming full game 2.0.2 inventory compatibility before the manual checklist passes.

## Success Criteria

- The scanner searches all enumerated readable private regions using known non-empty sigil patterns.
- Complete record decoding is restricted to discovered candidate runs.
- Every discovered qualifying run is validated before outcome classification.
- The 16 GiB and 60-second limits are gone; one 10-second deadline is enforced.
- Existing read-only security and fail-closed result semantics remain intact.
- Automated tests pass.
- The actual-game capture reaches `STABLE` within 10 seconds; any other terminal status returns to root-cause investigation instead of completing this stage.

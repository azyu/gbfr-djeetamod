# Game 2.0.2 inventory probe validation

This checklist records manual validation of the debug-only, read-only sigil inventory probe.

- Supported executable SHA-256: `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F`
- Required application execution level: `asInvoker`
- Run the application as the current standard user. Do not elevate it to administrator.
- Enable the control only in a debug build with `DJEETA_INVENTORY_PROBE=1`.
- Open the in-game sigil inventory before selecting **Capture owned sigils**.

Do not record memory addresses, raw bytes, player names, or full sigil lists in this document. The digest is the 16-character prefix emitted by the development log.

| Check | Procedure | PID | Candidate records | Occupied | Digest | Game UI count | Result |
| --- | --- | --- | ---: | ---: | --- | ---: | --- |
| [ ] Baseline count | Open the unfiltered sigil inventory, capture once, and compare the candidate counts with the in-game count. |  |  |  |  |  |  |
| [ ] Sort/filter stability | Change only the inventory sort and filter settings, capture each view, and confirm the digest and counts stay stable. |  |  |  |  |  |  |
| [ ] Inventory mutation | Acquire, remove, or otherwise change one owned sigil, reopen the inventory, and confirm the count or digest changes as expected. |  |  |  |  |  |  |
| [ ] Process restart 1 | Fully exit the game and app, restart both, then repeat the baseline capture. |  |  |  |  |  |  |
| [ ] Process restart 2 | Fully exit the game and app, restart both, then repeat the baseline capture. |  |  |  |  |  |  |
| [ ] Process restart 3 | Fully exit the game and app, restart both, then repeat the baseline capture. |  |  |  |  |  |  |
| [ ] Meter regression | Run a battle and confirm encounter start, party totals, DPS, and reward-boundary clearing still behave normally. |  |  |  |  |  |  |
| [ ] Equipped-sigil regression | Change an equipped sigil and confirm the existing Equipment Analysis view still updates its equipped traits correctly. |  |  |  |  |  |  |
| [ ] Disabled debug run | Start a debug build without `DJEETA_INVENTORY_PROBE=1` and confirm the capture control is absent and no inventory process scan occurs. |  |  |  |  |  |  |
| [ ] Release rejection | Start a release build with `DJEETA_INVENTORY_PROBE=1` and confirm the capture control is absent and the backend rejects capture before process access. |  |  |  |  |  |  |

Unchecked rows do not establish full inventory compatibility. Do not claim Granblue Fantasy: Relink Endless Ragnarok 2.0.2 full-inventory support until every row is completed with evidence.

## Game 2.0.4 migration observation

- Executable SHA-256:
  `F827F3C13CAA90B290FAB2FE7E28165A80448FDE0A3F7A96D79DAC6B8343FF2A`
- Session: user-confirmed offline
- Process rights:
  `PROCESS_QUERY_INFORMATION | PROCESS_VM_READ`
- Stable baseline: `2,192` candidate and occupied records with digest
  `83cbf2aab841e127`
- Sort/filter control: the same `2,192` records and digest remained stable
- Controlled mutation: the game UI and probe both changed by `+4`, producing
  `2,196` records and digest `eaa851a8457fd59d`
- Restart coverage: the logical inventory was relocated across three game
  process lifetimes

The scanner initially exceeded its ten-second safety deadline after the
2.0.4 private-region size changed. Restricting structural validation to the
bounded large-region candidates restored a stable result without adding write
rights. This is a read-only inventory-probe `PASS`, not full inventory or game
compatibility.

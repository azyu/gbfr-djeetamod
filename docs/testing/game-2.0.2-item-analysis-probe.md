# Game 2.0.2 general-item analysis probe validation

- Supported executable SHA-256: `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F`
- Required process rights: `PROCESS_QUERY_INFORMATION | PROCESS_VM_READ`
- Session: offline or private
- Do not record absolute addresses, raw bytes, player names, save contents, or a full inventory.

| Check | Expected evidence | Result |
| --- | --- | --- |
| Baseline | Three known item IDs and quantities match the game UI. | PASS — 12 visible ordinary-item quantities matched, including 궁극의 증표 918. |
| Controlled +1 | Only the chosen item's decoded quantity changes by one. | |
| Controlled decrease | Only the chosen item's decoded quantity decreases by the chosen amount. | |
| Boundary 899 | The item is decoded but excluded from warnings. | |
| Boundary 900 | The item is decoded and included in warnings. | |
| Boundary 999 | The item is decoded and included in warnings. | |
| Sort/filter stability | In-game item-menu presentation changes do not change the snapshot digest. | PARTIAL — repeated full-region snapshots and the app refresh were stable; explicit in-game sort/filter changes remain. |
| Restart 1 | Locator resolves the same logical inventory after restart. | |
| Restart 2 | Locator resolves the same logical inventory after restart. | |
| Restart 3 | Locator resolves the same logical inventory after restart. | |
| Read-only access | Process access contains no write or operation right. | PASS — the probe and production reader request only query-information and VM-read access. |

## Candidate layout

The following layout is sufficient for the read-only feature build, but remains provisional
until every controlled comparison above agrees. Do not record a reusable absolute address.

- Locator: exactly one committed private region of 243,269,632 bytes, validated from two
  equal logical snapshots before publishing results.
- Record stride: `0x30`.
- Fields: item ID at `+0x00`, quantity at `+0x04`.
- Structural signature: `+0x08 = 0x0c`, `+0x10 = 0`,
  `+0x14/+0x18/+0x1c = 0xffffffff`.
- Empty and unrelated records are ignored unless the ID is in the version-pinned ordinary
  item catalog. Catalog records above 999 are treated as higher-cap currencies and excluded.
- Live app result: `궁극의 증표 918 / 999` appeared on initial page load and remained the
  same after manual refresh.

## Remaining tasks

- [x] Regenerate the Korean and English item-name catalogs from verified 2.0.2 assets
  (384 named rows per language; 64 unnamed rows excluded).
- [x] Verify all 281 ordinary-item IDs are displayable: 278 localized names and the
  existing ID fallback for the three official unnamed records.
- [x] Verify the notification setting defaults off and requires Windows notification permission.
- [x] Verify automated snapshot comparison, 900-inclusive boundaries, five-second debounce,
  grouping, and failure handling.
- [ ] In an offline or private session, enable the setting and verify one post-battle Windows
  notification for an ordinary item that increases to at least 900.
- [ ] Verify multiple qualifying post-battle item gains are grouped into one Windows notification.
- [ ] Verify no notification appears for an unchanged/decreased item or while the setting is off.
- [ ] Verify the notification remains visible through Windows Notification Center when the
  management window is hidden.
- [ ] Verify a controlled `+1` quantity change for one ordinary item.
- [ ] Verify a controlled quantity decrease for one ordinary item.
- [ ] Verify the 899, 900, and 999 warning boundaries in game.
- [ ] Verify snapshot stability after changing the in-game item-menu sort and filter.
- [ ] Restart validation 1/3: resolve the same logical inventory.
- [ ] Restart validation 2/3: resolve the same logical inventory.
- [ ] Restart validation 3/3: resolve the same logical inventory.
- [ ] After the manual evidence is complete, rerun the required frontend and Rust
  verification suite and record the final result.

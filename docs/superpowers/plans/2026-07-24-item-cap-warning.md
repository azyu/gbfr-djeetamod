# Item Cap Warning Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an `아이템 분석` management page that reads the verified 2.0.2 general-item inventory on demand and lists every item with quantity 900 through 999.

**Architecture:** First establish the general-item record and locator contract in a controlled offline/private live session; no guessed offsets may enter product code. Put byte decoding in `equipment-core`, keep read-only process discovery and snapshot stabilization in a focused Tauri module, and expose one request/response command to a standalone React page.

**Tech Stack:** Rust 2021, Tauri 1, Windows `ReadProcessMemory`, React 18, TypeScript, Mantine 7, i18next, Vitest, Cargo test.

## Global Constraints

- Target only Granblue Fantasy: Relink Endless Ragnarok 2.0.2 on Windows x64 with executable SHA-256 `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F`.
- Request only `PROCESS_QUERY_INFORMATION | PROCESS_VM_READ`; never write to game memory.
- Include every ordinary stackable item whose game cap is 999; exclude sigils, weapons, wrightstones, summons, and other individually represented assets.
- Use fixed warning threshold `900` and fixed maximum `999`; do not add settings.
- Read once when the page mounts and again only when the user presses `새로고침`; do not poll.
- Keep the previous successful result visible if a refresh fails.
- Never treat game-not-running, unsupported, unavailable, unstable, busy, or internal failures as an empty successful result.
- Do not log raw memory, reusable absolute addresses, player names, or a complete inventory.
- Do not alter protocol variants, hook behavior, meter connection state, equipment state, encounter parsing, or `logs.db`.
- Keep the feature unavailable until the controlled live layout checks in Task 1 pass.
- Execute on the current branch/worktree unless the user explicitly requests another worktree or subagents.

## File Map

- Create `docs/testing/game-2.0.2-item-analysis-probe.md`: live evidence and release gate for the general-item layout.
- Create `equipment-core/src/item_inventory.rs`: pure ordinary-item record decoding, duplicate detection, and threshold filtering.
- Modify `equipment-core/src/lib.rs`: export the item-inventory decoder.
- Create `src-tauri/src/item_analysis.rs`: read-only locator, stable snapshot read, Tauri response/error contract, and command state.
- Modify `src-tauri/src/main.rs`: register item-analysis state and command.
- Create `src/itemAnalysisContract.ts`: validate and normalize the untrusted Tauri response.
- Create `src/pages/ItemAnalysis.tsx`: mount-time load, manual refresh, stale-result retention, and table/empty/error rendering.
- Create `src/pages/ItemAnalysis.test.tsx`: frontend behavior and rendering regression coverage.
- Create `src/itemAnalysisContract.test.ts`: response normalization coverage.
- Modify `src/types.ts`: frontend item-analysis types.
- Modify `src/App.tsx`: add `/logs/items`.
- Modify `src/pages/Logs.tsx`: add the sidebar navigation entry.
- Modify `src/pages/Logs.test.tsx` and `src/pages/Logs.repeatQuest.test.tsx`: navigation regression coverage.
- Modify `src-tauri/lang/ko/ui.json` and `src-tauri/lang/en/ui.json`: page, button, empty-state, and bounded error copy.
- Create `src/pages/ItemAnalysis.localization.test.ts`: require matching Korean and English keys.
- Modify `src/securityConfiguration.test.ts`: retain read-only process-access and logging guards.

---

### Task 1: Prove the 2.0.2 General-Item Layout

**Files:**
- Create: `docs/testing/game-2.0.2-item-analysis-probe.md`

**Interfaces:**
- Consumes: pinned 2.0.2 executable, bundled `src-tauri/lang/en/items.json` item hashes, offline/private game session.
- Produces: one verified locator recipe, `ITEM_RECORD_BYTES`, `ITEM_ID_OFFSET`, `ITEM_QUANTITY_OFFSET`, occupied/empty record rules, record-count or end-of-array rule, and restart-stability evidence used by Tasks 2 and 3.

- [ ] **Step 1: Create the live-validation checklist**

Write the document with these exact required rows:

```markdown
# Game 2.0.2 general-item analysis probe validation

- Supported executable SHA-256: `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F`
- Required process rights: `PROCESS_QUERY_INFORMATION | PROCESS_VM_READ`
- Session: offline or private
- Do not record absolute addresses, raw bytes, player names, save contents, or a full inventory.

| Check | Expected evidence | Result |
| --- | --- | --- |
| Baseline | Three known item IDs and quantities match the game UI. | |
| Controlled +1 | Only the chosen item's decoded quantity changes by one. | |
| Controlled decrease | Only the chosen item's decoded quantity decreases by the chosen amount. | |
| Boundary 899 | The item is decoded but excluded from warnings. | |
| Boundary 900 | The item is decoded and included in warnings. | |
| Boundary 999 | The item is decoded and included in warnings. | |
| Sort/filter stability | In-game item-menu presentation changes do not change the snapshot digest. | |
| Restart 1 | Locator resolves the same logical inventory after restart. | |
| Restart 2 | Locator resolves the same logical inventory after restart. | |
| Restart 3 | Locator resolves the same logical inventory after restart. | |
| Read-only access | Process access contains no write or operation right. | |

## Verified layout

Record the executable-relative locator or signature recipe, record stride, field offsets,
empty-record rule, and array extent only after every controlled comparison agrees.
Do not record a reusable absolute address.
```

- [ ] **Step 2: Run the controlled live probe**

Use the `gbfr-live-probe-validation` skill. Pin the exact game PID and executable hash, choose three ordinary items with distinct known quantities, and use one-variable changes. Do not proceed if the reader cannot distinguish the item ID field from the quantity field.

Expected: every checklist row through the three restarts records `MATCH`, and the verified-layout section contains no absolute address.

- [ ] **Step 3: Confirm the catalog scope**

Compare occupied IDs from the verified snapshot with keys in `src-tauri/lang/en/items.json`.

Expected: every occupied ordinary-item ID is known or is explicitly classified as a valid unknown ID; sigil hashes and other individually represented assets are absent. If this fails, stop and revise the design instead of broadening the decoder heuristics.

- [ ] **Step 4: Commit the evidence gate**

```powershell
git add -- docs/testing/game-2.0.2-item-analysis-probe.md
git commit -m "docs: validate general item inventory layout"
```

Expected: the commit contains only the checklist and non-sensitive layout evidence.

---

### Task 2: Add the Pure General-Item Decoder

**Files:**
- Create: `equipment-core/src/item_inventory.rs`
- Modify: `equipment-core/src/lib.rs`
- Test: `equipment-core/src/item_inventory.rs`

**Interfaces:**
- Consumes: exact layout constants proven in Task 1 and `HashSet<u32>` of ordinary-item IDs.
- Produces:
  - `pub const ITEM_WARNING_THRESHOLD: u32 = 900`
  - `pub const ITEM_MAX_QUANTITY: u32 = 999`
  - `pub struct OwnedItem { pub item_id: u32, pub quantity: u32 }`
  - `pub fn decode_item_inventory(bytes: &[u8], known_ids: &HashSet<u32>) -> Result<Vec<OwnedItem>, ItemInventoryDecodeError>`
  - `pub fn warning_items(items: &[OwnedItem]) -> Vec<OwnedItem>`

- [ ] **Step 1: Write failing decoder tests**

Add tests using a fixture builder driven by the verified offsets:

```rust
fn record(item_id: u32, quantity: u32) -> Vec<u8> {
    let mut bytes = vec![0; ITEM_RECORD_BYTES];
    put_u32(&mut bytes, ITEM_ID_OFFSET, item_id);
    put_u32(&mut bytes, ITEM_QUANTITY_OFFSET, quantity);
    bytes
}

#[test]
fn decodes_known_items_and_ignores_verified_empty_records() {
    let known = HashSet::from([0x01EA_A064, 0xDF71_3FA9]);
    let mut bytes = empty_record();
    bytes.extend(record(0x01EA_A064, 899));
    bytes.extend(record(0xDF71_3FA9, 900));

    assert_eq!(
        decode_item_inventory(&bytes, &known).unwrap(),
        vec![
            OwnedItem { item_id: 0x01EA_A064, quantity: 899 },
            OwnedItem { item_id: 0xDF71_3FA9, quantity: 900 },
        ]
    );
}

#[test]
fn filters_inclusive_warning_boundaries() {
    let items = vec![
        OwnedItem { item_id: 1, quantity: 899 },
        OwnedItem { item_id: 2, quantity: 900 },
        OwnedItem { item_id: 3, quantity: 999 },
    ];
    assert_eq!(warning_items(&items), items[1..]);
}

#[test]
fn rejects_unknown_ids_invalid_quantities_duplicates_and_partial_records() {
    let known = HashSet::from([0x01EA_A064]);
    assert_eq!(
        decode_item_inventory(&record(0xDF71_3FA9, 1), &known),
        Err(ItemInventoryDecodeError::UnknownItem(0xDF71_3FA9))
    );
    assert_eq!(
        decode_item_inventory(&record(0x01EA_A064, 1000), &known),
        Err(ItemInventoryDecodeError::InvalidQuantity {
            item_id: 0x01EA_A064,
            quantity: 1000,
        })
    );
    let duplicate = [record(0x01EA_A064, 1), record(0x01EA_A064, 2)].concat();
    assert_eq!(
        decode_item_inventory(&duplicate, &known),
        Err(ItemInventoryDecodeError::DuplicateItem(0x01EA_A064))
    );
    assert_eq!(
        decode_item_inventory(&vec![0; ITEM_RECORD_BYTES - 1], &known),
        Err(ItemInventoryDecodeError::PartialRecord)
    );
}
```

- [ ] **Step 2: Run the focused tests to verify RED**

Run:

```powershell
cargo test --package equipment-core item_inventory --locked
```

Expected: compilation fails because `item_inventory` and its public interfaces do not exist.

- [ ] **Step 3: Implement the minimal decoder**

Create `equipment-core/src/item_inventory.rs` with the exact constants from Task 1 and this contract:

```rust
use std::collections::{HashMap, HashSet};
use thiserror::Error;

pub const ITEM_WARNING_THRESHOLD: u32 = 900;
pub const ITEM_MAX_QUANTITY: u32 = 999;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnedItem {
    pub item_id: u32,
    pub quantity: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ItemInventoryDecodeError {
    #[error("item inventory ends with a partial record")]
    PartialRecord,
    #[error("unknown ordinary item id {0:#010x}")]
    UnknownItem(u32),
    #[error("item {item_id:#010x} has invalid quantity {quantity}")]
    InvalidQuantity { item_id: u32, quantity: u32 },
    #[error("item {0:#010x} appears more than once")]
    DuplicateItem(u32),
}

pub fn decode_item_inventory(
    bytes: &[u8],
    known_ids: &HashSet<u32>,
) -> Result<Vec<OwnedItem>, ItemInventoryDecodeError> {
    if bytes.len() % ITEM_RECORD_BYTES != 0 {
        return Err(ItemInventoryDecodeError::PartialRecord);
    }

    let mut items = HashMap::new();
    for record in bytes.chunks_exact(ITEM_RECORD_BYTES) {
        let item_id = read_u32(record, ITEM_ID_OFFSET);
        let quantity = read_u32(record, ITEM_QUANTITY_OFFSET);
        if is_verified_empty_record(item_id, quantity) {
            continue;
        }
        if !known_ids.contains(&item_id) {
            return Err(ItemInventoryDecodeError::UnknownItem(item_id));
        }
        if quantity > ITEM_MAX_QUANTITY {
            return Err(ItemInventoryDecodeError::InvalidQuantity { item_id, quantity });
        }
        if items.insert(item_id, OwnedItem { item_id, quantity }).is_some() {
            return Err(ItemInventoryDecodeError::DuplicateItem(item_id));
        }
    }
    let mut items = items.into_values().collect::<Vec<_>>();
    items.sort_unstable_by_key(|item| item.item_id);
    Ok(items)
}

pub fn warning_items(items: &[OwnedItem]) -> Vec<OwnedItem> {
    items
        .iter()
        .copied()
        .filter(|item| item.quantity >= ITEM_WARNING_THRESHOLD)
        .collect()
}
```

Add `serde = { version = "1", features = ["derive"] }` to `equipment-core/Cargo.toml`. Implement `ITEM_RECORD_BYTES`, `ITEM_ID_OFFSET`, `ITEM_QUANTITY_OFFSET`, `read_u32`, and `is_verified_empty_record` exactly from Task 1.

Export the module from `equipment-core/src/lib.rs`:

```rust
mod item_inventory;
pub use item_inventory::*;
```

- [ ] **Step 4: Run decoder tests to verify GREEN**

Run:

```powershell
cargo test --package equipment-core item_inventory --locked
```

Expected: all item-inventory decoder and boundary tests pass.

- [ ] **Step 5: Commit the decoder**

```powershell
git add -- equipment-core/Cargo.toml equipment-core/src/lib.rs equipment-core/src/item_inventory.rs Cargo.lock
git commit -m "feat: decode ordinary item inventory"
```

Expected: only the pure decoder, tests, exports, and necessary lockfile change are committed.

---

### Task 3: Add the Read-Only Tauri Item Analyzer

**Files:**
- Create: `src-tauri/src/item_analysis.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src/securityConfiguration.test.ts`
- Test: `src-tauri/src/item_analysis.rs`

**Interfaces:**
- Consumes: `RemoteProcess`, `MemoryReader`, pinned executable hash, Task 1 locator recipe, `decode_item_inventory`, and `warning_items`.
- Produces:
  - `pub(crate) struct ItemAnalysisState`
  - `#[tauri::command] pub(crate) async fn fetch_item_analysis(...) -> Result<ItemAnalysisResponse, String>`
  - camelCase JSON `{ inspectedAtMs, threshold, maximum, items: [{ itemId, quantity }] }`
  - bounded codes `ALREADY_RUNNING`, `GAME_NOT_RUNNING`, `UNSUPPORTED_GAME`, `UNAVAILABLE`, `UNSTABLE`, `INTERNAL`

- [ ] **Step 1: Write failing backend tests**

Cover the locator and stable-read decision with a fake `MemoryReader`:

```rust
#[test]
fn returns_only_warning_items_from_two_equal_reads() {
    let memory = FakeMemory::with_repeated_snapshot(fixture(&[(ITEM_A, 899), (ITEM_B, 900)]));
    let response = analyze_with_reader(&memory, verified_locator(), &catalog()).unwrap();
    assert_eq!(response.threshold, 900);
    assert_eq!(response.maximum, 999);
    assert_eq!(response.items, vec![OwnedItem { item_id: ITEM_B, quantity: 900 }]);
}

#[test]
fn rejects_changed_second_read_and_duplicate_ids() {
    assert_eq!(
        analyze_with_reader(&changing_memory(), verified_locator(), &catalog()),
        Err(ItemAnalysisCode::Unstable)
    );
    assert_eq!(
        analyze_with_reader(&duplicate_memory(), verified_locator(), &catalog()),
        Err(ItemAnalysisCode::Unavailable)
    );
}

#[test]
fn state_rejects_overlapping_requests() {
    let state = ItemAnalysisState::default();
    let _guard = state.try_begin().unwrap();
    assert_eq!(state.try_begin().unwrap_err(), ItemAnalysisCode::AlreadyRunning);
}
```

Extend `src/securityConfiguration.test.ts`:

```ts
test("item analysis requests read-only process access and does not log inventory contents", () => {
  const memory = readRepositoryFile("src-tauri/src/equipment_probe/memory.rs");
  const source = readRepositoryFile("src-tauri/src/item_analysis.rs");
  expect(memory).toContain("PROCESS_QUERY_INFORMATION | PROCESS_VM_READ");
  for (const forbidden of ["PROCESS_VM_WRITE", "PROCESS_VM_OPERATION", "WriteProcessMemory"]) {
    expect(memory + source).not.toContain(forbidden);
  }
  expect(source).not.toMatch(/log::\\w+!\\([^)]*(raw|address|items|inventory_json)/i);
});
```

- [ ] **Step 2: Run focused backend tests to verify RED**

Run:

```powershell
cargo test --package gbfr-logs item_analysis --locked
npm.cmd test -- --run src/securityConfiguration.test.ts
```

Expected: Rust fails because `item_analysis` does not exist, and the frontend security test fails because the source file is absent.

- [ ] **Step 3: Implement state, locator, stable read, and response**

Create `src-tauri/src/item_analysis.rs` around these exact public contracts:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemAnalysisCode {
    AlreadyRunning,
    GameNotRunning,
    UnsupportedGame,
    Unavailable,
    Unstable,
    Internal,
}

impl ItemAnalysisCode {
    fn as_str(self) -> &'static str {
        match self {
            Self::AlreadyRunning => "ALREADY_RUNNING",
            Self::GameNotRunning => "GAME_NOT_RUNNING",
            Self::UnsupportedGame => "UNSUPPORTED_GAME",
            Self::Unavailable => "UNAVAILABLE",
            Self::Unstable => "UNSTABLE",
            Self::Internal => "INTERNAL",
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct ItemAnalysisState {
    running: Arc<AtomicBool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ItemAnalysisResponse {
    inspected_at_ms: u64,
    threshold: u32,
    maximum: u32,
    items: Vec<OwnedItem>,
}
```

Implementation order inside the blocking worker:

1. Acquire the single-run guard.
2. Find exact `Granblue_Fantasy_Relink.exe`.
3. Compute and compare the pinned SHA-256 before interpreting memory.
4. Load known ordinary-item IDs from `include_str!("../lang/en/items.json")`.
5. Resolve the exact locator/signature recipe proved in Task 1.
6. Read the bounded inventory snapshot.
7. Wait 50 ms and read the same logical snapshot again.
8. Require byte-for-byte equality.
9. Decode once and return only `warning_items`.
10. Log only status, elapsed milliseconds, occupied count, warning count, and a 16-character digest prefix.

The locator must be deterministic and bounded. Encode the exact executable-relative pointer path or signature from Task 1, and add a byte-fixture test for unique, missing, and ambiguous matches. Do not fall back to an unbounded heuristic scan.

- [ ] **Step 4: Register the command**

Modify `src-tauri/src/main.rs`:

```rust
mod item_analysis;
```

Add state beside existing probe state:

```rust
.manage(item_analysis::ItemAnalysisState::default())
```

Add the command to `tauri::generate_handler!`:

```rust
item_analysis::fetch_item_analysis,
```

- [ ] **Step 5: Run focused tests to verify GREEN**

Run:

```powershell
cargo test --package gbfr-logs item_analysis --locked
npm.cmd test -- --run src/securityConfiguration.test.ts
```

Expected: stable-read, failure-code, concurrency, locator, and read-only security tests pass.

- [ ] **Step 6: Commit the backend**

```powershell
git add -- src-tauri/src/item_analysis.rs src-tauri/src/main.rs src/securityConfiguration.test.ts
git commit -m "feat: expose read-only item analysis"
```

---

### Task 4: Add the Item Analysis Contract and Page

**Files:**
- Create: `src/itemAnalysisContract.ts`
- Create: `src/itemAnalysisContract.test.ts`
- Create: `src/pages/ItemAnalysis.tsx`
- Create: `src/pages/ItemAnalysis.test.tsx`
- Modify: `src/types.ts`

**Interfaces:**
- Consumes: Tauri command `fetch_item_analysis`, `translateItemId`, and i18n keys under `ui.item-analysis`.
- Produces: `ItemAnalysisResponse`, `ItemAnalysisEntry`, normalized/sorted render data, mount-time request, and manual refresh.

- [ ] **Step 1: Write failing contract tests**

```ts
it("keeps valid warning entries and rejects malformed entries", () => {
  expect(
    normalizeItemAnalysisResponse({
      inspectedAtMs: 123,
      threshold: 900,
      maximum: 999,
      items: [
        { itemId: 0x01eaa064, quantity: 999 },
        { itemId: -1, quantity: 999 },
        { itemId: 0xdf713fa9, quantity: 899 },
      ],
    })
  ).toEqual({
    inspectedAtMs: 123,
    threshold: 900,
    maximum: 999,
    items: [{ itemId: 0x01eaa064, quantity: 999 }],
  });
});

it("rejects a malformed envelope instead of treating it as an empty success", () => {
  expect(() => normalizeItemAnalysisResponse({ items: [] })).toThrow("invalid item analysis response");
});
```

- [ ] **Step 2: Run contract tests to verify RED**

Run:

```powershell
npm.cmd test -- --run src/itemAnalysisContract.test.ts
```

Expected: FAIL because the module and types do not exist.

- [ ] **Step 3: Add types and strict normalization**

Append to `src/types.ts`:

```ts
export type ItemAnalysisEntry = {
  itemId: number;
  quantity: number;
};

export type ItemAnalysisResponse = {
  inspectedAtMs: number;
  threshold: 900;
  maximum: 999;
  items: ItemAnalysisEntry[];
};
```

Implement `normalizeItemAnalysisResponse(value: unknown)` so the envelope must be valid, item IDs are unsigned 32-bit integers, quantities are integers from 900 through 999, duplicate IDs throw, and malformed rows are discarded only after a valid envelope is established.

- [ ] **Step 4: Run contract tests to verify GREEN**

Run:

```powershell
npm.cmd test -- --run src/itemAnalysisContract.test.ts
```

Expected: both normalization tests pass.

- [ ] **Step 5: Write failing page behavior tests**

Mock `invoke` and `translateItemId`, then cover:

```ts
it("loads once on mount and sorts quantity descending then translated name", async () => {
  renderPage();
  await screen.findByText("999 / 999");
  expect(invoke).toHaveBeenCalledTimes(1);
  expect(invoke).toHaveBeenCalledWith("fetch_item_analysis");
  expect(screen.getAllByTestId("item-row").map((row) => row.textContent)).toEqual([
    "궁극의 기억999 / 999",
    "반신의 날개900 / 999",
  ]);
});

it("does not overlap refresh and preserves stale data after failure", async () => {
  renderPage();
  await screen.findByText("999 / 999");
  fireEvent.click(screen.getByRole("button", { name: "새로고침" }));
  fireEvent.click(screen.getByRole("button", { name: "새로고침" }));
  expect(invoke).toHaveBeenCalledTimes(2);
  rejectRefresh("UNSTABLE");
  expect(await screen.findByText("아이템 정보가 변경되어 다시 읽어야 합니다.")).toBeTruthy();
  expect(screen.getByText("999 / 999")).toBeTruthy();
});

it("distinguishes a successful empty result from command errors", async () => {
  mocks.responses = [
    { inspectedAtMs: 123, threshold: 900, maximum: 999, items: [] },
  ];
  const empty = renderPage();
  expect(await screen.findByText("보유 한도에 가까운 아이템이 없습니다.")).toBeTruthy();
  empty.unmount();

  mocks.responses = [Promise.reject("GAME_NOT_RUNNING")];
  renderPage();
  expect(await screen.findByText("게임이 실행 중이 아닙니다.")).toBeTruthy();
  expect(screen.queryByText("보유 한도에 가까운 아이템이 없습니다.")).toBeNull();
});
```

- [ ] **Step 6: Run page tests to verify RED**

Run:

```powershell
npm.cmd test -- --run src/pages/ItemAnalysis.test.tsx
```

Expected: FAIL because `ItemAnalysis` does not exist.

- [ ] **Step 7: Implement the page**

Create `src/pages/ItemAnalysis.tsx` with:

```tsx
const ERROR_CODES = new Set([
  "ALREADY_RUNNING",
  "GAME_NOT_RUNNING",
  "UNSUPPORTED_GAME",
  "UNAVAILABLE",
  "UNSTABLE",
  "INTERNAL",
]);

export const ItemAnalysis = () => {
  const { t } = useTranslation();
  const [response, setResponse] = useState<ItemAnalysisResponse | null>(null);
  const [errorCode, setErrorCode] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const runningRef = useRef(false);

  const refresh = useCallback(async () => {
    if (runningRef.current) return;
    runningRef.current = true;
    setPending(true);
    setErrorCode(null);
    try {
      const value = await invoke("fetch_item_analysis");
      setResponse(normalizeItemAnalysisResponse(value));
    } catch (error) {
      setErrorCode(typeof error === "string" && ERROR_CODES.has(error) ? error : "INTERNAL");
    } finally {
      runningRef.current = false;
      setPending(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const items = [...(response?.items ?? [])].sort(
    (left, right) =>
      right.quantity - left.quantity ||
      translateItemId(left.itemId).localeCompare(translateItemId(right.itemId))
  );

  return (
    <Stack>
      <Group justify="space-between">
        <div>
          <Title order={2}>{t("ui.item-analysis.title")}</Title>
          <Text c="dimmed">{t("ui.item-analysis.description")}</Text>
        </div>
        <Button disabled={pending} onClick={() => void refresh()}>
          {t("ui.item-analysis.refresh")}
        </Button>
      </Group>
      {errorCode && (
        <Alert role="alert" color="red">
          {t(`ui.item-analysis.error.${errorCode}`)}
        </Alert>
      )}
      {response === null && pending ? (
        <Text>{t("ui.item-analysis.loading")}</Text>
      ) : response !== null && items.length === 0 ? (
        <Text>{t("ui.item-analysis.empty")}</Text>
      ) : (
        <Table>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>{t("ui.item-analysis.title")}</Table.Th>
              <Table.Th>{t("ui.item-analysis.quantity")}</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {items.map((item) => (
              <Table.Tr key={item.itemId} data-testid="item-row">
                <Table.Td>{translateItemId(item.itemId)}</Table.Td>
                <Table.Td>{item.quantity} / 999</Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      )}
    </Stack>
  );
};
```

Render unknown translations through the existing `translateItemId` fallback, retaining the eight-digit hexadecimal ID.

- [ ] **Step 8: Run page tests to verify GREEN**

Run:

```powershell
npm.cmd test -- --run src/pages/ItemAnalysis.test.tsx src/itemAnalysisContract.test.ts
```

Expected: mount, sorting, refresh locking, stale-result retention, empty state, and bounded errors pass.

- [ ] **Step 9: Commit the frontend page**

```powershell
git add -- src/types.ts src/itemAnalysisContract.ts src/itemAnalysisContract.test.ts src/pages/ItemAnalysis.tsx src/pages/ItemAnalysis.test.tsx
git commit -m "feat: render item cap warnings"
```

---

### Task 5: Add Navigation and Localized Copy

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/pages/Logs.tsx`
- Modify: `src/pages/Logs.test.tsx`
- Modify: `src/pages/Logs.repeatQuest.test.tsx`
- Modify: `src-tauri/lang/ko/ui.json`
- Modify: `src-tauri/lang/en/ui.json`
- Create: `src/pages/ItemAnalysis.localization.test.ts`

**Interfaces:**
- Consumes: `ItemAnalysis` page.
- Produces: `/logs/items` route, `아이템 분석` sidebar entry between equipment analysis and battle records, and matching Korean/English keys.

- [ ] **Step 1: Write failing route, navigation, and localization tests**

Add assertions:

```ts
expect(screen.getByText("아이템 분석")).toBeTruthy();
expect(screen.getByText("아이템 분석").closest("a")?.getAttribute("href")).toBe("/logs/items");
```

Create a localization parity test requiring:

```ts
const REQUIRED_KEYS = [
  "title",
  "description",
  "refresh",
  "loading",
  "empty",
  "quantity",
  "error.ALREADY_RUNNING",
  "error.GAME_NOT_RUNNING",
  "error.UNSUPPORTED_GAME",
  "error.UNAVAILABLE",
  "error.UNSTABLE",
  "error.INTERNAL",
];
```

- [ ] **Step 2: Run focused tests to verify RED**

Run:

```powershell
npm.cmd test -- --run src/pages/Logs.test.tsx src/pages/Logs.repeatQuest.test.tsx src/pages/ItemAnalysis.localization.test.ts
```

Expected: failures for the missing nav entry and translation group.

- [ ] **Step 3: Add route and sidebar entry**

Modify `src/App.tsx`:

```tsx
import { ItemAnalysis } from "./pages/ItemAnalysis";
// ...
<Route path="items" element={<ItemAnalysis />} />
```

Modify `src/pages/Logs.tsx` after the equipment-analysis link:

```tsx
<NavLink
  label={t("ui.item-analysis.title")}
  leftSection={<Package size="1rem" />}
  component={Link}
  to="/logs/items"
/>
```

Import `Package` from `@phosphor-icons/react`.

- [ ] **Step 4: Add exact Korean and English copy**

Under `ui.item-analysis`, add:

```json
{
  "title": "아이템 분석",
  "description": "보유 한도 999개에 가까운 일반 아이템만 표시합니다.",
  "refresh": "새로고침",
  "loading": "아이템 정보를 확인하는 중입니다.",
  "empty": "보유 한도에 가까운 아이템이 없습니다.",
  "quantity": "수량",
  "error": {
    "ALREADY_RUNNING": "아이템 정보를 이미 확인하고 있습니다.",
    "GAME_NOT_RUNNING": "게임이 실행 중이 아닙니다.",
    "UNSUPPORTED_GAME": "지원하는 게임 2.0.2 실행 파일이 아닙니다.",
    "UNAVAILABLE": "일반 아이템 정보를 찾을 수 없습니다.",
    "UNSTABLE": "아이템 정보가 변경되어 다시 읽어야 합니다.",
    "INTERNAL": "아이템 정보를 확인하지 못했습니다."
  }
}
```

```json
{
  "title": "Item Analysis",
  "description": "Shows ordinary items approaching the 999-item holding limit.",
  "refresh": "Refresh",
  "loading": "Checking item quantities.",
  "empty": "No items are close to the holding limit.",
  "quantity": "Quantity",
  "error": {
    "ALREADY_RUNNING": "Item quantities are already being checked.",
    "GAME_NOT_RUNNING": "The game is not running.",
    "UNSUPPORTED_GAME": "This is not the supported game 2.0.2 executable.",
    "UNAVAILABLE": "The ordinary-item inventory could not be found.",
    "UNSTABLE": "Item quantities changed while being read. Refresh again.",
    "INTERNAL": "Item quantities could not be checked."
  }
}
```

- [ ] **Step 5: Run focused tests to verify GREEN**

Run:

```powershell
npm.cmd test -- --run src/pages/Logs.test.tsx src/pages/Logs.repeatQuest.test.tsx src/pages/ItemAnalysis.localization.test.ts src/pages/ItemAnalysis.test.tsx
```

Expected: route/navigation, localization parity, and page tests pass.

- [ ] **Step 6: Commit navigation and copy**

```powershell
git add -- src/App.tsx src/pages/Logs.tsx src/pages/Logs.test.tsx src/pages/Logs.repeatQuest.test.tsx src/pages/ItemAnalysis.localization.test.ts src-tauri/lang/ko/ui.json src-tauri/lang/en/ui.json
git commit -m "feat: add item analysis navigation"
```

---

### Task 6: Verify the Complete Feature

**Files:**
- Modify: `docs/testing/game-2.0.2-item-analysis-probe.md` only if final UI evidence needs recording.

**Interfaces:**
- Consumes: all previous tasks.
- Produces: fresh automated verification and final offline/private UI evidence without claiming broader compatibility.

- [ ] **Step 1: Run the narrow regressions**

```powershell
npm.cmd test -- --run src/pages/ItemAnalysis.test.tsx src/itemAnalysisContract.test.ts src/pages/Logs.test.tsx src/pages/ItemAnalysis.localization.test.ts src/securityConfiguration.test.ts
cargo test --package equipment-core item_inventory --locked
cargo test --package gbfr-logs item_analysis --locked
```

Expected: every named test passes with zero failures.

- [ ] **Step 2: Run required frontend verification**

```powershell
npm.cmd run format-check
npm.cmd run lint
npm.cmd run tsc
npm.cmd test -- --run
npm.cmd run build
```

Expected: all commands exit 0.

- [ ] **Step 3: Run required Rust verification**

Load the Visual Studio developer environment if MSVC is not already available, then run:

```powershell
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
```

Expected: release hook build and all workspace tests exit 0.

- [ ] **Step 4: Perform final offline/private UI validation**

Start the supported game and debug app as the current standard user. Open `아이템 분석` and verify:

1. The initial read occurs once.
2. Only quantities 900 through 999 appear.
3. Names agree with the game UI and rows show `현재 수량 / 999`.
4. Refresh reflects one controlled quantity change.
5. A forced unstable/unavailable run keeps the last successful rows visible and adds an error.
6. No 900+ item produces the successful empty-state copy.
7. Meter, equipment analysis, battle records, and repeat quest remain functional.

Record only pass/fail evidence in `docs/testing/game-2.0.2-item-analysis-probe.md`; do not add addresses, raw bytes, or a full inventory.

- [ ] **Step 5: Inspect the final diff**

```powershell
git status --short
git diff --check
git diff --stat
```

Expected: no whitespace errors, no `logs.db`, no unrelated user files, and every changed line traces to this feature.

- [ ] **Step 6: Commit final evidence if it changed**

```powershell
git add -- docs/testing/game-2.0.2-item-analysis-probe.md
git commit -m "test: record item analysis validation"
```

Skip this commit when the checklist already contains the final evidence and has no changes.

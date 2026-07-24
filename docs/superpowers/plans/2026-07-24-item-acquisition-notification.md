# Item Acquisition Notification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an opt-in Windows system notification that checks ordinary-item gains once after each battle and reports, in one localized notification, every increased item whose resulting quantity is at least 900.

**Architecture:** Keep the existing warning-table command unchanged and add a second read-only command that returns the complete stable ordinary-item snapshot. A persisted frontend setting and a layout-level controller will establish a runtime baseline, listen for a backend `battle-ended` event, wait five seconds for rewards, compare snapshots, and send one native Tauri notification.

**Tech Stack:** Rust, Tauri 1.6 notification API, React 18, TypeScript, Zustand persist, Mantine Tabs/Switch/Alert, i18next, Vitest, Testing Library.

## Global Constraints

- Target Granblue Fantasy: Relink Endless Ragnarok 2.0.2 on Windows x64 only.
- The notification threshold is inclusive: a resulting quantity of exactly 900 qualifies.
- The setting defaults to off and persists; runtime inventory baselines never persist.
- The feature performs one logical stable snapshot after a battle, never interval polling.
- The post-battle delay is exactly five seconds and repeated pending battle events debounce to one scan.
- A notification is sent only when `current > previous && current >= 900`.
- One battle produces at most one Windows notification; do not also show an in-app toast.
- Enabling requires notification permission. Denied or unavailable permission keeps the setting off and shows localized guidance.
- Preserve the existing `fetch_item_analysis` response and its 900–999 table behavior.
- Reuse the existing read-only process access, exact 2.0.2 executable hash, stable double-read, region validation, and shared overlap guard.
- Do not log raw inventory data, absolute addresses, item quantities, or notification bodies.
- Do not read, modify, stage, or commit `logs.db`.
- Automated tests do not establish game compatibility; leave manual notification validation unchecked until observed in an offline or private session.

---

### Task 1: Complete read-only inventory snapshot command

**Files:**
- Modify: `src-tauri/src/item_analysis.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src/types.ts`
- Modify: `src/itemAnalysisContract.ts`
- Test: `src-tauri/src/item_analysis.rs`
- Test: `src/itemAnalysisContract.test.ts`

**Interfaces:**
- Produces Rust `ItemInventorySnapshotResponse { inspected_at_ms: u64, items: Vec<OwnedItem> }`.
- Produces Tauri command `fetch_item_inventory_snapshot`.
- Produces TypeScript `ItemInventorySnapshotResponse`.
- Produces `normalizeItemInventorySnapshotResponse(value: unknown): ItemInventorySnapshotResponse`.
- Preserves `fetch_item_analysis` and `normalizeItemAnalysisResponse`.

- [ ] **Step 1: Add failing Rust tests for complete stable snapshots**

In `src-tauri/src/item_analysis.rs`, split the stable decode result from warning
filtering in the tests first:

```rust
#[test]
fn full_snapshot_keeps_items_below_the_warning_threshold() {
    let ids = HashSet::from([0x1111_1111, 0x2222_2222]);
    let bytes = [
        record(0x1111_1111, ITEM_WARNING_THRESHOLD - 1),
        record(0x2222_2222, ITEM_WARNING_THRESHOLD),
    ]
    .concat();

    let snapshot = stable_inventory_snapshot(&bytes, &bytes, &ids).unwrap();

    assert_eq!(
        snapshot.items,
        vec![
            OwnedItem {
                item_id: 0x2222_2222,
                quantity: ITEM_WARNING_THRESHOLD,
            },
            OwnedItem {
                item_id: 0x1111_1111,
                quantity: ITEM_WARNING_THRESHOLD - 1,
            },
        ]
    );
}

#[test]
fn warning_response_still_filters_the_complete_snapshot() {
    let snapshot = ItemInventorySnapshotResponse {
        inspected_at_ms: 123,
        items: vec![
            OwnedItem {
                item_id: 1,
                quantity: ITEM_WARNING_THRESHOLD - 1,
            },
            OwnedItem {
                item_id: 2,
                quantity: ITEM_WARNING_THRESHOLD,
            },
        ],
    };

    let response = ItemAnalysisResponse::from(snapshot);

    assert_eq!(response.items, vec![OwnedItem { item_id: 2, quantity: 900 }]);
    assert_eq!(response.threshold, ITEM_WARNING_THRESHOLD);
    assert_eq!(response.maximum, ITEM_MAX_QUANTITY);
}
```

Retain the existing changed-second-snapshot and region-selection tests.

- [ ] **Step 2: Run the Rust tests to verify RED**

Run:

```powershell
cargo test --locked --package gbfr-logs item_analysis::tests
```

Expected: compilation fails because `stable_inventory_snapshot`,
`ItemInventorySnapshotResponse`, and the conversion do not exist.

- [ ] **Step 3: Implement the complete snapshot without duplicating the scan**

In `src-tauri/src/item_analysis.rs`, add:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ItemInventorySnapshotResponse {
    inspected_at_ms: u64,
    items: Vec<OwnedItem>,
}

fn stable_inventory_snapshot(
    first: &[u8],
    second: &[u8],
    ordinary_item_ids: &HashSet<u32>,
) -> Result<ItemInventorySnapshotResponse, ItemAnalysisCode> {
    let first = decode_snapshot(first, ordinary_item_ids)?;
    let second = decode_snapshot(second, ordinary_item_ids)?;
    if first != second {
        return Err(ItemAnalysisCode::Unstable);
    }
    let inspected_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ItemAnalysisCode::Internal)?
        .as_millis()
        .try_into()
        .map_err(|_| ItemAnalysisCode::Internal)?;
    Ok(ItemInventorySnapshotResponse {
        inspected_at_ms,
        items: first,
    })
}

impl From<ItemInventorySnapshotResponse> for ItemAnalysisResponse {
    fn from(snapshot: ItemInventorySnapshotResponse) -> Self {
        Self {
            inspected_at_ms: snapshot.inspected_at_ms,
            threshold: ITEM_WARNING_THRESHOLD,
            maximum: ITEM_MAX_QUANTITY,
            items: warning_items(&snapshot.items),
        }
    }
}
```

Refactor `analyze_process` to return `ItemInventorySnapshotResponse` and keep
all current validation and the two reads. Keep the log limited to PID, elapsed
time, and decoded item count; do not serialize inventory contents.

Add a shared async runner so both commands use the same guard:

```rust
async fn fetch_snapshot(
    state: State<'_, ItemAnalysisState>,
) -> Result<ItemInventorySnapshotResponse, String> {
    let _guard = state.try_begin().map_err(|code| code.as_str().to_owned())?;
    tauri::async_runtime::spawn_blocking(analyze_process)
        .await
        .map_err(|_| ItemAnalysisCode::Internal.as_str().to_owned())?
        .map_err(|code| code.as_str().to_owned())
}

#[tauri::command]
pub(crate) async fn fetch_item_inventory_snapshot(
    state: State<'_, ItemAnalysisState>,
) -> Result<ItemInventorySnapshotResponse, String> {
    fetch_snapshot(state).await
}

#[tauri::command]
pub(crate) async fn fetch_item_analysis(
    state: State<'_, ItemAnalysisState>,
) -> Result<ItemAnalysisResponse, String> {
    fetch_snapshot(state).await.map(Into::into)
}
```

Register `item_analysis::fetch_item_inventory_snapshot` next to the existing
command in `src-tauri/src/main.rs`.

- [ ] **Step 4: Add the frontend contract test and implementation**

In `src/types.ts`, add:

```ts
export type ItemInventorySnapshotResponse = {
  inspectedAtMs: number;
  items: ItemAnalysisEntry[];
};
```

In `src/itemAnalysisContract.test.ts`, add:

```ts
it("normalizes a complete item inventory snapshot", () => {
  expect(
    normalizeItemInventorySnapshotResponse({
      inspectedAtMs: 456,
      items: [
        { itemId: 0x11111111, quantity: 899 },
        { itemId: 0x22222222, quantity: 900 },
      ],
    })
  ).toEqual({
    inspectedAtMs: 456,
    items: [
      { itemId: 0x11111111, quantity: 899 },
      { itemId: 0x22222222, quantity: 900 },
    ],
  });
});
```

Run:

```powershell
npm.cmd test -- --run src/itemAnalysisContract.test.ts
```

Expected: FAIL because the normalizer is missing.

In `src/itemAnalysisContract.ts`, add
`normalizeItemInventorySnapshotResponse`. Reuse the existing finite integer,
item ID, and 0–999 quantity checks rather than copying a second validator.
Reject malformed top-level values and drop malformed item rows in the same way
as the existing response normalizer.

- [ ] **Step 5: Verify Task 1**

Run:

```powershell
cargo test --locked --package gbfr-logs item_analysis::tests
npm.cmd test -- --run src/itemAnalysisContract.test.ts
```

Expected: both commands PASS; the Rust tests prove below-threshold items remain
in the complete snapshot and the old response remains filtered.

- [ ] **Step 6: Commit Task 1**

```powershell
git add -- src-tauri/src/item_analysis.rs src-tauri/src/main.rs src/types.ts src/itemAnalysisContract.ts src/itemAnalysisContract.test.ts
git commit -m "feat: expose stable item inventory snapshots"
```

---

### Task 2: Native notification capability and persisted setting

**Files:**
- Create: `src/stores/useItemNotificationStore.ts`
- Create: `src/stores/useItemNotificationStore.test.ts`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/tauri.conf.json`
- Test: `src/securityConfiguration.test.ts`

**Interfaces:**
- Produces `useItemNotificationStore`.
- Store fields: `enabled: boolean`, `permissionDenied: boolean`.
- Store actions: `setEnabled(enabled: boolean)`, `setPermissionDenied(denied: boolean)`.
- Enables Tauri frontend APIs `isPermissionGranted`, `requestPermission`, and `sendNotification`.

- [ ] **Step 1: Add failing security configuration assertions**

In `src/securityConfiguration.test.ts`, add:

```ts
it("enables only the native notification capability required by item alerts", () => {
  const config = JSON.parse(readRepositoryFile("src-tauri/tauri.conf.json")) as {
    tauri: {
      allowlist: {
        all: boolean;
        notification?: { all?: boolean };
      };
    };
  };
  const cargo = readRepositoryFile("src-tauri/Cargo.toml");

  expect(config.tauri.allowlist.all).toBe(false);
  expect(config.tauri.allowlist.notification).toEqual({ all: true });
  expect(cargo).toMatch(/tauri = \{[^\n]*features = \[[^\]]*"notification-all"/);
});
```

Run:

```powershell
npm.cmd test -- --run src/securityConfiguration.test.ts
```

Expected: FAIL because the notification capability is not configured.

- [ ] **Step 2: Enable the minimal Tauri notification API**

In `src-tauri/Cargo.toml`, append `"notification-all"` to the existing Tauri
feature array. In `src-tauri/tauri.conf.json`, add under `tauri.allowlist`:

```json
"notification": {
  "all": true
}
```

Do not set the top-level allowlist `all` field to true and do not add shell,
filesystem, process-write, or window-focus permissions.

- [ ] **Step 3: Add the failing persisted-store test**

Create `src/stores/useItemNotificationStore.test.ts`:

```ts
import { beforeEach, expect, it } from "vitest";

import { useItemNotificationStore } from "./useItemNotificationStore";

beforeEach(() => {
  localStorage.clear();
  useItemNotificationStore.persist.clearStorage();
  useItemNotificationStore.setState({
    enabled: false,
    permissionDenied: false,
  });
});

it("defaults off and persists only the enabled preference", async () => {
  expect(useItemNotificationStore.getState().enabled).toBe(false);

  useItemNotificationStore.getState().setEnabled(true);

  const saved = JSON.parse(localStorage.getItem("item-notification-settings") ?? "{}");
  expect(saved.state).toEqual({ enabled: true });
  expect(saved.state).not.toHaveProperty("permissionDenied");
});
```

Run:

```powershell
npm.cmd test -- --run src/stores/useItemNotificationStore.test.ts
```

Expected: FAIL because the store does not exist.

- [ ] **Step 4: Implement the focused store**

Create `src/stores/useItemNotificationStore.ts`:

```ts
import { create } from "zustand";
import { persist } from "zustand/middleware";

type ItemNotificationState = {
  enabled: boolean;
  permissionDenied: boolean;
  setEnabled: (enabled: boolean) => void;
  setPermissionDenied: (permissionDenied: boolean) => void;
};

export const useItemNotificationStore = create<ItemNotificationState>()(
  persist(
    (set) => ({
      enabled: false,
      permissionDenied: false,
      setEnabled: (enabled) => set({ enabled }),
      setPermissionDenied: (permissionDenied) => set({ permissionDenied }),
    }),
    {
      name: "item-notification-settings",
      partialize: (state) => ({ enabled: state.enabled }),
    }
  )
);
```

- [ ] **Step 5: Verify and commit Task 2**

Run:

```powershell
npm.cmd test -- --run src/securityConfiguration.test.ts src/stores/useItemNotificationStore.test.ts
cargo check --locked --package gbfr-logs
```

Expected: all tests PASS and Tauri compiles with the notification feature.

Commit:

```powershell
git add -- src-tauri/Cargo.toml src-tauri/tauri.conf.json src/securityConfiguration.test.ts src/stores/useItemNotificationStore.ts src/stores/useItemNotificationStore.test.ts
git commit -m "feat: add native item notification setting"
```

---

### Task 3: Item Analysis sub-tabs and permission UX

**Files:**
- Create: `src/itemNotificationPermission.ts`
- Modify: `src/pages/ItemAnalysis.tsx`
- Modify: `src/pages/ItemAnalysis.test.tsx`
- Modify: `src/pages/ItemAnalysis.localization.test.ts`
- Modify: `src-tauri/lang/ko/ui.json`
- Modify: `src-tauri/lang/en/ui.json`

**Interfaces:**
- Produces `requestItemNotificationPermission(): Promise<boolean>`.
- Consumes `useItemNotificationStore`.
- Produces translation keys under `ui.item-analysis.tabs` and `ui.item-analysis.notification`.

- [ ] **Step 1: Extend localization and page tests first**

Add these keys to `REQUIRED_KEYS` in
`src/pages/ItemAnalysis.localization.test.ts`:

```ts
"tabs.inventory",
"tabs.notifications",
"notification.label",
"notification.description",
"notification.permission-denied",
```

In `src/pages/ItemAnalysis.test.tsx`, mock the notification API:

```ts
const notification = vi.hoisted(() => ({
  isPermissionGranted: vi.fn(async () => false),
  requestPermission: vi.fn(async () => "denied" as const),
}));

vi.mock("@tauri-apps/api/notification", () => notification);
```

Add tests:

```ts
it("keeps the notification switch off when permission is denied", async () => {
  mocks.responses.push(response());
  renderPage();

  fireEvent.click(await screen.findByRole("tab", { name: "알림 설정" }));
  fireEvent.click(screen.getByRole("switch", { name: "아이템 획득 시, 900개 이상일 경우 알림" }));

  expect((await screen.findByRole("alert")).textContent).toContain(
    "Windows 알림 권한을 허용해야 사용할 수 있습니다."
  );
  expect((screen.getByRole("switch") as HTMLInputElement).checked).toBe(false);
});

it("persists the enabled setting only after permission is granted", async () => {
  notification.requestPermission.mockResolvedValueOnce("granted");
  mocks.responses.push(response());
  renderPage();

  fireEvent.click(await screen.findByRole("tab", { name: "알림 설정" }));
  fireEvent.click(screen.getByRole("switch"));

  await waitFor(() =>
    expect((screen.getByRole("switch") as HTMLInputElement).checked).toBe(true)
  );
  expect(useItemNotificationStore.getState().enabled).toBe(true);
});
```

Add `waitFor` and store reset setup as required by the tests.

- [ ] **Step 2: Run focused tests to verify RED**

Run:

```powershell
npm.cmd test -- --run src/pages/ItemAnalysis.localization.test.ts src/pages/ItemAnalysis.test.tsx
```

Expected: FAIL because tabs, copy, permission helper, and switch do not exist.

- [ ] **Step 3: Implement permission checking**

Create `src/itemNotificationPermission.ts`:

```ts
import { isPermissionGranted, requestPermission } from "@tauri-apps/api/notification";

export const hasItemNotificationPermission = async (): Promise<boolean> => isPermissionGranted();

export const requestItemNotificationPermission = async (): Promise<boolean> => {
  if (await isPermissionGranted()) return true;
  return (await requestPermission()) === "granted";
};
```

Any thrown API error is handled by the caller as a denied/unavailable result.

- [ ] **Step 4: Add the translated copy**

Under `ui.item-analysis` in Korean:

```json
"tabs": {
  "inventory": "보유 현황",
  "notifications": "알림 설정"
},
"notification": {
  "label": "아이템 획득 시, 900개 이상일 경우 알림",
  "description": "전투 종료 후 증가한 일반 아이템을 확인해 Windows 알림으로 알려줍니다.",
  "permission-denied": "Windows 알림 권한을 허용해야 사용할 수 있습니다."
}
```

Under the matching English location:

```json
"tabs": {
  "inventory": "Inventory",
  "notifications": "Notification settings"
},
"notification": {
  "label": "Notify me when an acquired item reaches 900 or more",
  "description": "Checks increased ordinary items after battle and sends a Windows notification.",
  "permission-denied": "Windows notification permission is required to use this setting."
}
```

- [ ] **Step 5: Implement the two sub-tabs**

In `src/pages/ItemAnalysis.tsx`, import Mantine `Tabs`, `Switch`, and the store.
Keep the existing inventory JSX together inside `Tabs.Panel value="inventory"`.
Place only the setting UI inside `Tabs.Panel value="notifications"`.

Use this permission-gated handler:

```ts
const setNotificationEnabled = async (checked: boolean) => {
  if (!checked) {
    setEnabled(false);
    setPermissionDenied(false);
    return;
  }
  try {
    const granted = await requestItemNotificationPermission();
    setPermissionDenied(!granted);
    setEnabled(granted);
  } catch {
    setPermissionDenied(true);
    setEnabled(false);
  }
};
```

Render:

```tsx
<Tabs defaultValue="inventory">
  <Tabs.List>
    <Tabs.Tab value="inventory">{t("ui.item-analysis.tabs.inventory")}</Tabs.Tab>
    <Tabs.Tab value="notifications">{t("ui.item-analysis.tabs.notifications")}</Tabs.Tab>
  </Tabs.List>
  <Tabs.Panel value="inventory" pt="md">
    {/* existing inventory content, unchanged */}
  </Tabs.Panel>
  <Tabs.Panel value="notifications" pt="md">
    <Switch
      label={t("ui.item-analysis.notification.label")}
      description={t("ui.item-analysis.notification.description")}
      checked={enabled}
      onChange={(event) => void setNotificationEnabled(event.currentTarget.checked)}
    />
    {permissionDenied ? (
      <Alert role="alert" color="yellow" mt="md">
        {t("ui.item-analysis.notification.permission-denied")}
      </Alert>
    ) : null}
  </Tabs.Panel>
</Tabs>
```

Keep the page title above the tabs. Keep the refresh button and scan error
inside the Inventory tab so the settings tab does not expose unrelated scan
controls.

- [ ] **Step 6: Verify and commit Task 3**

Run:

```powershell
npm.cmd test -- --run src/pages/ItemAnalysis.localization.test.ts src/pages/ItemAnalysis.test.tsx src/stores/useItemNotificationStore.test.ts
```

Expected: all focused UI and localization tests PASS.

Commit:

```powershell
git add -- src/itemNotificationPermission.ts src/pages/ItemAnalysis.tsx src/pages/ItemAnalysis.test.tsx src/pages/ItemAnalysis.localization.test.ts src-tauri/lang/ko/ui.json src-tauri/lang/en/ui.json
git commit -m "feat: add item notification settings tab"
```

---

### Task 4: Pure item-gain comparison

**Files:**
- Create: `src/itemAcquisitionNotification.ts`
- Create: `src/itemAcquisitionNotification.test.ts`

**Interfaces:**
- Produces `AcquiredWarningItem`.
- Produces `findAcquiredWarningItems(previous, current, threshold)`.
- Produces `formatAcquisitionNotificationBody(items, translateItem, formatRemaining)`.

- [ ] **Step 1: Write boundary and grouping tests**

Create `src/itemAcquisitionNotification.test.ts`:

```ts
import { expect, it } from "vitest";

import {
  findAcquiredWarningItems,
  formatAcquisitionNotificationBody,
} from "./itemAcquisitionNotification";

const entry = (itemId: number, quantity: number) => ({ itemId, quantity });

it("selects every increase whose resulting quantity is at least 900", () => {
  expect(
    findAcquiredWarningItems(
      [entry(1, 899), entry(2, 900), entry(3, 950), entry(4, 999)],
      [entry(1, 900), entry(2, 901), entry(3, 951), entry(4, 998)],
      900
    )
  ).toEqual([
    { itemId: 3, quantity: 951, increase: 1 },
    { itemId: 2, quantity: 901, increase: 1 },
    { itemId: 1, quantity: 900, increase: 1 },
  ]);
});

it("ignores unchanged, decreased, and below-threshold results", () => {
  expect(
    findAcquiredWarningItems(
      [entry(1, 899), entry(2, 900), entry(3, 950)],
      [entry(1, 899), entry(2, 899), entry(3, 950)],
      900
    )
  ).toEqual([]);
});

it("formats one system notification body for all qualifying items", () => {
  const items = [
    { itemId: 1, quantity: 918, increase: 3 },
    { itemId: 2, quantity: 905, increase: 1 },
    { itemId: 3, quantity: 900, increase: 2 },
  ];

  expect(
    formatAcquisitionNotificationBody(
      items,
      (itemId) => (itemId === 1 ? "궁극의 증표" : String(itemId)),
      (remaining) => `외 ${remaining}개`
    )
  ).toBe("궁극의 증표 918 (+3) 외 2개");
});
```

- [ ] **Step 2: Run the comparison tests to verify RED**

Run:

```powershell
npm.cmd test -- --run src/itemAcquisitionNotification.test.ts
```

Expected: FAIL because the module does not exist.

- [ ] **Step 3: Implement the pure functions**

Create `src/itemAcquisitionNotification.ts`:

```ts
import { ItemAnalysisEntry } from "@/types";

export type AcquiredWarningItem = ItemAnalysisEntry & {
  increase: number;
};

export const findAcquiredWarningItems = (
  previous: ItemAnalysisEntry[],
  current: ItemAnalysisEntry[],
  threshold = 900
): AcquiredWarningItem[] => {
  const previousQuantities = new Map(previous.map((item) => [item.itemId, item.quantity]));
  return current
    .flatMap((item): AcquiredWarningItem[] => {
      const previousQuantity = previousQuantities.get(item.itemId);
      if (previousQuantity === undefined || item.quantity <= previousQuantity || item.quantity < threshold) {
        return [];
      }
      return [{ ...item, increase: item.quantity - previousQuantity }];
    })
    .sort((left, right) => right.quantity - left.quantity || left.itemId - right.itemId);
};

export const formatAcquisitionNotificationBody = (
  items: AcquiredWarningItem[],
  translateItem: (itemId: number) => string,
  formatRemaining: (remaining: number) => string
): string => {
  const first = items[0];
  if (!first) return "";
  const summary = `${translateItem(first.itemId)} ${first.quantity} (+${first.increase})`;
  return items.length === 1 ? summary : `${summary} ${formatRemaining(items.length - 1)}`;
};
```

An item absent from a valid complete baseline is ignored because its previous
quantity is unknown. A crossing such as 899 to 900 is still detected because
the complete snapshot includes the 899 record.

- [ ] **Step 4: Verify and commit Task 4**

Run:

```powershell
npm.cmd test -- --run src/itemAcquisitionNotification.test.ts
```

Expected: 3 tests PASS.

Commit:

```powershell
git add -- src/itemAcquisitionNotification.ts src/itemAcquisitionNotification.test.ts
git commit -m "feat: compare post-battle item gains"
```

---

### Task 5: Battle event and global notification controller

**Files:**
- Create: `src/pages/useItemAcquisitionNotifications.ts`
- Create: `src/pages/useItemAcquisitionNotifications.test.tsx`
- Modify: `src/pages/Logs.tsx`
- Modify: `src/pages/Logs.test.tsx`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/lang/ko/ui.json`
- Modify: `src-tauri/lang/en/ui.json`

**Interfaces:**
- Backend emits app-wide event `battle-ended` with a unit payload.
- Produces hook `useItemAcquisitionNotifications(): void`.
- Consumes `fetch_item_inventory_snapshot`, `battle-ended`, the persisted store,
  comparison functions, item translation, and Tauri native notification APIs.

- [ ] **Step 1: Add failing event-wiring regression**

In `src/pages/Logs.test.tsx`, add a source-level contract test consistent with
the existing layout tests:

```ts
it("mounts the item acquisition notification controller in the management layout", () => {
  const source = readFileSync(resolve(process.cwd(), "src/pages/Logs.tsx"), "utf8");
  expect(source).toContain("useItemAcquisitionNotifications();");
});

it("emits the battle-ended app event from the accepted battle-end message", () => {
  const source = readFileSync(resolve(process.cwd(), "src-tauri/src/main.rs"), "utf8");
  expect(source).toMatch(
    /protocol::Message::OnBattleEnd\s*=>\s*\{[\s\S]*state\.on_battle_end_event\(\);[\s\S]*emit_all\(\"battle-ended\"/
  );
});
```

Run:

```powershell
npm.cmd test -- --run src/pages/Logs.test.tsx
```

Expected: FAIL because neither integration exists.

- [ ] **Step 2: Add focused controller tests**

Create `src/pages/useItemAcquisitionNotifications.test.tsx`. Mock:

- `listen` and capture the `battle-ended` callback;
- `invoke` responses for `fetch_item_inventory_snapshot`;
- `isPermissionGranted`;
- `sendNotification`;
- `translateItemId`;
- the persisted store.

Use `vi.useFakeTimers()` and a component that calls only the hook. Cover these
exact scenarios:

```ts
it("uses the first successful snapshot only as a baseline");
it("waits five seconds after battle-ended and sends one grouped notification");
it("debounces repeated battle-ended events into one scan");
it("keeps the previous baseline after a failed scan");
it("cancels the pending scan and clears the baseline when disabled");
it("disables a restored setting when notification permission is unavailable");
```

The grouped test must establish:

```ts
baseline = [
  { itemId: ITEM_A, quantity: 899 },
  { itemId: ITEM_B, quantity: 950 },
];
current = [
  { itemId: ITEM_A, quantity: 900 },
  { itemId: ITEM_B, quantity: 953 },
];
```

After advancing 4,999ms, expect no second invoke and no notification. After one
more millisecond, resolve the scan and expect exactly:

```ts
expect(sendNotification).toHaveBeenCalledTimes(1);
expect(sendNotification).toHaveBeenCalledWith({
  title: "Djeeta MOD · 아이템 분석",
  body: "아이템 B 953 (+3) 외 1개",
});
```

- [ ] **Step 3: Run controller tests to verify RED**

Run:

```powershell
npm.cmd test -- --run src/pages/useItemAcquisitionNotifications.test.tsx src/pages/Logs.test.tsx
```

Expected: FAIL because the hook, event, and layout mount are missing.

- [ ] **Step 4: Emit the accepted battle-end event**

In the existing `protocol::Message::OnBattleEnd` arm in
`src-tauri/src/main.rs`, preserve parser ordering and add:

```rust
protocol::Message::OnBattleEnd => {
    state.on_battle_end_event();
    let _ = app.emit_all("battle-ended", ());
}
```

Do not change the hook protocol variant, reward hook ordering, encounter save,
or meter clearing behavior.

- [ ] **Step 5: Add notification translations**

Under `ui.item-analysis.notification`, add Korean:

```json
"title": "Djeeta MOD · 아이템 분석",
"remaining": "외 {{count}}개"
```

Add English:

```json
"title": "Djeeta MOD · Item Analysis",
"remaining": "and {{count}} more"
```

The item name continues through `translateItemId`, including its existing ID
fallback.

- [ ] **Step 6: Implement the layout-level controller**

Create `src/pages/useItemAcquisitionNotifications.ts` with:

```ts
const POST_BATTLE_DELAY_MS = 5_000;

export const useItemAcquisitionNotifications = () => {
  const { t } = useTranslation();
  const enabled = useItemNotificationStore((state) => state.enabled);
  const setEnabled = useItemNotificationStore((state) => state.setEnabled);
  const setPermissionDenied = useItemNotificationStore((state) => state.setPermissionDenied);
  const baselineRef = useRef<ItemAnalysisEntry[] | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const fetchSnapshot = useCallback(async () => {
    const value = await invoke<unknown>("fetch_item_inventory_snapshot");
    return normalizeItemInventorySnapshotResponse(value).items;
  }, []);

  const establishBaseline = useCallback(async () => {
    try {
      baselineRef.current = await fetchSnapshot();
    } catch {
      // A missing baseline intentionally stays null.
    }
  }, [fetchSnapshot]);

  const inspectAfterBattle = useCallback(async () => {
    try {
      const current = await fetchSnapshot();
      const previous = baselineRef.current;
      baselineRef.current = current;
      if (previous === null) return;
      const acquired = findAcquiredWarningItems(previous, current, 900);
      if (acquired.length === 0) return;
      sendNotification({
        title: t("ui.item-analysis.notification.title"),
        body: formatAcquisitionNotificationBody(
          acquired,
          translateItemId,
          (count) => t("ui.item-analysis.notification.remaining", { count })
        ),
      });
    } catch {
      // Preserve the last valid baseline and stay silent.
    }
  }, [fetchSnapshot, t]);
```

Complete the controller with one lifecycle effect so permission, baseline, and
listener ordering cannot race:

```ts
  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;

    const clearPending = () => {
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    };

    if (!enabled) {
      clearPending();
      baselineRef.current = null;
      return () => {
        active = false;
      };
    }

    const start = async () => {
      let granted = false;
      try {
        granted = await hasItemNotificationPermission();
      } catch {
        granted = false;
      }
      if (!active) return;
      if (!granted) {
        baselineRef.current = null;
        setPermissionDenied(true);
        setEnabled(false);
        return;
      }

      setPermissionDenied(false);
      await establishBaseline();
      if (!active) return;

      const dispose = await listen("battle-ended", () => {
        clearPending();
        timerRef.current = setTimeout(() => {
          timerRef.current = null;
          void inspectAfterBattle();
        }, POST_BATTLE_DELAY_MS);
      });
      if (!active) {
        dispose();
        return;
      }
      unlisten = dispose;
    };

    void start();
    return () => {
      active = false;
      clearPending();
      unlisten?.();
      baselineRef.current = null;
    };
  }, [
    enabled,
    establishBaseline,
    inspectAfterBattle,
    setEnabled,
    setPermissionDenied,
  ]);
};
```

In `inspectAfterBattle`, assign the newly successful snapshot to
`baselineRef.current` before calling `sendNotification`. If native notification
sending throws, catch it without restoring the old baseline so the same reward
is not retried.

In `LayoutContent` in `src/pages/Logs.tsx`, call the hook once near the other
global hooks:

```ts
useItemAcquisitionNotifications();
```

- [ ] **Step 7: Verify Task 5**

Run:

```powershell
npm.cmd test -- --run src/pages/useItemAcquisitionNotifications.test.tsx src/pages/Logs.test.tsx src/pages/ItemAnalysis.localization.test.ts
cargo test --locked --package gbfr-logs tests::unsupported_handshake_rejects_later_gameplay
cargo test --locked --package gbfr-logs parser::v1::tests::battle_end_event_clears_and_saves_once
```

Expected: controller, event wiring, localization, and existing battle-end
lifecycle tests PASS.

- [ ] **Step 8: Commit Task 5**

```powershell
git add -- src/pages/useItemAcquisitionNotifications.ts src/pages/useItemAcquisitionNotifications.test.tsx src/pages/Logs.tsx src/pages/Logs.test.tsx src-tauri/src/main.rs src-tauri/lang/ko/ui.json src-tauri/lang/en/ui.json
git commit -m "feat: notify post-battle item gains"
```

---

### Task 6: Manual validation tracker and full verification

**Files:**
- Modify: `docs/testing/game-2.0.2-item-analysis-probe.md`

**Interfaces:**
- Records automated completion separately from unobserved Windows/game behavior.

- [ ] **Step 1: Extend the remaining-task checklist**

Append these entries under `Remaining tasks`:

```markdown
- [x] Verify the notification setting defaults off and requires Windows notification permission.
- [x] Verify automated snapshot comparison, 900-inclusive boundaries, five-second debounce,
  grouping, and failure handling.
- [ ] In an offline or private session, enable the setting and verify one post-battle Windows
  notification for an ordinary item that increases to at least 900.
- [ ] Verify multiple qualifying post-battle item gains are grouped into one Windows notification.
- [ ] Verify no notification appears for an unchanged/decreased item or while the setting is off.
- [ ] Verify the notification remains visible through Windows Notification Center when the
  management window is hidden.
```

Do not mark the four manual checks complete without observed evidence.

- [ ] **Step 2: Run focused regression tests**

Run:

```powershell
cargo test --locked --package gbfr-logs item_analysis::tests
npm.cmd test -- --run src/itemAnalysisContract.test.ts src/itemAcquisitionNotification.test.ts src/stores/useItemNotificationStore.test.ts src/pages/ItemAnalysis.localization.test.ts src/pages/ItemAnalysis.test.tsx src/pages/useItemAcquisitionNotifications.test.tsx src/pages/Logs.test.tsx src/securityConfiguration.test.ts
```

Expected: all focused Rust and frontend tests PASS.

- [ ] **Step 3: Run required frontend verification**

Run each command and require exit code 0:

```powershell
npm.cmd run format-check
npm.cmd run lint
npm.cmd run tsc
npm.cmd test -- --run
npm.cmd run build
```

Expected:

- Prettier reports all matched files formatted.
- ESLint reports no errors.
- TypeScript reports no errors.
- Vitest reports zero failed files and zero failed tests.
- Vite completes the production build. Existing chunk-size warnings are not failures.

- [ ] **Step 4: Run required Rust verification**

Run:

```powershell
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
```

Expected: both commands exit 0. Existing hook dead-code and `ctor` cfg warnings
may remain, but no new item-notification warning is acceptable.

- [ ] **Step 5: Review the final diff and commit the tracker**

Run:

```powershell
git diff --check
git status --short
git diff main...HEAD --stat
```

Confirm that `AGENTS.md`, `docs/research/2026-07-24-relink-modding-reference.md`,
and `logs.db` are not staged or committed.

Commit:

```powershell
git add -- docs/testing/game-2.0.2-item-analysis-probe.md
git commit -m "docs: track item notification validation"
```

- [ ] **Step 6: Finish the development branch**

Invoke `superpowers:verification-before-completion`, then
`superpowers:finishing-a-development-branch`. Report the exact automated test
counts and leave all manual game/Windows checks explicitly pending.

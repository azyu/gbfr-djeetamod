# Equipment Trait Overflow Analysis Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show every local character's equipped trait totals in Djeeta MOD and identify traits below, at, or above their verified Granblue Fantasy: Relink - Endless Ragnarok 2.0.2 effective level cap.

**Architecture:** Generate a version-pinned trait-cap catalog from the installed 2.0.2 `skill_status` table, capture validated local equipment snapshots in the injected Rust DLL, and append a dedicated wire message without reordering existing bincode variants. The Tauri backend stores the latest snapshot per character and runs a pure analyzer; React renders a character selector and overflow-first table.

**Tech Stack:** Rust nightly-2024-05-04, serde/bincode, Tauri 1, React 18, TypeScript, Zustand, Mantine, Vitest, Rust unit tests, GBFRDataTools 2.0.0.

## Global Constraints

- Public name: `Djeeta MOD`; package: `djeeta-mod`; Tauri identifier: `com.azyu.djeeta-mod`.
- Target: Granblue Fantasy: Relink - Endless Ragnarok 2.0.2 on Windows x64.
- Verified local executable SHA-256: `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F`.
- Equipment capture is read-only; never alter game memory, saves, or inputs.
- Online party equipment is out of scope.
- Unknown caps and incomplete snapshots never become numeric zeroes or normal results.
- `Message` variants are append-only; never reorder existing bincode variants.
- Equipment-only failure must not change the core meter's latched hook status.
- Do not claim 2.0.2 compatibility until the offline/private manual checklist passes.
- Every behavior change follows RED → GREEN tests and ends in a focused commit.

---

## File Structure

**Create**

- `src-tauri/src/bin/build_trait_caps.rs` — maintainer-only 2.0.2 catalog generator.
- `src-tauri/assets/trait-caps.json` — generated trait-cap catalog.
- `src-hook/src/hooks/equipment.rs` — validated layout decoding, caching, and emission.
- `src-tauri/src/equipment/mod.rs` — backend state and response types.
- `src-tauri/src/equipment/analyzer.rs` — pure trait aggregation and classification.
- `src/stores/useEquipmentAnalysisStore.ts` — live response store.
- `src/pages/EquipmentAnalysis.tsx` and `src/pages/EquipmentAnalysis.test.tsx` — UI and tests.
- `docs/testing/game-2.0.2-equipment-layout.md` — controlled layout evidence.

**Modify**

- `protocol/src/lib.rs` and `protocol/tests/legacy_damage_wire.rs` — append-only equipment event.
- `src-hook/src/hooks/{ffi,mod,player}.rs` and `src-hook/Cargo.toml` — safe capture installation.
- `src-tauri/src/main.rs` — receive, fetch, emit, and clear analysis state.
- `src/types.ts`, `src/App.tsx`, `src/pages/Logs.tsx` — frontend contracts and route.
- `src-tauri/lang/{ko,en}/ui.json` — Korean UI and English fallback.
- `docs/research/2026-07-18-gbfr-er-2.0.2-trait-overflow.md` — provenance.
- `docs/testing/game-2.0.2-smoke-test.md` and `README.md` — verification hashes.

---

### Task 1: Generate the 2.0.2 Trait-Cap Catalog

**Files:**
- Create: `src-tauri/src/bin/build_trait_caps.rs`
- Create: `src-tauri/assets/trait-caps.json`
- Modify: `docs/research/2026-07-18-gbfr-er-2.0.2-trait-overflow.md`

**Interfaces:**
- Consumes: SQLite made from 2.0.2 `system/table/skill_status.tbl` with GBFRDataTools 2.0.0.
- Produces: `TraitCapCatalog { game_exe_sha256, records }` where each record is `{ traitId, maxLevel }`.

- [ ] **Step 1: Extract only the required table outside the repository**

```powershell
$gameDir = 'D:\SteamLibrary\steamapps\common\Granblue Fantasy Relink'
$workDir = Join-Path $env:TEMP 'djeeta-trait-caps-2.0.2'
$toolsExe = Join-Path $workDir 'GBFRDataTools.exe'
$tableDir = Join-Path $workDir 'tables\system\table'
$sqlitePath = Join-Path $workDir 'gbfr-2.0.2.sqlite'
New-Item -ItemType Directory -Force -Path $tableDir | Out-Null
& $toolsExe extract -i (Join-Path $gameDir 'data.i') -f 'system/table/skill_status.tbl' -o (Join-Path $workDir 'tables')
& $toolsExe tbl-to-sqlite -i $tableDir -o $sqlitePath -v 2.0.2
Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $gameDir 'granblue_fantasy_relink.exe')
```

Expected: the executable hash matches the global constraint; SQLite has `skill_status(Key, Level)`. No proprietary table or SQLite file enters the repository.

- [ ] **Step 2: Write the failing generator test**

```rust
#[test]
fn selects_highest_level_per_trait_and_sorts_by_id() {
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch(
        "CREATE TABLE skill_status (Key TEXT NOT NULL, Level INTEGER NOT NULL);
         INSERT INTO skill_status VALUES
         ('0x00000002', 10), ('0x00000001', 1),
         ('0x00000001', 65), ('0x887AE0B0', 99);"
    ).unwrap();

    assert_eq!(
        load_records(&connection).unwrap(),
        vec![
            TraitCapRecord { trait_id: 1, max_level: 65 },
            TraitCapRecord { trait_id: 2, max_level: 10 },
        ]
    );
}
```

- [ ] **Step 3: Run RED**

```powershell
cargo test --locked --package gbfr-logs --bin build_trait_caps
```

Expected: FAIL because the generator and types do not exist.

- [ ] **Step 4: Implement the minimal generator**

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TraitCapRecord { trait_id: u32, max_level: u32 }

fn load_records(connection: &Connection) -> Result<Vec<TraitCapRecord>> {
    let mut statement = connection.prepare(
        "SELECT Key, MAX(Level) FROM skill_status GROUP BY Key ORDER BY Key"
    )?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
    })?;
    let mut records = Vec::new();
    for row in rows {
        let (key, max_level) = row?;
        let trait_id = u32::from_str_radix(key.trim().trim_start_matches("0x"), 16)?;
        if trait_id != 0 && trait_id != 0x887A_E0B0 && max_level > 0 {
            records.push(TraitCapRecord { trait_id, max_level });
        }
    }
    records.sort_by_key(|record| record.trait_id);
    Ok(records)
}
```

`main` accepts exactly three arguments: SQLite input, JSON output, and executable SHA-256. It serializes `gameVersion: "2.0.2"`, the uppercase hash, and sorted records with `serde_json::to_writer_pretty`.

- [ ] **Step 5: Run GREEN and generate the catalog**

```powershell
cargo test --locked --package gbfr-logs --bin build_trait_caps
cargo run --locked --package gbfr-logs --bin build_trait_caps -- $sqlitePath 'src-tauri\assets\trait-caps.json' '63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F'
```

Expected: PASS; Damage Cap has `maxLevel: 65`; JSON is deterministic.

- [ ] **Step 6: Record provenance and commit**

Record GBFRDataTools tag `2.0.0`, executable hash, extracted table hash, row count, and generation command.

```powershell
git add src-tauri/src/bin/build_trait_caps.rs src-tauri/assets/trait-caps.json docs/research/2026-07-18-gbfr-er-2.0.2-trait-overflow.md
git commit -m "data: add verified 2.0.2 trait caps"
```

---

### Task 2: Append the Equipment Snapshot Wire Contract

**Files:**
- Modify: `protocol/src/lib.rs`
- Modify: `protocol/tests/legacy_damage_wire.rs`

**Interfaces:**
- Produces: `EquipmentSourceKind`, `EquipmentCaptureStatus`, `EquippedTraitSource`, `LocalEquipmentSnapshotEvent`.
- Preserves: every historical ordinal; `LocalEquipmentSnapshot` is the final variant.

- [ ] **Step 1: Write a failing round-trip test**

```rust
#[test]
fn local_equipment_snapshot_round_trips() {
    let message = Message::LocalEquipmentSnapshot(LocalEquipmentSnapshotEvent {
        character_type: 0x4C71_4F77,
        status: EquipmentCaptureStatus::Complete,
        sources: vec![EquippedTraitSource {
            kind: EquipmentSourceKind::SigilPrimary,
            slot: 0,
            item_id: 0x1234_5678,
            trait_id: 0x9ABC_DEF0,
            trait_level: 15,
        }],
    });
    let encoded = bincode::serialize(&message).unwrap();
    assert!(matches!(
        bincode::deserialize::<Message>(&encoded).unwrap(),
        Message::LocalEquipmentSnapshot(_)
    ));
}
```

Retain existing legacy fixture assertions unchanged.

- [ ] **Step 2: Run RED**

```powershell
cargo test --locked --package protocol
```

Expected: FAIL because the equipment contract does not exist.

- [ ] **Step 3: Add the contract**

```rust
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquipmentSourceKind {
    SigilPrimary, SigilSecondary, Weapon, Wrightstone, MasterTrait, Summon,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipmentCaptureStatus { Complete, Unsupported }

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct EquippedTraitSource {
    pub kind: EquipmentSourceKind,
    pub slot: u8,
    pub item_id: u32,
    pub trait_id: u32,
    pub trait_level: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct LocalEquipmentSnapshotEvent {
    pub character_type: u32,
    pub status: EquipmentCaptureStatus,
    pub sources: Vec<EquippedTraitSource>,
}
```

Append `LocalEquipmentSnapshot(LocalEquipmentSnapshotEvent)` after `HookStatus`. Do not add it to `LegacyMessage`.

- [ ] **Step 4: Run GREEN and commit**

```powershell
cargo test --locked --package protocol
git add protocol/src/lib.rs protocol/tests/legacy_damage_wire.rs
git commit -m "feat: add local equipment snapshot protocol"
```

---

### Task 3: Verify the 2.0.2 Local Equipment Layout

**Files:**
- Create: `docs/testing/game-2.0.2-equipment-layout.md`
- Modify: `src-hook/src/hooks/player.rs`
- Modify: `src-hook/Cargo.toml`

**Interfaces:**
- Consumes: verified refresh-player record, `PLAYER_IDENTITY_OFFSET = 0x5E60`, `PLAYER_KEY_OFFSET = 0x5EA8`, safe `ReadProcessMemory` helpers.
- Produces: documented numeric `EquipmentLayout` and controlled fixture rules for Task 4. No production equipment event is emitted.

- [ ] **Step 1: Add a compile-time diagnostic feature**

Add `equipment-debug = []`. Under that feature, log bounded bytes only for local records.

```rust
#[cfg(feature = "equipment-debug")]
fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(feature = "equipment-debug")]
fn log_equipment_probe(record: *const usize, snapshot: *const u8, player_key: u32) {
    if let Some(snapshot) = super::read_process_bytes(snapshot, 0x250) {
        info!("Equipment snapshot: key={player_key:#010x}, bytes={}", encode_hex(&snapshot));
    }
    if let Some(record) = super::read_process_bytes(record.cast::<u8>(), 0x6000) {
        info!("Equipment record: key={player_key:#010x}, bytes={}", encode_hex(&record));
    }
}
```

Default builds contain no full record logging.

- [ ] **Step 2: Verify unit safety before probing**

```powershell
cargo test --locked --package hook
```

Expected: existing hook tests PASS.

- [ ] **Step 3: Capture controlled offline samples**

For the same character, capture: twelve empty sigil slots; one known primary trait in slot 0; one known V+ in slot 0; and two known weapon/wrightstone combinations.

```powershell
cargo build --release --locked --package hook --features equipment-debug,console
Get-FileHash -Algorithm SHA256 -LiteralPath 'D:\SteamLibrary\steamapps\common\Granblue Fantasy Relink\granblue_fantasy_relink.exe'
```

Expected: samples share a character key; only controlled item fields change; the DLL performs no memory writes.

- [ ] **Step 4: Document differential evidence**

The evidence document records the executable hash, hook signature and unique match count, record/snapshot relationship, the local roster container and iteration bounds, character key mapping, sigil array base/stride/count, every trait/item/level offset, weapon and wrightstone sources, Master Trait/Summon participation, and rejection bounds.

Each numeric offset needs two controlled samples isolating the same field. If a normal trait-level source cannot be isolated, document `Unsupported` and keep production emission disabled.

- [ ] **Step 5: Remove raw record logging and commit**

```powershell
cargo test --locked --package hook
git add src-hook/src/hooks/player.rs src-hook/Cargo.toml docs/testing/game-2.0.2-equipment-layout.md
git commit -m "docs: verify 2.0.2 equipment layout"
```

Expected: default tests PASS; normal builds log no raw equipment bytes.

---

### Task 4: Capture and Deduplicate Equipment Snapshots

**Files:**
- Create: `src-hook/src/hooks/equipment.rs`
- Modify: `src-hook/src/hooks/ffi.rs`
- Modify: `src-hook/src/hooks/mod.rs`
- Modify: `src-hook/src/hooks/player.rs`

**Interfaces:**
- Consumes: Task 2 wire types and Task 3 numeric layout.
- Produces: `decode_snapshot` plus changed-only event emission.

- [ ] **Step 1: Write failing decoder/cache tests**

```rust
#[test]
fn decodes_sources_and_skips_empty_slots() {
    let fixture = fixture_with_sources(&[
        (EquipmentSourceKind::SigilPrimary, 0, 0x1000, 0x2000, 15),
        (EquipmentSourceKind::SigilSecondary, 0, 0x1000, 0x3000, 11),
        (EquipmentSourceKind::Weapon, 0, 0x4000, 0x2000, 5),
    ]);
    let event = decode_snapshot(&fixture).unwrap();
    assert_eq!(event.status, EquipmentCaptureStatus::Complete);
    assert_eq!(event.sources.len(), 3);
}

#[test]
fn identical_snapshots_are_suppressed() {
    let mut cache = EquipmentSnapshotCache::default();
    let event = complete_event(0x4C71_4F77);
    assert!(cache.replace_if_changed(event.clone()).is_some());
    assert!(cache.replace_if_changed(event).is_none());
}

#[test]
fn distinct_local_character_keys_keep_distinct_snapshots() {
    let mut cache = EquipmentSnapshotCache::default();
    assert!(cache.replace_if_changed(complete_event(0x4C71_4F77)).is_some());
    assert!(cache.replace_if_changed(complete_event(0xC315_5079)).is_some());
    assert_eq!(cache.len(), 2);
}
```

Also test a partial buffer and level `10_001` rejection.

- [ ] **Step 2: Run RED**

```powershell
cargo test --locked --package hook equipment
```

Expected: FAIL because equipment decoding does not exist.

- [ ] **Step 3: Implement bounded decoding**

```rust
const EMPTY_HASH: u32 = 0x887A_E0B0;
const MAX_TRAIT_LEVEL: u32 = 10_000;

fn valid_trait(trait_id: u32, level: u32) -> bool {
    trait_id != 0
        && trait_id != EMPTY_HASH
        && (1..=MAX_TRAIT_LEVEL).contains(&level)
}
```

Copy the documented Task 3 constants into one `EquipmentLayout`. At initial connection, walk the validated local roster container within its documented maximum count and capture every usable character; on the refresh hook, replace only the changed character. Read through `read_process_value` or a bounded copied slice, never direct pointer dereferences. Emit `Complete` only after every normal trait-level source documented in Task 3 is decoded.

- [ ] **Step 4: Install as an optional subsystem**

```rust
match equipment::OnLocalEquipmentRefreshHook::new(tx.clone()).setup(&process) {
    Ok(()) => info!("Local equipment hook enabled"),
    Err(error) => warn!("Local equipment analysis unavailable: {error}"),
}
```

Do not return the equipment error from `setup_hooks`.

- [ ] **Step 5: Run GREEN and commit**

```powershell
cargo test --locked --package hook
cargo test --workspace --all-targets --locked
git add src-hook/src/hooks/equipment.rs src-hook/src/hooks/ffi.rs src-hook/src/hooks/mod.rs src-hook/src/hooks/player.rs
git commit -m "feat: capture local equipment traits"
```

Expected: PASS; invalid or unchanged reads never emit a complete duplicate event.

---

### Task 5: Analyze Trait Totals in Tauri

**Files:**
- Create: `src-tauri/src/equipment/mod.rs`
- Create: `src-tauri/src/equipment/analyzer.rs`
- Modify: `src-tauri/src/main.rs`

**Interfaces:**
- Consumes: equipment events and `assets/trait-caps.json`.
- Produces: `EquipmentAnalysisResponse`, `fetch_equipment_analysis`, `equipment-analysis-update`.

- [ ] **Step 1: Write failing analyzer/state tests**

```rust
#[test]
fn classifies_all_states() {
    let caps = HashMap::from([(1, 65), (2, 50), (3, 45)]);
    let sources = vec![
        source(1, 60), source(1, 12), source(2, 50), source(3, 30), source(4, 15),
    ];
    let results = analyze_sources(&sources, &caps).unwrap();
    assert_eq!(find(&results, 1).state, TraitState::Overflow);
    assert_eq!(find(&results, 1).overflow_level, 7);
    assert_eq!(find(&results, 2).state, TraitState::Capped);
    assert_eq!(find(&results, 3).state, TraitState::Below);
    assert_eq!(find(&results, 4).state, TraitState::Unknown);
}
```

Add a state test proving `Unsupported` returns no numeric traits.

- [ ] **Step 2: Run RED**

```powershell
cargo test --locked --package gbfr-logs equipment
```

Expected: FAIL because the analyzer does not exist.

- [ ] **Step 3: Implement pure analysis**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TraitState { Overflow, Capped, Below, Unknown }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraitAnalysis {
    pub trait_id: u32,
    pub total_level: u32,
    pub max_level: Option<u32>,
    pub overflow_level: u32,
    pub state: TraitState,
    pub sources: Vec<EquippedTraitSource>,
}
```

Group by `trait_id`, sum with `checked_add`, and sort `Overflow, Capped, Below, Unknown` then `trait_id`. Overflow or incomplete input makes the character unsupported instead of saturating.

- [ ] **Step 4: Wire command and events**

```rust
protocol::Message::LocalEquipmentSnapshot(event) => {
    let response = {
        let mut state = equipment_state.lock().expect("equipment state lock poisoned");
        state.apply(event);
        state.response()
    };
    let _ = logs_window.emit("equipment-analysis-update", response);
}
```

Register `fetch_equipment_analysis`. Clear state and emit an empty response on disconnect.

- [ ] **Step 5: Run GREEN and commit**

```powershell
cargo test --locked --package gbfr-logs equipment
cargo test --workspace --all-targets --locked
git add src-tauri/src/equipment/mod.rs src-tauri/src/equipment/analyzer.rs src-tauri/src/main.rs
git commit -m "feat: analyze equipment trait overflow"
```

---

### Task 6: Add the Equipment Analysis Screen

**Files:**
- Create: `src/stores/useEquipmentAnalysisStore.ts`
- Create: `src/pages/EquipmentAnalysis.tsx`
- Create: `src/pages/EquipmentAnalysis.test.tsx`
- Modify: `src/types.ts`
- Modify: `src/App.tsx`
- Modify: `src/pages/Logs.tsx`
- Modify: `src-tauri/lang/ko/ui.json`
- Modify: `src-tauri/lang/en/ui.json`

**Interfaces:**
- Consumes: fetch command and update event.
- Produces: `/logs/equipment` with character selection and source details.

- [ ] **Step 1: Write failing UI tests**

```tsx
it("shows overflow first and preserves unknown caps", async () => {
  mockResponse({
    characters: [{
      characterType: "Pl0000",
      status: "complete",
      traits: [
        { traitId: 2, totalLevel: 15, maxLevel: null, overflowLevel: 0, state: "unknown", sources: [] },
        { traitId: 1, totalLevel: 72, maxLevel: 65, overflowLevel: 7, state: "overflow", sources: [] },
      ],
    }],
  });
  renderEquipmentAnalysis();
  expect(await screen.findByText("72 / 65")).toBeInTheDocument();
  expect(screen.getByText("7 초과")).toBeInTheDocument();
  expect(screen.getByText("최대치 미확인")).toBeInTheDocument();
});
```

Also test selection preservation and unsupported/game-disconnected states.

- [ ] **Step 2: Run RED**

```powershell
npm test -- --run src/pages/EquipmentAnalysis.test.tsx
```

Expected: FAIL because route, store, and screen do not exist.

- [ ] **Step 3: Add TypeScript contracts and the store**

```ts
export type TraitAnalysisState = "overflow" | "capped" | "below" | "unknown";
export type CharacterEquipmentStatus = "complete" | "unsupported";

export type TraitAnalysis = {
  traitId: number;
  totalLevel: number;
  maxLevel: number | null;
  overflowLevel: number;
  state: TraitAnalysisState;
  sources: EquippedTraitSource[];
};
```

The store fetches on mount, subscribes once, retains a still-present selection, and otherwise selects the first character.

- [ ] **Step 4: Implement route, navigation, and screen**

Use `translateTraitId`. Render Korean copy `초과`, `최대`, `정상`, `최대치 미확인`, `장비 정보 미지원`, `게임 연결 대기 중` with English fallbacks. Expand a row to show source kind, slot, item ID, and contributed level. Do not encode meaning by color alone.

- [ ] **Step 5: Run GREEN and commit**

```powershell
npm test -- --run src/pages/EquipmentAnalysis.test.tsx
npm run tsc
npm run lint
npm run format-check
git add src/stores/useEquipmentAnalysisStore.ts src/pages/EquipmentAnalysis.tsx src/pages/EquipmentAnalysis.test.tsx src/types.ts src/App.tsx src/pages/Logs.tsx src-tauri/lang/ko/ui.json src-tauri/lang/en/ui.json
git commit -m "feat: show equipment trait overflow"
```

---

### Task 7: Integrate, Package, and Record Verification

**Files:**
- Modify: `docs/testing/game-2.0.2-smoke-test.md`
- Modify: `README.md` after packaging succeeds.
- Modify: `src-tauri/hook.dll` after a verified release build.

- [ ] **Step 1: Add unchecked manual cases**

Add cases for empty slots, primary trait, V+ secondary trait, `64/65`, `65/65`, `72/65`, weapon/wrightstone contribution, Master Trait/Summon participation, character switching, one-item refresh, disconnect clearing, and equipment signature failure leaving the meter functional. Do not mark them complete without observation.

- [ ] **Step 2: Run required automated verification**

```powershell
npm ci
npm run format-check
npm run lint
npm run tsc
npm test -- --run
npm run build
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
```

Expected: every command exits 0.

- [ ] **Step 3: Copy the verified hook and package**

```powershell
$repo = (Resolve-Path '.').Path
$builtHook = (Resolve-Path 'target\release\hook.dll').Path
$bundledHook = (Resolve-Path 'src-tauri\hook.dll').Path
if (-not $builtHook.StartsWith($repo) -or -not $bundledHook.StartsWith($repo)) { throw 'hook path escaped repository' }
Copy-Item -LiteralPath $builtHook -Destination $bundledHook -Force
npm run tauri build -- --bundles msi
```

Expected: MSI exists below `target/release/bundle/msi`.

- [ ] **Step 4: Verify and record hashes**

```powershell
$builtHash = (Get-FileHash -Algorithm SHA256 -LiteralPath 'target\release\hook.dll').Hash
$bundledHash = (Get-FileHash -Algorithm SHA256 -LiteralPath 'src-tauri\hook.dll').Hash
if ($builtHash -ne $bundledHash) { throw 'hook hashes differ' }
$msi = Get-ChildItem -LiteralPath 'target\release\bundle\msi' -Filter '*.msi' | Sort-Object LastWriteTime -Descending | Select-Object -First 1
$msiHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $msi.FullName).Hash
```

Record both hashes in `README.md` and the smoke-test document. Manual cases remain unchecked until observed.

- [ ] **Step 5: Final diff check and packaging commit**

```powershell
git diff --check
git add README.md docs/testing/game-2.0.2-smoke-test.md src-tauri/hook.dll
git commit -m "chore: package equipment trait analysis"
```

Expected: `git diff --check` has no output.

---

## Completion Gate

Tasks 1–7 must be committed, every automated command must pass, built and bundled hook hashes must match, and incomplete snapshots must never expose numeric totals. The product remains explicitly unverified for game 2.0.2 until the offline/private manual checklist is completed.

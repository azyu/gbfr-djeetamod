# 2.0.2 Item Translation Catalog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Regenerate the complete Korean and English item-name catalogs from pinned Granblue Fantasy: Relink Endless Ragnarok 2.0.2 assets and record every remaining item-analysis validation task.

**Architecture:** Add a maintainer-only Rust example that reads the converted `item` SQLite table and Korean/English MessagePack text tables, validates both languages as one unit, and atomically writes the existing i18next JSON contract. Keep runtime lookup unchanged, add bundled-catalog regression tests to the generator, and use the existing item-analysis evidence file as the remaining-work tracker.

**Tech Stack:** Rust 2021, rusqlite, rmp-serde, serde/serde_json, Cargo examples, Vitest, JSON/i18next, PowerShell, GBFRDataTools 2.0.0.

## Global Constraints

- Target only Granblue Fantasy: Relink Endless Ragnarok 2.0.2 on Windows x64.
- Require executable SHA-256 `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F`.
- Regenerate only `src-tauri/lang/ko/items.json` and `src-tauri/lang/en/items.json`; do not change other languages.
- Preserve the runtime entry contract `{ "key": string, "text": string }` keyed by eight-digit lowercase item hash.
- Include all 384 rows with non-empty `ItemName`; exclude only the 64 unnamed internal rows.
- Do not commit game archives, extracted `.tbl`/`.msg` files, SQLite intermediates, or third-party extraction binaries.
- Treat `logs.db` as user data: do not read, modify, delete, stage, or commit it.
- Do not mark manual game checks complete without personally observed evidence.
- Do not claim full game 2.0.2 compatibility from this catalog refresh.
- Preserve unrelated `AGENTS.md` and `docs/research/2026-07-24-relink-modding-reference.md` changes.

## File Structure

- Create `src-tauri/examples/build_item_catalog.rs`: isolated parser, validator, deterministic serializer, atomic two-file writer, CLI, and focused unit tests.
- Modify `src-tauri/Cargo.toml`: register the maintainer example explicitly.
- Replace `src-tauri/lang/ko/items.json`: generated Korean 2.0.2 catalog.
- Replace `src-tauri/lang/en/items.json`: generated English 2.0.2 catalog.
- Create `docs/research/2026-07-24-item-catalog-2.0.2.md`: reproducible extraction/generation provenance without copyrighted data.
- Modify `docs/testing/game-2.0.2-item-analysis-probe.md`: add the agreed `Remaining tasks` checklist and check only completed automated work.

---

### Task 1: Build the Deterministic Two-Language Catalog Generator

**Files:**

- Create: `src-tauri/examples/build_item_catalog.rs`
- Modify: `src-tauri/Cargo.toml`

**Interfaces:**

- Consumes: read-only SQLite table `item(Key TEXT, ItemName TEXT, ...)`, Korean `text.msg`, English `text.msg`, two destination paths, and the pinned executable hash.
- Produces:
  - `fn load_item_definitions(connection: &Connection) -> Result<Vec<ItemDefinition>>`
  - `fn parse_message_names(bytes: &[u8], language: &str) -> Result<BTreeMap<String, String>>`
  - `fn generate_catalogs(connection: &Connection, ko_message: &[u8], en_message: &[u8], game_exe_sha256: &str) -> Result<GeneratedCatalogs>`
  - `fn write_prepared_outputs(outputs: [(&Path, &[u8]); 2]) -> Result<()>`
  - CLI: `build_item_catalog <input.sqlite> <ko-text.msg> <en-text.msg> <ko-items.json> <en-items.json> <game-exe-sha256>`

- [ ] **Step 1: Register the example and create failing schema/hash tests**

Add to `src-tauri/Cargo.toml`:

```toml
[[example]]
name = "build_item_catalog"
path = "examples/build_item_catalog.rs"
```

Create `src-tauri/examples/build_item_catalog.rs` with the data types below and a test module. The first implementation should contain only type definitions and `unimplemented!()` function bodies so the tests compile and fail.

```rust
use std::{
    collections::{BTreeMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};

const GAME_VERSION: &str = "2.0.2";
const GAME_EXE_SHA256: &str =
    "63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F";
const EXPECTED_ITEM_ROWS: usize = 448;
const EXPECTED_LOCALIZED_ITEMS: usize = 384;
const EXPECTED_UNNAMED_ROWS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ItemDefinition {
    item_id: u32,
    key: String,
    name_key: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct MessageTable {
    rows_: Vec<MessageRow>,
}

#[derive(Debug, Deserialize, Serialize)]
struct MessageRow {
    column_: MessageColumn,
}

#[derive(Debug, Deserialize, Serialize)]
struct MessageColumn {
    id_hash_: String,
    subid_hash_: String,
    text_: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ItemNameRecord {
    key: String,
    text: String,
}

type ItemNameCatalog = BTreeMap<String, ItemNameRecord>;

#[derive(Debug)]
struct GeneratedCatalogs {
    ko_names: Vec<u8>,
    en_names: Vec<u8>,
    item_count: usize,
}
```

Use an in-memory SQLite fixture and MessagePack fixture:

```rust
fn sqlite_fixture(rows: &[(&str, &str)]) -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch(r#"CREATE TABLE item ("Key" TEXT, "ItemName" TEXT);"#)
        .unwrap();
    for (key, name_key) in rows {
        connection
            .execute(
                r#"INSERT INTO item ("Key", "ItemName") VALUES (?1, ?2)"#,
                [key, name_key],
            )
            .unwrap();
    }
    connection
}

fn message_fixture(rows: &[(&str, &str)]) -> Vec<u8> {
    rmp_serde::to_vec(&MessageTable {
        rows_: rows
            .iter()
            .map(|(key, text)| MessageRow {
                column_: MessageColumn {
                    id_hash_: (*key).to_owned(),
                    subid_hash_: String::new(),
                    text_: (*text).to_owned(),
                },
            })
            .collect(),
    })
    .unwrap()
}
```

Add these tests:

```rust
#[test]
fn loads_symbolic_and_raw_item_keys_and_excludes_only_unnamed_rows() {
    let connection = sqlite_fixture(&[
        ("ITEM_80_0000", "TXT_ITEM_80_0000"),
        ("0EB683CD", "TXT_ITEM_90_0000"),
        ("INTERNAL_ROW", ""),
    ]);

    let definitions = load_item_definitions(&connection).unwrap();

    assert_eq!(definitions.len(), 2);
    let raw = definitions
        .iter()
        .find(|definition| definition.key == "0EB683CD")
        .unwrap();
    assert_eq!(raw.item_id, 0x0eb6_83cd);
    assert!(definitions
        .iter()
        .any(|definition| definition.key == "ITEM_80_0000"));
}

#[test]
fn joins_korean_and_english_names_and_serializes_deterministically() {
    let connection = sqlite_fixture(&[("0EB683CD", "TXT_ITEM_90_0000")]);
    let ko = message_fixture(&[("TXT_ITEM_90_0000", "테스트 아이템")]);
    let en = message_fixture(&[("TXT_ITEM_90_0000", "Test Item")]);

    let first = generate_catalogs(&connection, &ko, &en, GAME_EXE_SHA256).unwrap();
    let second = generate_catalogs(&connection, &ko, &en, GAME_EXE_SHA256).unwrap();

    assert_eq!(first.ko_names, second.ko_names);
    assert_eq!(first.en_names, second.en_names);
    let ko_catalog: ItemNameCatalog = serde_json::from_slice(&first.ko_names).unwrap();
    let en_catalog: ItemNameCatalog = serde_json::from_slice(&first.en_names).unwrap();
    assert_eq!(ko_catalog["0eb683cd"].key, "0EB683CD");
    assert_eq!(ko_catalog["0eb683cd"].text, "테스트 아이템");
    assert_eq!(en_catalog["0eb683cd"].text, "Test Item");
}

#[test]
fn rejects_missing_language_empty_text_duplicate_ids_and_wrong_game_hash() {
    let connection = sqlite_fixture(&[("0EB683CD", "TXT_ITEM_90_0000")]);
    let ko = message_fixture(&[("TXT_ITEM_90_0000", "테스트 아이템")]);
    let missing_en = message_fixture(&[]);
    assert!(generate_catalogs(&connection, &ko, &missing_en, GAME_EXE_SHA256).is_err());

    let empty_en = message_fixture(&[("TXT_ITEM_90_0000", " ")]);
    assert!(generate_catalogs(&connection, &ko, &empty_en, GAME_EXE_SHA256).is_err());

    let duplicate = sqlite_fixture(&[
        ("0EB683CD", "TXT_ITEM_90_0000"),
        ("0eb683cd", "TXT_ITEM_90_0001"),
    ]);
    assert!(load_item_definitions(&duplicate).is_err());

    let en = message_fixture(&[("TXT_ITEM_90_0000", "Test Item")]);
    assert!(generate_catalogs(&connection, &ko, &en, &"AB".repeat(32)).is_err());
}
```

- [ ] **Step 2: Run focused tests to verify RED**

Run:

```powershell
cargo test --locked --package gbfr-logs --example build_item_catalog
```

Expected: FAIL because `load_item_definitions` and `generate_catalogs` are not implemented.

- [ ] **Step 3: Implement item-key parsing and table loading**

Implement:

```rust
fn parse_item_id(key: &str) -> Result<u32> {
    let key = key.trim();
    let hexadecimal = key
        .strip_prefix("0x")
        .or_else(|| key.strip_prefix("0X"))
        .unwrap_or(key);
    if hexadecimal.len() == 8 && hexadecimal.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Ok(u32::from_str_radix(hexadecimal, 16)?);
    }
    if key.is_ascii() && key.starts_with("ITEM_") {
        return Ok(custom_xxhash32(key.as_bytes()));
    }
    bail!("item key {key:?} is neither ITEM_ text nor an eight-digit hash")
}

fn find_item_table(connection: &Connection) -> Result<String> {
    let tables = connection
        .prepare(
            "SELECT name FROM sqlite_master
             WHERE type = 'table' AND lower(name) = 'item'
             ORDER BY name",
        )?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    match tables.as_slice() {
        [table] => Ok(table.clone()),
        [] => bail!("SQLite contains no exact item table"),
        _ => bail!("SQLite contains multiple exact item tables"),
    }
}

fn load_item_definitions(connection: &Connection) -> Result<Vec<ItemDefinition>> {
    let table = find_item_table(connection)?.replace('"', "\"\"");
    let mut statement = connection.prepare(&format!(
        "SELECT Key, ItemName FROM \"{table}\" ORDER BY Key"
    ))?;
    let rows = statement
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut definitions = Vec::new();
    let mut item_ids = HashSet::new();
    for (key, name_key) in rows {
        let key = key.trim().to_owned();
        let name_key = name_key.trim().to_owned();
        if name_key.is_empty() {
            continue;
        }
        if key.is_empty() {
            bail!("localized item row has an empty key");
        }
        let item_id = parse_item_id(&key)?;
        if !item_ids.insert(item_id) {
            bail!("duplicate item ID {item_id:#010x}");
        }
        definitions.push(ItemDefinition {
            item_id,
            key,
            name_key,
        });
    }
    definitions.sort_by_key(|definition| definition.item_id);
    Ok(definitions)
}
```

Copy the complete `custom_xxhash32` implementation and its five prime constants from
`src-tauri/examples/build_trait_caps.rs`, preserving the seed and wrapping arithmetic exactly.
Add this independent known-vector assertion:

```rust
assert_eq!(custom_xxhash32(b"ITEM_31_0090"), 0x000e_b8f5);
```

- [ ] **Step 4: Implement bilingual MessagePack joining and deterministic JSON**

Implement:

```rust
fn parse_message_names(bytes: &[u8], language: &str) -> Result<BTreeMap<String, String>> {
    let table: MessageTable =
        rmp_serde::from_slice(bytes).with_context(|| format!("parse {language} text.msg"))?;
    let mut names = BTreeMap::new();
    for row in table.rows_ {
        let key = row.column_.id_hash_.trim().to_owned();
        let text = row.column_.text_.trim().to_owned();
        if key.is_empty() {
            continue;
        }
        if let Some(previous) = names.insert(key.clone(), text.clone()) {
            if previous != text {
                bail!("{language} contains conflicting text for {key}");
            }
        }
    }
    Ok(names)
}

fn build_catalog(
    definitions: &[ItemDefinition],
    names: &BTreeMap<String, String>,
    language: &str,
) -> Result<ItemNameCatalog> {
    let mut catalog = BTreeMap::new();
    for definition in definitions {
        let text = names
            .get(&definition.name_key)
            .with_context(|| format!("{language} is missing {}", definition.name_key))?
            .trim();
        if text.is_empty() {
            bail!("{language} has an empty name for {}", definition.name_key);
        }
        let hash = format!("{:08x}", definition.item_id);
        let record = ItemNameRecord {
            key: definition.key.clone(),
            text: text.to_owned(),
        };
        if catalog.insert(hash.clone(), record).is_some() {
            bail!("{language} item hash collision at {hash}");
        }
    }
    Ok(catalog)
}

fn normalized_sha256(value: &str) -> Result<String> {
    let value = value.trim();
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("game executable SHA-256 must contain exactly 64 hexadecimal characters");
    }
    Ok(value.to_ascii_uppercase())
}

fn validated_game_sha256(value: &str) -> Result<String> {
    let normalized = normalized_sha256(value)?;
    if normalized != GAME_EXE_SHA256 {
        bail!("game executable SHA-256 does not match Granblue Fantasy: Relink 2.0.2");
    }
    Ok(normalized)
}

fn pretty_json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn generate_catalogs(
    connection: &Connection,
    ko_message: &[u8],
    en_message: &[u8],
    game_exe_sha256: &str,
) -> Result<GeneratedCatalogs> {
    validated_game_sha256(game_exe_sha256)?;
    let definitions = load_item_definitions(connection)?;
    let ko = build_catalog(
        &definitions,
        &parse_message_names(ko_message, "Korean")?,
        "Korean",
    )?;
    let en = build_catalog(
        &definitions,
        &parse_message_names(en_message, "English")?,
        "English",
    )?;
    if ko.keys().ne(en.keys())
        || ko.iter().any(|(hash, record)| en[hash].key != record.key)
    {
        bail!("Korean and English item catalogs differ");
    }
    Ok(GeneratedCatalogs {
        ko_names: pretty_json(&ko)?,
        en_names: pretty_json(&en)?,
        item_count: definitions.len(),
    })
}
```

Before returning from the production CLI, query `SELECT COUNT(*) FROM item`, calculate
`unnamed_count = total_rows - item_count`, and require all three pinned counts:

```rust
if total_rows != EXPECTED_ITEM_ROWS
    || generated.item_count != EXPECTED_LOCALIZED_ITEMS
    || unnamed_count != EXPECTED_UNNAMED_ROWS
{
    bail!(
        "unexpected 2.0.2 item counts: total={total_rows}, localized={}, unnamed={unnamed_count}",
        generated.item_count
    );
}
```

- [ ] **Step 5: Implement two-file staging and the CLI**

Use the existing trait generator’s stage-then-replace pattern, but accept exactly two files:

```rust
fn staged_path(destination: &Path) -> Result<PathBuf> {
    let mut name = destination
        .file_name()
        .context("output path has no file name")?
        .to_os_string();
    name.push(".djeeta-stage");
    Ok(destination.with_file_name(name))
}

fn write_prepared_outputs(outputs: [(&Path, &[u8]); 2]) -> Result<()> {
    let staged = outputs
        .iter()
        .map(|(destination, _)| staged_path(destination))
        .collect::<Result<Vec<_>>>()?;
    for ((destination, bytes), stage) in outputs.iter().zip(&staged) {
        fs::write(stage, *bytes)
            .with_context(|| format!("failed to stage {}", destination.display()))?;
    }
    for ((destination, _), stage) in outputs.iter().zip(&staged) {
        fs::copy(stage, destination)
            .with_context(|| format!("failed to replace {}", destination.display()))?;
        fs::remove_file(stage)?;
    }
    Ok(())
}
```

The CLI must open SQLite read-only and validate everything before writing:

```rust
fn main() -> Result<()> {
    let arguments: Vec<_> = env::args_os().skip(1).collect();
    if arguments.len() != 6 {
        bail!(
            "usage: build_item_catalog <input.sqlite> <ko-text.msg> <en-text.msg> \
             <ko-items.json> <en-items.json> <game-exe-sha256>"
        );
    }
    let sqlite_path = Path::new(&arguments[0]);
    let ko_message_path = Path::new(&arguments[1]);
    let en_message_path = Path::new(&arguments[2]);
    let ko_output_path = Path::new(&arguments[3]);
    let en_output_path = Path::new(&arguments[4]);
    let game_hash = arguments[5]
        .to_str()
        .context("game executable SHA-256 must be valid Unicode")?;

    let connection = Connection::open_with_flags(sqlite_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let ko_message = fs::read(ko_message_path)?;
    let en_message = fs::read(en_message_path)?;
    let generated = generate_catalogs(&connection, &ko_message, &en_message, game_hash)?;
    let total_rows: usize = connection
        .query_row("SELECT COUNT(*) FROM item", [], |row| row.get(0))
        .context("count item rows")?;
    let unnamed_count = total_rows.saturating_sub(generated.item_count);
    if total_rows != EXPECTED_ITEM_ROWS
        || generated.item_count != EXPECTED_LOCALIZED_ITEMS
        || unnamed_count != EXPECTED_UNNAMED_ROWS
    {
        bail!("unexpected 2.0.2 item counts");
    }

    write_prepared_outputs([
        (ko_output_path, &generated.ko_names),
        (en_output_path, &generated.en_names),
    ])?;
    println!(
        "wrote {} localized items per language for game {}; excluded {} unnamed rows",
        generated.item_count, GAME_VERSION, unnamed_count
    );
    Ok(())
}
```

Add a `writes_both_prepared_outputs` test using a unique directory under
`std::env::temp_dir()`, assert both bytes, and remove only that exact directory afterward.

- [ ] **Step 6: Run focused tests to verify GREEN**

Run:

```powershell
cargo test --locked --package gbfr-logs --example build_item_catalog
cargo clippy --locked --package gbfr-logs --example build_item_catalog -- -D warnings
```

Expected: all generator tests PASS and Clippy exits 0.

- [ ] **Step 7: Commit the generator**

```powershell
git add -- src-tauri/Cargo.toml src-tauri/examples/build_item_catalog.rs
git commit -m "feat: add 2.0.2 item catalog generator"
```

---

### Task 2: Generate and Lock the 2.0.2 Korean and English Catalogs

**Files:**

- Modify: `src-tauri/examples/build_item_catalog.rs`
- Replace: `src-tauri/lang/ko/items.json`
- Replace: `src-tauri/lang/en/items.json`
- Create: `docs/research/2026-07-24-item-catalog-2.0.2.md`

**Interfaces:**

- Consumes: the Task 1 CLI and verified local extraction artifacts.
- Produces: two 384-entry catalogs and bundled regression tests that cover all 281 ordinary-item IDs.

- [ ] **Step 1: Add failing bundled-catalog tests**

At the bottom of `build_item_catalog.rs`, deserialize the bundled files and the ordinary-item
catalog:

```rust
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrdinaryItemCatalog {
    game_version: String,
    game_exe_sha256: String,
    item_ids: Vec<String>,
}

#[test]
fn bundled_2_0_2_catalogs_are_complete_and_cover_all_ordinary_items() {
    let ko: ItemNameCatalog =
        serde_json::from_str(include_str!("../lang/ko/items.json")).unwrap();
    let en: ItemNameCatalog =
        serde_json::from_str(include_str!("../lang/en/items.json")).unwrap();
    let ordinary: OrdinaryItemCatalog =
        serde_json::from_str(include_str!("../data/ordinary-items-2.0.2.json")).unwrap();

    assert_eq!(ordinary.game_version, GAME_VERSION);
    assert_eq!(ordinary.game_exe_sha256, GAME_EXE_SHA256);
    assert_eq!(ko.len(), EXPECTED_LOCALIZED_ITEMS);
    assert_eq!(en.len(), EXPECTED_LOCALIZED_ITEMS);
    assert_eq!(ko.keys().collect::<Vec<_>>(), en.keys().collect::<Vec<_>>());
    assert!(ko.values().all(|record| !record.key.trim().is_empty() && !record.text.trim().is_empty()));
    assert!(en.values().all(|record| !record.key.trim().is_empty() && !record.text.trim().is_empty()));
    assert!(ko.iter().all(|(hash, record)| en[hash].key == record.key));
    assert_eq!(ordinary.item_ids.len(), 281);
    assert!(ordinary
        .item_ids
        .iter()
        .all(|hash| ko.contains_key(&hash.to_ascii_lowercase())));
}

#[test]
fn bundled_catalog_contains_endless_ragnarok_items() {
    let ko: ItemNameCatalog =
        serde_json::from_str(include_str!("../lang/ko/items.json")).unwrap();
    let en: ItemNameCatalog =
        serde_json::from_str(include_str!("../lang/en/items.json")).unwrap();

    for hash in ["0eb683cd", "20c742de", "98cdb46f"] {
        assert!(ko.contains_key(hash), "missing Korean item {hash}");
        assert!(en.contains_key(hash), "missing English item {hash}");
    }
}
```

- [ ] **Step 2: Run the bundled test to verify RED**

Run:

```powershell
cargo test --locked --package gbfr-logs --example build_item_catalog bundled_2_0_2_catalog
```

Expected: FAIL because the current 1.3-era catalogs contain 289 entries and omit 2.0.2 IDs.

- [ ] **Step 3: Verify extraction provenance**

Use these already extracted read-only artifacts if their hashes still match:

```text
GBFRDataTools-2.0.0-win-x64.zip
SHA-256 2F355E7785D7ED7D1A4F99B1FCCC626BB9D949CE29A4F08B816A233DAB77F63B

system/table/item.tbl
SHA-256 99D6450E0908F13F3D1E8A76E7A3E66A3E16505D24858F6D0E4D2B6FBD7FA9D0

system/table/text/ko/text.msg
SHA-256 E03EF29EAC56BB6EAA32EE48D848887F073806F8514A52D31D38F2D9F0397090

system/table/text/en/text.msg
SHA-256 0230DC9A2E42B97C2BFC7B9B6DD074A43F82A661BAA23C69EE8F4A3DA3D0096D
```

Verify with `Get-FileHash`. If any input differs, stop and re-extract exactly:

```powershell
& $dataTool extract `
  -i 'D:\SteamLibrary\steamapps\common\Granblue Fantasy Relink\data.i' `
  -f 'system/table/item.tbl' `
  -o $extractedRoot
& $dataTool extract `
  -i 'D:\SteamLibrary\steamapps\common\Granblue Fantasy Relink\data.i' `
  -f 'system/table/text/ko/text.msg' `
  -o $extractedRoot
& $dataTool extract `
  -i 'D:\SteamLibrary\steamapps\common\Granblue Fantasy Relink\data.i' `
  -f 'system/table/text/en/text.msg' `
  -o $extractedRoot
& $dataTool tbl-to-sqlite `
  -i (Join-Path $extractedRoot 'system\table') `
  -o $sqlitePath `
  -v 2.0.2
```

Expected SQLite evidence:

```text
item rows: 448
non-empty ItemName rows: 384
empty ItemName rows: 64
Korean missing names: 0
English missing names: 0
```

- [ ] **Step 4: Generate both catalogs in one validated run**

Run with the exact verified paths:

```powershell
cargo run --locked --release --package gbfr-logs --example build_item_catalog -- `
  $sqlitePath `
  (Join-Path $extractedRoot 'system\table\text\ko\text.msg') `
  (Join-Path $extractedRoot 'system\table\text\en\text.msg') `
  'src-tauri\lang\ko\items.json' `
  'src-tauri\lang\en\items.json' `
  '63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F'
```

Expected:

```text
wrote 384 localized items per language for game 2.0.2; excluded 64 unnamed rows
```

- [ ] **Step 5: Run generated-catalog tests to verify GREEN**

Run:

```powershell
cargo test --locked --package gbfr-logs --example build_item_catalog
npm.cmd test -- --run src/pages/ItemAnalysis.localization.test.ts src/pages/ItemAnalysis.test.tsx
```

Expected: generator tests and both item-analysis frontend tests PASS.

- [ ] **Step 6: Document reproducible provenance**

Create `docs/research/2026-07-24-item-catalog-2.0.2.md` with:

```markdown
# Granblue Fantasy: Relink 2.0.2 item-name catalog

- Game executable SHA-256: `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F`
- Converter: GBFRDataTools 2.0.0
- Converter archive SHA-256: `2F355E7785D7ED7D1A4F99B1FCCC626BB9D949CE29A4F08B816A233DAB77F63B`
- Table conversion version: `2.0.2`
- Source rows: 448
- Localized output rows per language: 384
- Excluded unnamed internal rows: 64
- Missing Korean/English names: 0 / 0

The source game assets and converter binaries are not committed. Re-run the exact
read-only extraction and generator commands from this document after a supported game
version change. Catalog generation does not establish runtime compatibility.
```

Include the exact extraction, hash-verification, conversion, and generator commands from
Steps 3–4. Do not include temporary absolute paths or extracted content.

- [ ] **Step 7: Commit generated catalogs and provenance together**

```powershell
git add -- src-tauri/examples/build_item_catalog.rs src-tauri/lang/ko/items.json src-tauri/lang/en/items.json docs/research/2026-07-24-item-catalog-2.0.2.md
git commit -m "feat: refresh item names from game 2.0.2"
```

---

### Task 3: Record Remaining Item-Analysis Work

**Files:**

- Modify: `docs/testing/game-2.0.2-item-analysis-probe.md`

**Interfaces:**

- Consumes: Task 2’s passing catalog evidence.
- Produces: one authoritative `Remaining tasks` checklist with automatic work separated from manual game work.

- [ ] **Step 1: Add the checklist**

Append:

```markdown
## Remaining tasks

- [x] Regenerate the complete Korean and English item-name catalogs from pinned 2.0.2 assets.
- [x] Verify all 281 ordinary-item IDs have Korean and English names.
- [ ] Observe a controlled +1 change affecting only the selected item.
- [ ] Observe a controlled decrease affecting only the selected item.
- [ ] Validate live warning boundaries at 899, 900, and 999.
- [ ] Change the in-game item-menu sort and filter and confirm the logical snapshot is unchanged.
- [ ] Restart the game and resolve the same logical inventory (1/3).
- [ ] Restart the game and resolve the same logical inventory (2/3).
- [ ] Restart the game and resolve the same logical inventory (3/3).
- [ ] Run the final required frontend, Rust, and build verification after all manual evidence is complete.
```

Do not modify the existing result cells unless new live evidence was personally observed.

- [ ] **Step 2: Verify checklist truthfulness**

Run:

```powershell
rg -n "Remaining tasks|281 ordinary|controlled|899|900|999|1/3|2/3|3/3" `
  docs/testing/game-2.0.2-item-analysis-probe.md
git diff --check -- docs/testing/game-2.0.2-item-analysis-probe.md
```

Expected: every agreed task appears once, only the catalog tasks are checked, and diff check exits 0.

- [ ] **Step 3: Commit the task record**

```powershell
git add -- docs/testing/game-2.0.2-item-analysis-probe.md
git commit -m "docs: track remaining item analysis validation"
```

---

### Task 4: Run Required Regression and Build Verification

**Files:**

- Verify only; modify files only if a failure directly caused by this work requires a focused fix.

**Interfaces:**

- Consumes: Tasks 1–3.
- Produces: final automated evidence for the catalog refresh without changing manual checklist results.

- [ ] **Step 1: Run narrow regressions**

```powershell
cargo test --locked --package gbfr-logs --example build_item_catalog
npm.cmd test -- --run src/pages/ItemAnalysis.localization.test.ts src/pages/ItemAnalysis.test.tsx src/itemAnalysisContract.test.ts
```

Expected: all focused tests PASS.

- [ ] **Step 2: Run required frontend verification**

```powershell
npm.cmd run format-check
npm.cmd run lint
npm.cmd run tsc
npm.cmd test -- --run
npm.cmd run build
```

Expected: all commands exit 0. Existing intentional ErrorBoundary stderr does not count as failure when Vitest exits 0.

- [ ] **Step 3: Run required Rust verification**

```powershell
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
```

Expected: both commands exit 0. Existing dead-code and `used_linker` warnings may remain but no new generator warning is allowed.

- [ ] **Step 4: Inspect exact final scope**

```powershell
git diff --check
git status --short
git log -5 --oneline
```

Expected:

- no staged or unstaged catalog-feature changes remain after the task commits;
- unrelated `AGENTS.md`, the pre-existing research index, and `logs.db` remain untouched;
- no game asset, SQLite file, extraction binary, or temporary stage file is tracked.

- [ ] **Step 5: Report automated completion and manual remainder**

Report:

- Korean/English catalog entry count: 384 each;
- unnamed internal rows excluded: 64;
- ordinary-item translation coverage: 281/281 each language;
- exact focused and full test/build results;
- unchecked manual quantity, boundary, sort/filter, and restart tasks;
- no claim of full 2.0.2 compatibility.

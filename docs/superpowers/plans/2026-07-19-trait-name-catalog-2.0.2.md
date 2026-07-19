# Granblue Fantasy: Relink 2.0.2 Trait Name Catalog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the stale 1.3.x equipment-trait name catalogs with verified Korean and English names from Granblue Fantasy: Relink Endless Ragnarok 2.0.2, while ensuring every unresolved trait is displayed with its hexadecimal ID.

**Architecture:** Extend the existing Rust `build_trait_caps` generator so one validated run joins `skill_status.tbl` with the Korean and English `text.msg` tables and prepares all three generated catalogs. Keep runtime lookup unchanged for known traits; change only the final UI fallback so an unresolved hash remains actionable. This stage deliberately continues to analyze the existing 12 equipped sigils only—weapon, wrightstone, summon, and master-trait contributions remain later roadmap stages.

**Tech Stack:** Rust, rusqlite, serde, serde_json, rmp-serde, Tauri 1, React, TypeScript, i18next, Vitest, PowerShell, GBFRDataTools 2.0.0.

## Global Constraints

- Work on branch `codex/trait-name-catalog-2.0.2`; do not switch to `master`.
- Do not stage or modify the unrelated untracked `logs.db`.
- Follow RED → GREEN → REFACTOR for every behavioral change.
- Preserve existing bincode protocol variant ordering; this work requires no protocol change.
- Do not claim game 2.0.2 compatibility. Only the catalog provenance is verified until `docs/testing/game-2.0.2-smoke-test.md` is completed in game.
- Treat the game files and GBFRDataTools archive as read-only inputs. Extract only to a fresh temporary directory.
- Before reporting completion, run every command in the repository's Required verification section and rebuild the MSI.

---

### Task 1: Parse 2.0.2 localized trait-name tables

**Files:**

- Modify: `src-tauri/Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `src-tauri/src/bin/build_trait_caps.rs`

- [ ] **Step 1: Add the MessagePack dependency**

Add this dependency beside the existing serde dependencies in `src-tauri/Cargo.toml`:

```toml
rmp = "=0.8.14"
rmp-serde = "=1.3.0"
```

Resolve and lock it without updating unrelated packages:

```powershell
cargo check --package gbfr-logs --bin build_trait_caps
git diff -- Cargo.lock
```

Expected: Cargo adds the toolchain-compatible `rmp-serde 1.3.0`, `rmp 0.8.14`, and their required MessagePack dependencies to `Cargo.lock`, the existing generator still compiles, and no unrelated locked package changes version. The exact pins are required because their next patch releases use an Edition 2024 manifest that the repository's pinned 2024-05 Cargo cannot parse.

- [ ] **Step 2: Write failing tests for MessagePack parsing and catalog joining**

At the bottom of the existing `tests` module in `src-tauri/src/bin/build_trait_caps.rs`, add fixtures and tests equivalent to:

Extend the test module imports with `std::collections::BTreeMap` and the new private production items used below (`build_name_catalog`, `parse_message_names`, and `TraitDefinition`).

```rust
fn message_fixture(rows: &[(&str, &str)]) -> Vec<u8> {
    let rows = rows
        .iter()
        .map(|(id_hash, text)| {
            serde_json::json!({
                "column_": {
                    "id_hash_": id_hash,
                    "subid_hash_": "",
                    "text_": text,
                }
            })
        })
        .collect::<Vec<_>>();

    rmp_serde::to_vec_named(&serde_json::json!({ "rows_": rows })).unwrap()
}

#[test]
fn parses_skill_names_from_messagepack() {
    let bytes = message_fixture(&[
        ("TXT_SKILL_020_00", "대미지 상한"),
        ("TXT_OTHER", "무시"),
    ]);

    let names = parse_message_names(&bytes).unwrap();

    assert_eq!(names.get("SKILL_020_00"), Some(&"대미지 상한".to_owned()));
    assert!(!names.contains_key("OTHER"));
}

#[test]
fn joins_only_symbolic_trait_keys_with_localized_names() {
    let definitions = vec![
        TraitDefinition::symbolic("SKILL_020_00", custom_xxhash32(b"SKILL_020_00"), 65),
        TraitDefinition::raw(0x0151_cf9e, 30),
    ];
    let names = BTreeMap::from([("SKILL_020_00".to_owned(), "Damage Cap".to_owned())]);

    let catalog = build_name_catalog(&definitions, &names, "English").unwrap();

    assert_eq!(catalog.len(), 1);
    assert_eq!(catalog[&format!("{:08x}", custom_xxhash32(b"SKILL_020_00"))].key, "SKILL_020_00");
    assert_eq!(catalog[&format!("{:08x}", custom_xxhash32(b"SKILL_020_00"))].text, "Damage Cap");
    assert!(!catalog.contains_key("0151cf9e"));
}

#[test]
fn omits_a_symbolic_trait_missing_from_a_language_table() {
    let definitions = vec![TraitDefinition::symbolic(
        "SKILL_020_00",
        custom_xxhash32(b"SKILL_020_00"),
        65,
    )];

    let catalog = build_name_catalog(&definitions, &BTreeMap::new(), "Korean").unwrap();

    assert!(catalog.is_empty());
}

#[test]
fn rejects_empty_names_and_trait_hash_collisions() {
    let empty = message_fixture(&[("TXT_SKILL_020_00", "   ")]);
    assert!(parse_message_names(&empty).unwrap_err().to_string().contains("empty"));

    let definitions = vec![
        TraitDefinition::symbolic("SKILL_020_00", 7, 65),
        TraitDefinition::symbolic("SKILL_173_00", 7, 30),
    ];
    let names = BTreeMap::from([
        ("SKILL_020_00".to_owned(), "Damage Cap".to_owned()),
        ("SKILL_173_00".to_owned(), "Gladiator's Frenzy".to_owned()),
    ]);

    assert!(build_name_catalog(&definitions, &names, "English")
        .unwrap_err()
        .to_string()
        .contains("collision"));
}
```

Expose `TraitDefinition::symbolic` and `TraitDefinition::raw` as private test-friendly constructors rather than public API.

- [ ] **Step 3: Run the focused test and verify RED**

```powershell
cargo test --locked --package gbfr-logs --bin build_trait_caps parses_skill_names_from_messagepack
```

Expected: compilation fails because `parse_message_names`, `TraitDefinition`, and `build_name_catalog` do not exist yet.

- [ ] **Step 4: Implement the minimum parser and join model**

In `src-tauri/src/bin/build_trait_caps.rs`:

1. Extend imports with `BTreeMap` and `serde::Deserialize`; keep the existing `anyhow::{bail, Context}` imports.
2. Replace the cap-only query model with a definition that retains the source key.
3. Deserialize only the verified `text.msg` shape.
4. Include only `TXT_SKILL_...` rows.
5. Omit a symbolic key with no verified localized text, and reject empty text, conflicting duplicate message rows, and hash collisions.

Use these shapes and behavior:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
enum TraitKey {
    Symbolic(String),
    RawHash,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TraitDefinition {
    key: TraitKey,
    trait_id: u32,
    max_level: u32,
}

impl TraitDefinition {
    fn symbolic(key: &str, trait_id: u32, max_level: u32) -> Self {
        Self {
            key: TraitKey::Symbolic(key.to_owned()),
            trait_id,
            max_level,
        }
    }

    fn raw(trait_id: u32, max_level: u32) -> Self {
        Self {
            key: TraitKey::RawHash,
            trait_id,
            max_level,
        }
    }
}

#[derive(Debug, Deserialize)]
struct MessageTable {
    rows_: Vec<MessageRow>,
}

#[derive(Debug, Deserialize)]
struct MessageRow {
    column_: MessageColumn,
}

#[derive(Debug, Deserialize)]
struct MessageColumn {
    id_hash_: String,
    subid_hash_: String,
    text_: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct TraitNameRecord {
    key: String,
    text: String,
}

type TraitNameCatalog = BTreeMap<String, TraitNameRecord>;

fn parse_message_names(bytes: &[u8]) -> anyhow::Result<BTreeMap<String, String>> {
    let table: MessageTable = rmp_serde::from_slice(bytes).context("parse text.msg MessagePack")?;
    let mut names = BTreeMap::new();

    for row in table.rows_ {
        let Some(key) = row.column_.id_hash_.strip_prefix("TXT_SKILL_") else {
            continue;
        };
        let key = format!("SKILL_{key}");
        let text = row.column_.text_.trim();
        if text.is_empty() {
            bail!("empty localized name for {key}");
        }
        if let Some(previous) = names.insert(key.clone(), text.to_owned()) {
            if previous != text {
                bail!("conflicting localized names for {key}");
            }
        }
    }

    Ok(names)
}

fn build_name_catalog(
    definitions: &[TraitDefinition],
    names: &BTreeMap<String, String>,
    language: &str,
) -> anyhow::Result<TraitNameCatalog> {
    let mut catalog = BTreeMap::new();

    for definition in definitions {
        let TraitKey::Symbolic(key) = &definition.key else {
            continue;
        };
        let Some(text) = names.get(key) else {
            continue;
        };
        let hash = format!("{:08x}", definition.trait_id);
        let record = TraitNameRecord {
            key: key.clone(),
            text: text.clone(),
        };
        if let Some(previous) = catalog.insert(hash.clone(), record.clone()) {
            if previous != record {
                bail!("trait hash collision at {hash}");
            }
        }
    }

    Ok(catalog)
}
```

Read `subid_hash_` into the schema for format fidelity, then explicitly bind it to `_` in the loop so lint does not report an unread field. Do not merge sub-ID variants in this stage.

Refactor the existing SQLite row loader to return `Vec<TraitDefinition>`. For text keys:

- `SKILL_...` becomes `TraitKey::Symbolic` and is hashed with the existing `custom_xxhash32`.
- Exactly eight hexadecimal characters become `TraitKey::RawHash` and retain their numeric value in `trait_id`.
- Integer SQLite keys remain raw IDs for backward-compatible tests.
- Any other string is rejected instead of silently entering a generated catalog.

Derive the existing `TraitCapRecord` list from those definitions, preserving ascending `trait_id` ordering and the existing max-level aggregation.

- [ ] **Step 5: Run all generator tests and verify GREEN**

```powershell
cargo test --locked --package gbfr-logs --bin build_trait_caps
```

Expected: all existing cap/hash tests and the three new localization tests pass.

- [ ] **Step 6: Commit the parser unit**

```powershell
git add src-tauri/Cargo.toml Cargo.lock src-tauri/src/bin/build_trait_caps.rs
git commit -m "feat: parse localized trait names"
```

---

### Task 2: Generate cap, Korean, and English catalogs as one validated operation

**Files:**

- Modify: `src-tauri/src/bin/build_trait_caps.rs`

- [ ] **Step 1: Write failing tests for complete output preparation**

Add tests that create an in-memory `skill_status` table with one symbolic and one raw trait, then verify all output bytes are prepared together:

Extend the test module's `super` import with `generate_catalogs`, `validated_game_sha256`, `TraitCapCatalog`, `TraitNameCatalog`, and `GAME_EXE_SHA256`.

```rust
#[test]
fn prepares_caps_and_both_language_catalogs_together() {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch(
            "CREATE TABLE skill_status (Key TEXT, Level INTEGER, MAX INTEGER);\n\
             INSERT INTO skill_status VALUES ('SKILL_020_00', 65, 65);\n\
             INSERT INTO skill_status VALUES ('0151cf9e', 1, 30);",
        )
        .unwrap();
    let ko = message_fixture(&[("TXT_SKILL_020_00", "대미지 상한")]);
    let en = message_fixture(&[("TXT_SKILL_020_00", "Damage Cap")]);

    let generated = generate_catalogs(&connection, &ko, &en, GAME_EXE_SHA256).unwrap();

    let caps: TraitCapCatalog = serde_json::from_slice(&generated.caps).unwrap();
    let ko_names: TraitNameCatalog = serde_json::from_slice(&generated.ko_names).unwrap();
    let en_names: TraitNameCatalog = serde_json::from_slice(&generated.en_names).unwrap();
    assert_eq!(caps.records.len(), 2);
    assert_eq!(
        caps.records
            .iter()
            .find(|record| record.trait_id == custom_xxhash32(b"SKILL_020_00"))
            .unwrap()
            .max_level,
        65,
    );
    assert_eq!(ko_names.len(), 1);
    assert_eq!(ko_names.keys().collect::<Vec<_>>(), en_names.keys().collect::<Vec<_>>());
}

#[test]
fn refuses_all_outputs_when_one_language_is_incomplete() {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch(
            "CREATE TABLE skill_status (Key TEXT, Level INTEGER, MAX INTEGER);\n\
             INSERT INTO skill_status VALUES ('SKILL_020_00', 1, 65);",
        )
        .unwrap();

    let error = generate_catalogs(
        &connection,
        &message_fixture(&[("TXT_SKILL_020_00", "대미지 상한")]),
        &message_fixture(&[]),
        GAME_EXE_SHA256,
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("English"));
}

#[test]
fn rejects_a_well_formed_but_different_game_executable_hash() {
    assert!(validated_game_sha256(&"AB".repeat(32))
        .unwrap_err()
        .to_string()
        .contains("2.0.2"));
}
```

Add `Deserialize` to `TraitCapCatalog`, `TraitCapRecord`, and `TraitNameRecord` so the test reads the prepared bytes through the production schema. Change `TraitCapCatalog.game_version` from `&'static str` to `String`, then construct it with `GAME_VERSION.to_owned()`; this avoids an invalid deserialization lifetime while leaving the JSON schema unchanged.

- [ ] **Step 2: Run the new test and verify RED**

```powershell
cargo test --locked --package gbfr-logs --bin build_trait_caps prepares_caps_and_both_language_catalogs_together
```

Expected: compilation fails because `GeneratedCatalogs` and `generate_catalogs` do not exist.

- [ ] **Step 3: Implement validation-before-write output preparation**

Add `fs` and `PathBuf` to the standard-library imports. Once `write_catalog` is replaced, remove the no-longer-used `File` and `io::Write` imports. Then add:

```rust
const GAME_EXE_SHA256: &str =
    "63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F";

#[derive(Debug)]
struct GeneratedCatalogs {
    caps: Vec<u8>,
    ko_names: Vec<u8>,
    en_names: Vec<u8>,
    cap_count: usize,
    name_count: usize,
}

fn pretty_json<T: serde::Serialize>(value: &T) -> anyhow::Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn validated_game_sha256(value: &str) -> anyhow::Result<String> {
    let normalized = normalized_sha256(value)?;
    if normalized != GAME_EXE_SHA256 {
        bail!("game executable SHA-256 does not match Granblue Fantasy: Relink 2.0.2");
    }
    Ok(normalized)
}

fn generate_catalogs(
    connection: &Connection,
    ko_message: &[u8],
    en_message: &[u8],
    game_exe_sha256: &str,
) -> anyhow::Result<GeneratedCatalogs> {
    let definitions = load_definitions(connection)?;
    let records = build_cap_records(&definitions);
    let ko_names = build_name_catalog(
        &definitions,
        &parse_message_names(ko_message)?,
        "Korean",
    )?;
    let en_names = build_name_catalog(
        &definitions,
        &parse_message_names(en_message)?,
        "English",
    )?;

    if ko_names.keys().ne(en_names.keys()) {
        bail!("Korean and English trait-name catalogs have different keys");
    }

    let cap_count = records.len();
    let name_count = ko_names.len();

    let caps = TraitCapCatalog {
        game_version: GAME_VERSION.to_owned(),
        game_exe_sha256: validated_game_sha256(game_exe_sha256)?,
        records,
    };

    Ok(GeneratedCatalogs {
        caps: pretty_json(&caps)?,
        ko_names: pretty_json(&ko_names)?,
        en_names: pretty_json(&en_names)?,
        cap_count,
        name_count,
    })
}
```

Keep `game_version` and hash ownership semantics identical to the current cap generator.

- [ ] **Step 4: Expand the CLI and defer every repository write until validation succeeds**

Change the CLI contract to:

```text
build_trait_caps input.sqlite ko-text.msg en-text.msg trait-caps.json ko-traits.json en-traits.json game-exe-sha256
```

Implement `main` so it:

1. Validates the seven positional arguments before opening files.
2. Reads both `.msg` inputs completely.
3. Opens the SQLite database read-only.
4. Calls `generate_catalogs` once.
5. Only after success, writes each prepared byte buffer to a sibling temporary file.
6. Replaces the three destination files from those staged files.
7. Leaves still-staged files in place with a destination-specific error if a filesystem replacement fails.
8. Prints cap-record count, localized-name count, and all destination paths.

Use a destination-specific staged name such as `traits.json.djeeta-stage`; never stage under the repository root. On Windows, if the destination exists, use `std::fs::copy` from the validated staged file followed by removal of that staged file; do not delete the destination first. The guarantee is specifically that argument, parsing, collision, and cross-language key-set validation failures occur before any checked-in output changes. If a filesystem write itself fails, report the destination that failed and retain any remaining staged files for diagnosis.

Implement the staging helper as:

```rust
fn staged_path(destination: &Path) -> anyhow::Result<PathBuf> {
    let mut file_name = destination
        .file_name()
        .context("output path has no file name")?
        .to_os_string();
    file_name.push(".djeeta-stage");
    Ok(destination.with_file_name(file_name))
}

fn write_prepared_outputs(outputs: [(&Path, &[u8]); 3]) -> anyhow::Result<()> {
    let staged = outputs
        .iter()
        .map(|(destination, _)| staged_path(destination))
        .collect::<anyhow::Result<Vec<_>>>()?;

    for ((destination, bytes), staged_path) in outputs.iter().zip(&staged) {
        fs::write(staged_path, *bytes)
            .with_context(|| format!("failed to stage {}", destination.display()))?;
    }

    for ((destination, _), staged_path) in outputs.iter().zip(&staged) {
        fs::copy(staged_path, destination)
            .with_context(|| format!("failed to replace {}", destination.display()))?;
        fs::remove_file(staged_path)?;
    }

    Ok(())
}
```

Call it only after `generate_catalogs` returns successfully:

```rust
write_prepared_outputs([
    (cap_output_path, &generated.caps),
    (ko_output_path, &generated.ko_names),
    (en_output_path, &generated.en_names),
])?;
println!(
    "wrote {} trait cap records and {} localized names per language",
    generated.cap_count, generated.name_count
);
```

- [ ] **Step 5: Run focused and complete generator tests**

```powershell
cargo test --locked --package gbfr-logs --bin build_trait_caps prepares_caps_and_both_language_catalogs_together
cargo test --locked --package gbfr-logs --bin build_trait_caps
```

Expected: both commands pass.

- [ ] **Step 6: Verify formatting and lint for the Rust unit**

```powershell
cargo fmt --all -- --check
cargo clippy --locked --package gbfr-logs --bin build_trait_caps -- -D warnings
```

Expected: both commands exit 0.

- [ ] **Step 7: Commit the generator transaction**

```powershell
git add src-tauri/src/bin/build_trait_caps.rs
git commit -m "build: generate localized trait catalogs"
```

---

### Task 3: Regenerate verified 2.0.2 catalogs and lock their coverage

**Files:**

- Modify: `src-tauri/lang/ko/traits.json`
- Modify: `src-tauri/lang/en/traits.json`
- Verify or modify only if generated bytes differ: `src-tauri/assets/trait-caps.json`
- Modify: `src-tauri/src/bin/build_trait_caps.rs`
- Modify: `docs/research/2026-07-18-gbfr-er-2.0.2-trait-overflow.md`

- [ ] **Step 1: Verify immutable source inputs**

Use a fresh temporary directory and verify the known source hashes before extraction:

```powershell
$catalogWork = Join-Path ([System.IO.Path]::GetTempPath()) ("djeeta-trait-names-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $catalogWork | Out-Null
$gameRoot = 'D:\SteamLibrary\steamapps\common\Granblue Fantasy Relink'
$toolRoot = 'C:\Users\azyu\AppData\Local\Temp\djeeta-trait-caps-2.0.2-602167ada7144302ba969405d07f9674'
$dataToolArchive = Join-Path $toolRoot 'GBFRDataTools-2.0.0-win-x64.zip'
$dataTool = Join-Path $toolRoot 'tool\win-x64\GBFRDataTools.exe'

(Get-FileHash -Algorithm SHA256 (Join-Path $gameRoot 'granblue_fantasy_relink.exe')).Hash
(Get-FileHash -Algorithm SHA256 $dataToolArchive).Hash
(Get-FileHash -Algorithm SHA256 $dataTool).Hash
```

Expected game EXE SHA-256:

```text
63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F
```

Expected GBFRDataTools 2.0.0 distribution ZIP SHA-256:

```text
2F355E7785D7ED7D1A4F99B1FCCC626BB9D949CE29A4F08B816A233DAB77F63B
```

Expected extracted `GBFRDataTools.exe` SHA-256:

```text
8693668FC4E35DCC44EC54D15B9AD1756A5E9A4E4C5E838A32C9245EE5E83257
```

Stop if any of the three hashes differs; do not regenerate catalogs from an unrecorded source.

- [ ] **Step 2: Extract the three source tables and convert only `skill_status.tbl` to SQLite**

Run the tool's verified `extract` command three times against the installed game's data index, preserving each game-relative output path under `$catalogWork`. Then convert `skill_status.tbl` to SQLite:

```powershell
$tables = Join-Path $catalogWork 'tables'
$sqlitePath = Join-Path $catalogWork 'skill_status.sqlite'
$dataIndex = Join-Path $gameRoot 'data.i'

& $dataTool extract -i $dataIndex -f 'system/table/skill_status.tbl' -o $tables
& $dataTool extract -i $dataIndex -f 'system/table/text/ko/text.msg' -o $tables
& $dataTool extract -i $dataIndex -f 'system/table/text/en/text.msg' -o $tables
& $dataTool tbl-to-sqlite -i (Join-Path $tables 'system\table') -o $sqlitePath -v 2.0.2
```

The required source paths are exactly:

```text
system/table/skill_status.tbl
system/table/text/ko/text.msg
system/table/text/en/text.msg
```

Do not run `b-convert` on either `.msg`; GBFRDataTools 2.0.0 reports that extension as unrecognized. The Rust generator reads those MessagePack files directly.

Before continuing, verify the extracted `skill_status.tbl` SHA-256 is:

```powershell
(Get-FileHash -Algorithm SHA256 (Join-Path $tables 'system\table\skill_status.tbl')).Hash
```

Expected:

```text
96D56E65F107FD925B131D86959C9F829CE7102E6BDD39C7C6F3E80F663E7563
```

- [ ] **Step 3: Run the generator against staged outputs first**

Build and run the generator with staged destinations under `$catalogWork`:

```powershell
cargo run --locked --package gbfr-logs --bin build_trait_caps -- `
  (Join-Path $catalogWork 'skill_status.sqlite') `
  (Join-Path $catalogWork 'tables\system\table\text\ko\text.msg') `
  (Join-Path $catalogWork 'tables\system\table\text\en\text.msg') `
  (Join-Path $catalogWork 'trait-caps.json') `
  (Join-Path $catalogWork 'ko-traits.json') `
  (Join-Path $catalogWork 'en-traits.json') `
  '63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F'
```

Expected summary:

- 261 cap records.
- 170 Korean localized records verified in `text.msg`.
- 170 English localized records with the same key set.
- The Korean and English key sets are identical.
- 91 cap keys are intentionally absent from both name catalogs: 60 symbolic keys with no localization row in either language and 31 raw hashes.

- [ ] **Step 4: Write failing bundled-catalog coverage tests**

Add these tests to the generator test module before copying the staged files into the repository:

```rust
#[test]
fn bundled_2_0_2_catalogs_cover_every_verified_localized_trait() {
    let caps: TraitCapCatalog =
        serde_json::from_str(include_str!("../../assets/trait-caps.json")).unwrap();
    let ko: TraitNameCatalog =
        serde_json::from_str(include_str!("../../lang/ko/traits.json")).unwrap();
    let en: TraitNameCatalog =
        serde_json::from_str(include_str!("../../lang/en/traits.json")).unwrap();

    assert_eq!(caps.records.len(), 261);
    assert_eq!(ko.len(), 170);
    assert_eq!(en.len(), 170);
    assert_eq!(ko.keys().collect::<Vec<_>>(), en.keys().collect::<Vec<_>>());
    assert_eq!(caps.records.len() - ko.len(), 91);
    assert!(ko.values().all(|record| !record.text.trim().is_empty()));
    assert!(en.values().all(|record| !record.text.trim().is_empty()));
    assert!(ko.iter().all(|(hash, ko_record)| {
        en.get(hash).is_some_and(|en_record| en_record.key == ko_record.key)
    }));
    assert!(ko.keys().all(|hash| {
        let trait_id = u32::from_str_radix(hash, 16).unwrap();
        caps.records.iter().any(|record| record.trait_id == trait_id)
    }));
}

#[test]
fn bundled_2_0_2_catalog_contains_endless_ragnarok_trait() {
    let ko: TraitNameCatalog =
        serde_json::from_str(include_str!("../../lang/ko/traits.json")).unwrap();
    let en: TraitNameCatalog =
        serde_json::from_str(include_str!("../../lang/en/traits.json")).unwrap();
    let hash = format!("{:08x}", custom_xxhash32(b"SKILL_173_01"));

    assert_eq!(ko[&hash].key, "SKILL_173_01");
    assert_eq!(en[&hash].key, "SKILL_173_01");
    assert!(!ko[&hash].text.trim().is_empty());
    assert!(!en[&hash].text.trim().is_empty());
}
```

- [ ] **Step 5: Run the bundled coverage test and verify RED**

```powershell
cargo test --locked --package gbfr-logs --bin build_trait_caps bundled_2_0_2_catalogs_cover_every_verified_localized_trait
```

Expected: the old 1.3.x catalogs fail the 170-record assertion; the currently observed old count is 165.

- [ ] **Step 6: Copy only validated generated outputs into the repository**

```powershell
Copy-Item -LiteralPath (Join-Path $catalogWork 'trait-caps.json') -Destination 'src-tauri\assets\trait-caps.json'
Copy-Item -LiteralPath (Join-Path $catalogWork 'ko-traits.json') -Destination 'src-tauri\lang\ko\traits.json'
Copy-Item -LiteralPath (Join-Path $catalogWork 'en-traits.json') -Destination 'src-tauri\lang\en\traits.json'
```

Inspect the diff to ensure it is data-only and sorted by lowercase eight-digit hash:

```powershell
git diff --stat -- src-tauri/assets/trait-caps.json src-tauri/lang/ko/traits.json src-tauri/lang/en/traits.json
git diff --check
```

- [ ] **Step 7: Run the bundled coverage tests and verify GREEN**

```powershell
cargo test --locked --package gbfr-logs --bin build_trait_caps
```

Expected: every generator test passes, including 261/170/170 coverage and `SKILL_173_01`.

- [ ] **Step 8: Record reproducible provenance**

In `docs/research/2026-07-18-gbfr-er-2.0.2-trait-overflow.md`, add:

- the exact three game-relative source paths;
- SHA-256 for both extracted `.msg` files;
- SHA-256 for all three generated JSON files;
- 261 total cap records, 170 verified localized names, 60 unlocalized symbolic keys, and 31 raw hash-only records;
- the generator CLI shown above;
- the explicit statement that `.msg` is MessagePack and is parsed by `rmp-serde`;
- the explicit limitation that stage 1 still reads only the 12 equipped sigil slots.

Compute the new hashes rather than copying values from terminal history:

```powershell
Get-FileHash -Algorithm SHA256 `
  (Join-Path $catalogWork 'tables\system\table\text\ko\text.msg'), `
  (Join-Path $catalogWork 'tables\system\table\text\en\text.msg'), `
  'src-tauri\assets\trait-caps.json', `
  'src-tauri\lang\ko\traits.json', `
  'src-tauri\lang\en\traits.json'
```

- [ ] **Step 9: Commit generated data and provenance**

```powershell
git add src-tauri/src/bin/build_trait_caps.rs src-tauri/assets/trait-caps.json src-tauri/lang/ko/traits.json src-tauri/lang/en/traits.json docs/research/2026-07-18-gbfr-er-2.0.2-trait-overflow.md
git commit -m "data: update 2.0.2 trait names"
```

---

### Task 4: Show unresolved traits with a stable hexadecimal ID

**Files:**

- Modify: `src/utils.test.ts`
- Modify: `src/utils.ts`
- Modify: `src/pages/EquipmentAnalysis.test.tsx`
- Modify: `src-tauri/lang/ko/ui.json`
- Modify: `src-tauri/lang/en/ui.json`

- [ ] **Step 1: Write a failing translation fallback test**

At the top of `src/utils.test.ts`, add `beforeEach` and `vi` to the existing Vitest import, mock only the imported i18next `t` function, and add `translateTraitId` to the existing utility import:

```typescript
import { beforeEach, describe, expect, it, vi } from "vitest";

import enUi from "../src-tauri/lang/en/ui.json";
import koUi from "../src-tauri/lang/ko/ui.json";

const i18nextMocks = vi.hoisted(() => ({
  t: vi.fn(),
}));

vi.mock("i18next", () => ({
  t: i18nextMocks.t,
}));

import { getSkillTranslationKeys, toHash, toHashString, translateTraitId } from "./utils";
```

Add:

```typescript
it("passes an eight-digit trait ID to the unknown-trait fallback", () => {
  i18nextMocks.t.mockImplementation((keys, options) => {
    expect(keys).toEqual([
      "traits:0151cf9e.text",
      "ui.equipment-analysis.unknown-trait",
    ]);
    expect(options).toEqual({ id: "0151cf9e" });
    return "알 수 없는 특성 (0x0151cf9e)";
  });

  expect(translateTraitId(0x0151cf9e)).toBe("알 수 없는 특성 (0x0151cf9e)");
});
```

Clear `i18nextMocks.t` in the test suite's `beforeEach` so other utility tests are isolated.

Also add a resource assertion so both checked-in fallback strings are covered directly:

```typescript
it("defines ID-bearing unknown-trait fallbacks in both languages", () => {
  expect(koUi.ui["equipment-analysis"]["unknown-trait"]).toBe(
    "알 수 없는 특성 (0x{{id}})"
  );
  expect(enUi.ui["equipment-analysis"]["unknown-trait"]).toBe(
    "Unknown trait (0x{{id}})"
  );
});
```

- [ ] **Step 2: Run the test and verify RED**

```powershell
npm test -- --run src/utils.test.ts
```

Expected: the fallback key differs because production still requests `ui.unknown`.

- [ ] **Step 3: Add localized ID-bearing fallback strings**

Under the existing `equipment-analysis` object in `src-tauri/lang/ko/ui.json`, add:

```json
"unknown-trait": "알 수 없는 특성 (0x{{id}})"
```

Under the matching object in `src-tauri/lang/en/ui.json`, add:

```json
"unknown-trait": "Unknown trait (0x{{id}})"
```

Preserve the surrounding JSON key order and formatting.

- [ ] **Step 4: Change only the final trait-name fallback**

In `src/utils.ts`, change:

```typescript
return t([`traits:${hash}.text`, "ui.unknown"], { id: hash });
```

to:

```typescript
return t([`traits:${hash}.text`, "ui.equipment-analysis.unknown-trait"], {
  id: hash,
});
```

Keep the `null` and `EMPTY_ID` empty-string behavior unchanged.

- [ ] **Step 5: Prove an unresolved name does not hide its level or state**

In `src/pages/EquipmentAnalysis.test.tsx`, extend the existing `@/utils` mock so the known Damage Cap ID still returns `데미지 상한`, while every other ID returns `알 수 없는 특성 (0x${id.toString(16).padStart(8, "0")})`.

Add this rendering test:

```typescript
it("keeps level and cap state visible for an unresolved trait name", async () => {
  mocks.response = {
    connected: true,
    characters: [
      {
        characterType: "Pl1400",
        status: "complete",
        traits: [
          {
            traitId: 0x0151cf9e,
            totalLevel: 15,
            maxLevel: null,
            overflowLevel: 0,
            state: "unknown",
            sources: [],
          },
        ],
      },
    ],
  };

  renderPage();

  expect(await screen.findByText("알 수 없는 특성 (0x0151cf9e)")).toBeTruthy();
  expect(screen.getByText("15 / —")).toBeTruthy();
  expect(screen.getByText("최대치 미확인")).toBeTruthy();
});
```

- [ ] **Step 6: Run focused UI tests and verify GREEN**

```powershell
npm test -- --run src/utils.test.ts src/pages/EquipmentAnalysis.test.tsx
```

Expected: both test files pass and no known-trait rendering changes.

- [ ] **Step 7: Run frontend static checks**

```powershell
npm run format-check
npm run lint
npm run tsc
```

Expected: all commands exit 0.

- [ ] **Step 8: Commit the fallback behavior**

```powershell
git add src/utils.test.ts src/utils.ts src/pages/EquipmentAnalysis.test.tsx src-tauri/lang/ko/ui.json src-tauri/lang/en/ui.json
git commit -m "fix: identify unknown equipment traits"
```

---

### Task 5: Update user-facing scope and perform complete release verification

**Files:**

- Modify: `README.md`
- Modify: `docs/testing/game-2.0.2-smoke-test.md`
- Modify after packaging hash refresh: `README.md`
- Modify after packaging hash refresh: `docs/testing/game-2.0.2-smoke-test.md`

- [ ] **Step 1: Update the Korean and English user guide**

In both README language sections, state concisely:

- official Korean/English 2.0.2 trait names are used for recognized equipped sigils;
- an unresolved entry is displayed as `알 수 없는 특성 (0x1234abcd)` / `Unknown trait (0x1234abcd)` so it can be reported;
- current analysis covers the 12 equipped sigils;
- weapon, wrightstone, summon, and master-trait contributions are planned follow-up stages and are not yet included in totals.

Do not imply that cap totals are complete for builds using those excluded sources.

- [ ] **Step 2: Extend the manual smoke-test checklist**

In `docs/testing/game-2.0.2-smoke-test.md`, add unchecked checks for:

- a known Endless Ragnarok trait renders its official Korean name;
- switching the app to English renders the matching official English name;
- a raw hash-only trait, if encountered, shows an eight-digit hexadecimal ID and does not crash the screen;
- the UI clearly states or otherwise preserves the current 12-sigil-only scope.

Leave these items unchecked until verified in a real 2.0.2 session.

- [ ] **Step 3: Run the complete required verification suite**

Ensure the game is stopped before packaging. Then run exactly:

```powershell
npm ci
npm run format-check
npm run lint
npm run tsc
npm test -- --run
npm run build
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
npm run tauri build -- --bundles msi
```

Expected:

- every command exits 0;
- all Vitest and Cargo tests pass;
- the hook DLL and MSI are rebuilt successfully;
- no output claims the manual game smoke test has passed.

- [ ] **Step 4: Verify packaged hook equality and refresh release hashes**

```powershell
$releaseHook = (Resolve-Path 'target\release\hook.dll').Path
$bundledHook = (Resolve-Path 'src-tauri\hook.dll').Path
$releaseHookHash = (Get-FileHash -Algorithm SHA256 $releaseHook).Hash
$bundledHookHash = (Get-FileHash -Algorithm SHA256 $bundledHook).Hash
if ($releaseHookHash -ne $bundledHookHash) { throw 'Packaged hook.dll does not match target/release/hook.dll' }

$msi = Get-ChildItem -LiteralPath 'target\release\bundle\msi' -Filter '*.msi' |
  Sort-Object LastWriteTimeUtc -Descending |
  Select-Object -First 1
$msiHash = (Get-FileHash -Algorithm SHA256 $msi.FullName).Hash
$releaseHookHash
$msi.FullName
$msiHash
```

Write the new hook and MSI SHA-256 values to the existing release-hash sections in both `README.md` and `docs/testing/game-2.0.2-smoke-test.md`. Do not alter the manual compatibility status.

- [ ] **Step 5: Inspect the final scope and repository state**

```powershell
git status --short
git diff --check
git diff --stat master...HEAD
git diff master...HEAD -- . ':(exclude)logs.db'
```

Verify:

- `logs.db` remains untracked and unstaged;
- no protocol files changed;
- no weapon, wrightstone, summon, or master-trait reader was introduced;
- generated catalogs contain 170 verified official names in each language;
- fallback output always includes `0x` plus eight lowercase hex digits;
- README and smoke-test hashes match the files just built.

- [ ] **Step 6: Commit documentation and release hashes**

```powershell
git add README.md docs/testing/game-2.0.2-smoke-test.md
git commit -m "docs: document 2.0.2 trait catalog scope"
```

---

## Completion Criteria

- The generator derives caps and both localized catalogs from one 2.0.2 `skill_status.tbl` plus Korean and English `text.msg` inputs.
- Korean and English catalogs each contain the same 170 verified localized trait hashes.
- The cap catalog still contains all 261 records, including 60 unlocalized symbolic records and 31 raw hash-only records.
- Known traits use official extracted 2.0.2 text; raw or otherwise unresolved traits show a stable hexadecimal ID.
- Current totals remain explicitly scoped to the 12 equipped sigils.
- Unit tests, formatting, lint, TypeScript checks, frontend build, release hook build, all Cargo tests, and MSI packaging pass.
- Packaged and release hook DLL SHA-256 values are equal and recorded alongside the rebuilt MSI hash.
- Manual game smoke-test items remain unchecked until an actual 2.0.2 test session is performed.

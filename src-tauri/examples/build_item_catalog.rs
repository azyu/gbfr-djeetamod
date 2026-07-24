use std::{
    collections::{BTreeMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};

const GAME_VERSION: &str = "2.0.2";
const GAME_EXE_SHA256: &str = "63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F";
const EXPECTED_ITEM_ROWS: usize = 448;
const EXPECTED_LOCALIZED_ITEMS: usize = 384;
const EXPECTED_UNNAMED_ROWS: usize = 64;
const PRIME32_1: u32 = 0x9E37_79B1;
const PRIME32_2: u32 = 0x85EB_CA77;
const PRIME32_3: u32 = 0xC2B2_AE3D;
const PRIME32_4: u32 = 0x27D4_EB2F;
const PRIME32_5: u32 = 0x1656_67B1;

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

fn custom_xxhash32(input: &[u8]) -> u32 {
    fn round(seed: u32, input: u32) -> u32 {
        seed.wrapping_add(input.wrapping_mul(PRIME32_2))
            .rotate_left(13)
            .wrapping_mul(PRIME32_1)
    }

    fn read_u32(input: &[u8]) -> u32 {
        u32::from_le_bytes(input[..4].try_into().expect("four-byte slice"))
    }

    let mut remaining = input;
    let mut hash = 0x178A_54A4_u32;
    if input.len() >= 16 {
        let mut v1 = 0x2557_311B_u32;
        let mut v2 = 0x871F_B76A_u32;
        let mut v3 = 0x0133_ECF3_u32;
        let mut v4 = 0x62FC_7342_u32;
        loop {
            v1 = round(v1, read_u32(remaining));
            v2 = round(v2, read_u32(&remaining[4..]));
            v3 = round(v3, read_u32(&remaining[8..]));
            v4 = round(v4, read_u32(&remaining[12..]));
            remaining = &remaining[16..];
            if remaining.len() <= 16 {
                break;
            }
        }
        hash = v1
            .rotate_left(1)
            .wrapping_add(v2.rotate_left(7))
            .wrapping_add(v3.rotate_left(12))
            .wrapping_add(v4.rotate_left(18));
    }
    hash = hash.wrapping_add(input.len() as u32);

    while remaining.len() >= 4 {
        hash = hash
            .wrapping_add(read_u32(remaining).wrapping_mul(PRIME32_3))
            .rotate_left(17)
            .wrapping_mul(PRIME32_4);
        remaining = &remaining[4..];
    }
    for byte in remaining {
        hash = hash
            .wrapping_add(u32::from(*byte).wrapping_mul(PRIME32_5))
            .rotate_left(11)
            .wrapping_mul(PRIME32_1);
    }
    hash ^= hash >> 15;
    hash = hash.wrapping_mul(PRIME32_2);
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(PRIME32_3);
    hash ^ (hash >> 16)
}

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
    let mut statement = connection.prepare(
        "SELECT name FROM sqlite_master
         WHERE type = 'table' AND lower(name) = 'item'
         ORDER BY name",
    )?;
    let tables = statement
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
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
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
    if ko.keys().ne(en.keys()) || ko.iter().any(|(hash, record)| en[hash].key != record.key) {
        bail!("Korean and English item catalogs differ");
    }
    Ok(GeneratedCatalogs {
        ko_names: pretty_json(&ko)?,
        en_names: pretty_json(&en)?,
        item_count: definitions.len(),
    })
}

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

    let connection = Connection::open_with_flags(sqlite_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("failed to open {}", sqlite_path.display()))?;
    let ko_message = fs::read(ko_message_path)
        .with_context(|| format!("failed to read {}", ko_message_path.display()))?;
    let en_message = fs::read(en_message_path)
        .with_context(|| format!("failed to read {}", en_message_path.display()))?;
    let generated = generate_catalogs(&connection, &ko_message, &en_message, game_hash)?;
    let total_rows = usize::try_from(
        connection
            .query_row("SELECT COUNT(*) FROM item", [], |row| row.get::<_, i64>(0))
            .context("count item rows")?,
    )
    .context("item row count is outside usize")?;
    let unnamed_count = total_rows.saturating_sub(generated.item_count);
    if total_rows != EXPECTED_ITEM_ROWS
        || generated.item_count != EXPECTED_LOCALIZED_ITEMS
        || unnamed_count != EXPECTED_UNNAMED_ROWS
    {
        bail!(
            "unexpected 2.0.2 item counts: total={total_rows}, localized={}, unnamed={unnamed_count}",
            generated.item_count
        );
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

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::Connection;

    use super::{
        custom_xxhash32, generate_catalogs, load_item_definitions, write_prepared_outputs,
        ItemNameCatalog, MessageColumn, MessageRow, MessageTable, GAME_EXE_SHA256,
    };

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

    #[test]
    fn hashes_symbolic_item_keys_like_the_game() {
        assert_eq!(custom_xxhash32(b"ITEM_31_0090"), 0x000e_b8f5);
    }

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

    #[test]
    fn writes_both_prepared_outputs() {
        let root = std::env::temp_dir().join(format!(
            "djeeta-item-catalog-output-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let ko_path = root.join("ko-items.json");
        let en_path = root.join("en-items.json");

        write_prepared_outputs([
            (&ko_path, b"korean".as_slice()),
            (&en_path, b"english".as_slice()),
        ])
        .unwrap();

        assert_eq!(fs::read(&ko_path).unwrap(), b"korean");
        assert_eq!(fs::read(&en_path).unwrap(), b"english");
        fs::remove_dir_all(root).unwrap();
    }
}

use std::{collections::BTreeMap, env, fs::File, io::Write, path::Path};

use anyhow::{bail, Context, Result};
use rusqlite::{types::ValueRef, Connection};
use serde::{Deserialize, Serialize};

const EMPTY_TRAIT_ID: u32 = 0x887A_E0B0;
const GAME_VERSION: &str = "2.0.2";
const PRIME32_1: u32 = 0x9E37_79B1;
const PRIME32_2: u32 = 0x85EB_CA77;
const PRIME32_3: u32 = 0xC2B2_AE3D;
const PRIME32_4: u32 = 0x27D4_EB2F;
const PRIME32_5: u32 = 0x1656_67B1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TraitCapRecord {
    trait_id: u32,
    max_level: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TraitCapCatalog {
    game_version: &'static str,
    game_exe_sha256: String,
    records: Vec<TraitCapRecord>,
}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct TraitNameRecord {
    key: String,
    text: String,
}

type TraitNameCatalog = BTreeMap<String, TraitNameRecord>;

fn parse_trait_key(value: ValueRef<'_>, max_level: u32) -> Result<TraitDefinition> {
    match value {
        ValueRef::Integer(value) => Ok(TraitDefinition::raw(
            u32::try_from(value).context("trait key is outside u32")?,
            max_level,
        )),
        ValueRef::Text(value) => {
            let value = std::str::from_utf8(value)?.trim();
            let hexadecimal = value
                .strip_prefix("0x")
                .or_else(|| value.strip_prefix("0X"))
                .unwrap_or(value);
            if hexadecimal.len() == 8 && hexadecimal.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                Ok(TraitDefinition::raw(
                    u32::from_str_radix(hexadecimal, 16)?,
                    max_level,
                ))
            } else if value.is_ascii() && value.starts_with("SKILL_") {
                Ok(TraitDefinition::symbolic(
                    value,
                    custom_xxhash32(value.as_bytes()),
                    max_level,
                ))
            } else {
                bail!("trait key {value:?} is neither SKILL_ text nor an eight-digit hash")
            }
        }
        _ => bail!("trait key must be an integer or hexadecimal text"),
    }
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

fn parse_message_names(bytes: &[u8]) -> Result<BTreeMap<String, String>> {
    let table: MessageTable = rmp_serde::from_slice(bytes).context("parse text.msg MessagePack")?;
    let mut names = BTreeMap::new();

    for row in table.rows_ {
        let MessageColumn {
            id_hash_,
            subid_hash_,
            text_,
        } = row.column_;
        let _ = subid_hash_;
        let Some(key) = id_hash_.strip_prefix("TXT_SKILL_") else {
            continue;
        };
        let key = format!("SKILL_{key}");
        let text = text_.trim();
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
) -> Result<TraitNameCatalog> {
    let mut catalog = BTreeMap::new();

    for definition in definitions {
        let TraitKey::Symbolic(key) = &definition.key else {
            continue;
        };
        let text = names
            .get(key)
            .with_context(|| format!("{language} name missing for {key}"))?;
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

fn load_definitions(connection: &Connection) -> Result<Vec<TraitDefinition>> {
    let table_name = find_skill_status_table(connection)?;
    let quoted_table_name = table_name.replace('"', "\"\"");
    let query =
        format!("SELECT Key, MAX(Level) FROM \"{quoted_table_name}\" GROUP BY Key ORDER BY Key");
    let mut statement = connection.prepare(&query)?;
    let mut rows = statement.query([])?;
    let mut definitions = Vec::new();

    while let Some(row) = rows.next()? {
        let max_level =
            u32::try_from(row.get::<_, i64>(1)?).context("trait maximum level is outside u32")?;
        let definition = parse_trait_key(row.get_ref(0)?, max_level)?;
        if definition.trait_id != 0
            && definition.trait_id != EMPTY_TRAIT_ID
            && definition.max_level > 0
        {
            definitions.push(definition);
        }
    }

    definitions.sort_by_key(|definition| definition.trait_id);
    Ok(definitions)
}

fn build_cap_records(definitions: &[TraitDefinition]) -> Vec<TraitCapRecord> {
    definitions
        .iter()
        .map(|definition| TraitCapRecord {
            trait_id: definition.trait_id,
            max_level: definition.max_level,
        })
        .collect()
}

fn load_records(connection: &Connection) -> Result<Vec<TraitCapRecord>> {
    Ok(build_cap_records(&load_definitions(connection)?))
}

fn find_skill_status_table(connection: &Connection) -> Result<String> {
    let mut statement = connection.prepare(
        "SELECT name FROM sqlite_master
         WHERE type = 'table' AND lower(name) LIKE '%skill_status%'
         ORDER BY name",
    )?;
    let candidates = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    if let Some(exact) = candidates
        .iter()
        .find(|candidate| candidate.eq_ignore_ascii_case("skill_status"))
    {
        return Ok(exact.clone());
    }
    match candidates.as_slice() {
        [only] => Ok(only.clone()),
        [] => bail!("SQLite contains no table whose name includes skill_status"),
        _ => bail!(
            "SQLite contains multiple possible skill_status tables: {}",
            candidates.join(", ")
        ),
    }
}

fn normalized_sha256(value: &str) -> Result<String> {
    let value = value.trim();
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("game executable SHA-256 must contain exactly 64 hexadecimal characters");
    }
    Ok(value.to_ascii_uppercase())
}

fn write_catalog(sqlite_path: &Path, output_path: &Path, game_exe_sha256: &str) -> Result<usize> {
    let connection = Connection::open(sqlite_path)
        .with_context(|| format!("failed to open {}", sqlite_path.display()))?;
    let records = load_records(&connection)?;
    let record_count = records.len();
    let catalog = TraitCapCatalog {
        game_version: GAME_VERSION,
        game_exe_sha256: normalized_sha256(game_exe_sha256)?,
        records,
    };
    let mut output = File::create(output_path)
        .with_context(|| format!("failed to create {}", output_path.display()))?;
    serde_json::to_writer_pretty(&mut output, &catalog)?;
    output.write_all(b"\n")?;
    Ok(record_count)
}

fn main() -> Result<()> {
    let arguments: Vec<_> = env::args_os().skip(1).collect();
    if arguments.len() != 3 {
        bail!("usage: build_trait_caps <input.sqlite> <output.json> <game-exe-sha256>");
    }
    let sqlite_path = Path::new(&arguments[0]);
    let output_path = Path::new(&arguments[1]);
    let game_exe_sha256 = arguments[2]
        .to_str()
        .context("game executable SHA-256 must be valid Unicode")?;
    let record_count = write_catalog(sqlite_path, output_path, game_exe_sha256)?;
    println!("wrote {record_count} trait cap records");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        build_name_catalog, custom_xxhash32, load_records, normalized_sha256, parse_message_names,
        TraitCapRecord, TraitDefinition,
    };
    use rusqlite::Connection;

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
    fn selects_highest_level_per_trait_and_sorts_by_id() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE skill_status (Key TEXT NOT NULL, Level INTEGER NOT NULL);
                 INSERT INTO skill_status VALUES
                 ('0x00000002', 10), ('0x00000001', 1),
                 ('0x00000001', 65), ('0x887AE0B0', 99);",
            )
            .unwrap();

        assert_eq!(
            load_records(&connection).unwrap(),
            vec![
                TraitCapRecord {
                    trait_id: 1,
                    max_level: 65,
                },
                TraitCapRecord {
                    trait_id: 2,
                    max_level: 10,
                },
            ]
        );
    }

    #[test]
    fn normalizes_and_validates_executable_hash() {
        assert_eq!(
            normalized_sha256(&"ab".repeat(32)).unwrap(),
            "AB".repeat(32)
        );
        assert!(normalized_sha256("not-a-hash").is_err());
    }

    #[test]
    fn hashes_symbolic_trait_keys_like_the_game() {
        assert_eq!(custom_xxhash32(b"SKILL_000_00"), 0x5007_9A1C);
        assert_eq!(custom_xxhash32(b"SKILL_020_00"), 0xDC58_4F60);
    }

    #[test]
    fn parses_skill_names_from_messagepack() {
        let bytes = message_fixture(&[("TXT_SKILL_020_00", "대미지 상한"), ("TXT_OTHER", "무시")]);

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
        let damage_cap_hash = format!("{:08x}", custom_xxhash32(b"SKILL_020_00"));

        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog[&damage_cap_hash].key, "SKILL_020_00");
        assert_eq!(catalog[&damage_cap_hash].text, "Damage Cap");
        assert!(!catalog.contains_key("0151cf9e"));
    }

    #[test]
    fn rejects_a_symbolic_trait_missing_from_a_language_table() {
        let definitions = vec![TraitDefinition::symbolic(
            "SKILL_020_00",
            custom_xxhash32(b"SKILL_020_00"),
            65,
        )];

        let error = build_name_catalog(&definitions, &BTreeMap::new(), "Korean")
            .unwrap_err()
            .to_string();

        assert!(error.contains("Korean"));
        assert!(error.contains("SKILL_020_00"));
    }

    #[test]
    fn rejects_empty_names_and_trait_hash_collisions() {
        let empty = message_fixture(&[("TXT_SKILL_020_00", "   ")]);
        assert!(parse_message_names(&empty)
            .unwrap_err()
            .to_string()
            .contains("empty"));

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
}

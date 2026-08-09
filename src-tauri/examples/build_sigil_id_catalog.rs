use std::{
    collections::HashSet,
    env, fs,
    path::Path,
};

use anyhow::{bail, Context, Result};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};

const GAME_VERSION: &str = "2.0.4";
const GAME_EXE_SHA256: &str = "F827F3C13CAA90B290FAB2FE7E28165A80448FDE0A3F7A96D79DAC6B8343FF2A";
const EXPECTED_SIGIL_ROWS: usize = 1_034;
const EXPECTED_TRAIT_IDS: usize = 193;
const PRIME32_1: u32 = 0x9E37_79B1;
const PRIME32_2: u32 = 0x85EB_CA77;
const PRIME32_3: u32 = 0xC2B2_AE3D;
const PRIME32_4: u32 = 0x27D4_EB2F;
const PRIME32_5: u32 = 0x1656_67B1;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SigilIdCatalog {
    game_version: String,
    game_exe_sha256: String,
    sigil_ids: Vec<String>,
    trait_ids: Vec<String>,
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

fn parse_id(key: &str, prefix: &str, label: &str) -> Result<u32> {
    let key = key.trim();
    let hexadecimal = key
        .strip_prefix("0x")
        .or_else(|| key.strip_prefix("0X"))
        .unwrap_or(key);
    if hexadecimal.len() == 8 && hexadecimal.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Ok(u32::from_str_radix(hexadecimal, 16)?);
    }
    if key.is_ascii() && key.starts_with(prefix) {
        return Ok(custom_xxhash32(key.as_bytes()));
    }
    bail!("{label} key {key:?} is neither {prefix} text nor an eight-digit hash")
}

fn sorted_unique(values: impl IntoIterator<Item = u32>, label: &str) -> Result<Vec<u32>> {
    let mut values = values.into_iter().collect::<Vec<_>>();
    values.sort_unstable();
    let unique = values.iter().copied().collect::<HashSet<_>>();
    if unique.len() != values.len() {
        bail!("{label} keys contain hash collisions")
    }
    Ok(values)
}

fn load_catalog_ids(connection: &Connection) -> Result<(Vec<u32>, Vec<u32>)> {
    let table_count = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND lower(name) = 'gem'",
        [],
        |row| row.get::<_, usize>(0),
    )?;
    if table_count != 1 {
        bail!("SQLite must contain exactly one gem table; found {table_count}")
    }

    let mut statement =
        connection.prepare("SELECT Key, SkillId1, SkillId2 FROM gem ORDER BY Key")?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if rows.len() != EXPECTED_SIGIL_ROWS {
        bail!(
            "unexpected {GAME_VERSION} sigil row count: expected {EXPECTED_SIGIL_ROWS}, got {}",
            rows.len()
        )
    }

    let sigil_ids = sorted_unique(
        rows.iter()
            .map(|(key, _, _)| parse_id(key, "GEEN_", "sigil"))
            .collect::<Result<Vec<_>>>()?,
        "sigil",
    )?;
    let trait_ids = rows
        .iter()
        .flat_map(|(_, primary, secondary)| [primary, secondary])
        .filter(|key| !key.trim().is_empty())
        .map(|key| parse_id(key, "SKILL_", "trait"))
        .collect::<Result<HashSet<_>>>()?;
    if trait_ids.len() != EXPECTED_TRAIT_IDS {
        bail!(
            "unexpected {GAME_VERSION} trait ID count: expected {EXPECTED_TRAIT_IDS}, got {}",
            trait_ids.len()
        )
    }
    let mut trait_ids = trait_ids.into_iter().collect::<Vec<_>>();
    trait_ids.sort_unstable();
    Ok((sigil_ids, trait_ids))
}

fn build_catalog(connection: &Connection, game_hash: &str) -> Result<SigilIdCatalog> {
    let normalized_hash = game_hash.trim().to_ascii_uppercase();
    if normalized_hash != GAME_EXE_SHA256 {
        bail!("expected game executable SHA-256 {GAME_EXE_SHA256}, got {normalized_hash}")
    }
    let (sigil_ids, trait_ids) = load_catalog_ids(connection)?;
    Ok(SigilIdCatalog {
        game_version: GAME_VERSION.to_owned(),
        game_exe_sha256: normalized_hash,
        sigil_ids: sigil_ids
            .into_iter()
            .map(|value| format!("{value:08x}"))
            .collect(),
        trait_ids: trait_ids
            .into_iter()
            .map(|value| format!("{value:08x}"))
            .collect(),
    })
}

fn main() -> Result<()> {
    let arguments = env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.len() != 3 {
        bail!(
            "usage: build_sigil_id_catalog <input.sqlite> <output.json> <game-exe-sha256>"
        )
    }
    let sqlite_path = Path::new(&arguments[0]);
    let output_path = Path::new(&arguments[1]);
    let game_hash = arguments[2]
        .to_str()
        .context("game executable SHA-256 must be valid Unicode")?;
    let connection = Connection::open_with_flags(sqlite_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("failed to open {}", sqlite_path.display()))?;
    let catalog = build_catalog(&connection, game_hash)?;
    let mut bytes = serde_json::to_vec_pretty(&catalog)?;
    bytes.push(b'\n');
    fs::write(output_path, bytes)
        .with_context(|| format!("failed to write {}", output_path.display()))?;
    println!(
        "wrote {} sigil IDs and {} trait IDs for game {GAME_VERSION}",
        catalog.sigil_ids.len(),
        catalog.trait_ids.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::{build_catalog, custom_xxhash32, GAME_EXE_SHA256};

    #[test]
    fn hashes_symbolic_sigil_keys_like_the_game() {
        assert_eq!(custom_xxhash32(b"GEEN_158_13"), 0x0045_57B8);
    }

    #[test]
    fn rejects_an_incomplete_table_and_wrong_game_hash() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute(
                "CREATE TABLE gem (
                    Key TEXT NOT NULL,
                    SkillId1 TEXT NOT NULL,
                    SkillId2 TEXT NOT NULL
                )",
                [],
            )
            .unwrap();
        assert!(build_catalog(&connection, GAME_EXE_SHA256)
            .unwrap_err()
            .to_string()
            .contains("row count"));
        assert!(build_catalog(&connection, "00")
            .unwrap_err()
            .to_string()
            .contains("expected game executable"));
    }
}

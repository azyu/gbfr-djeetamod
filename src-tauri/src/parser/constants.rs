use serde::{Deserialize, Serialize};
use strum_macros::Display;

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone, Copy, Display)]
pub enum CharacterType {
    /// Gran
    Pl0000,
    /// Djeeta
    Pl0100,
    /// Katalina
    Pl0200,
    /// Rackam
    Pl0300,
    /// Io
    Pl0400,
    /// Eugen
    Pl0500,
    /// Rosetta
    Pl0600,
    /// Ferry
    Pl0700,
    /// Lancelot
    Pl0800,
    /// Vane
    Pl0900,
    /// Percival
    Pl1000,
    /// Siegfried
    Pl1100,
    /// Charlotta
    Pl1200,
    /// Yodarha
    Pl1300,
    /// Narmaya
    Pl1400,
    /// Ghandagoza
    Pl1500,
    /// Zeta
    Pl1600,
    /// Vaseraga
    Pl1700,
    /// Cagliostro
    Pl1800,
    /// Id
    Pl1900,
    /// Id (Transformation)
    Pl2000,
    /// Sandalphon
    Pl2100,
    /// Seofon
    Pl2200,
    /// Tweyen
    Pl2300,
    /// Gallanza
    Pl2400,
    /// Maglielle
    Pl2500,
    /// Beatrix
    Pl2600,
    /// Eustace
    Pl2700,
    /// Fraux
    Pl2800,
    /// Fediel
    Pl2900,
    /// Ferry Ghost
    Pl0700Ghost,
    /// Ferry Ghost (Satellite) / Umlauf
    Pl0700GhostSatellite,
    #[strum(default)]
    Unknown(u32),
}

impl CharacterType {
    pub fn from_hash(hash: u32) -> Self {
        match hash {
            0x26A4848A => CharacterType::Pl0000,
            0x9498420D => CharacterType::Pl0100,
            0x34D4FD8F => CharacterType::Pl0200,
            0xF8D73D33 => CharacterType::Pl0300,
            0x7B5934AD => CharacterType::Pl0400,
            0x443D46BB => CharacterType::Pl0500,
            0xA9D6569E => CharacterType::Pl0600,
            0xFBA6615D => CharacterType::Pl0700,
            0x63A7C3F0 => CharacterType::Pl0800,
            0xF96A90C2 => CharacterType::Pl0900,
            0x28AC1108 => CharacterType::Pl1000,
            0x94E2514E => CharacterType::Pl1100,
            0x2B4AA114 => CharacterType::Pl1200,
            0xC97F3365 => CharacterType::Pl1300,
            0x601AA977 => CharacterType::Pl1400,
            0xBCC238DE => CharacterType::Pl1500,
            0xC3155079 => CharacterType::Pl1600,
            0xD16CFBDE => CharacterType::Pl1700,
            0x6FDD6932 => CharacterType::Pl1800,
            0x8056ABCD => CharacterType::Pl1900,
            0xF5755C0E => CharacterType::Pl2000,
            0x9C89A455 => CharacterType::Pl2100,
            0x59DB0CD9 => CharacterType::Pl2200,
            0xDA5A8E25 => CharacterType::Pl2300,
            0x4C714F77 => CharacterType::Pl2400,
            0xE330418F => CharacterType::Pl2500,
            0xE3D1BE26 => CharacterType::Pl2600,
            0x91418145 => CharacterType::Pl2700,
            0x48ADDA36 => CharacterType::Pl2800,
            0x0A58FB4D => CharacterType::Pl2900,
            0x2AF678E8 => CharacterType::Pl0700Ghost,
            0x8364C8BC => CharacterType::Pl0700GhostSatellite,
            // Equipment snapshots identify characters with the game's custom
            // XXHash32 of the uppercase PLxxxx code, rather than actor type IDs.
            0x2A26B1B2 => CharacterType::Pl0000,
            0xA4ACBA76 => CharacterType::Pl0100,
            0x18E2F9F9 => CharacterType::Pl0200,
            0x079DF0CC => CharacterType::Pl0300,
            0x4D0A60C3 => CharacterType::Pl0400,
            0xDD7A151E => CharacterType::Pl0500,
            0xC8616284 => CharacterType::Pl0600,
            0xC3FFD418 => CharacterType::Pl0700,
            0x22E437E5 => CharacterType::Pl0800,
            0x2EBE91D5 => CharacterType::Pl0900,
            0xBDEF7181 => CharacterType::Pl1000,
            0x627BCB0D => CharacterType::Pl1100,
            0xFD3BE362 => CharacterType::Pl1200,
            0xFC6CDF7B => CharacterType::Pl1300,
            0xE7053919 => CharacterType::Pl1400,
            0x978E4B18 => CharacterType::Pl1500,
            0x0D21B430 => CharacterType::Pl1600,
            0xF0EB77EF => CharacterType::Pl1700,
            0xAA66178A => CharacterType::Pl1800,
            0xA3A3CB2F => CharacterType::Pl1900,
            0xF92C7821 => CharacterType::Pl2000,
            0x718E1A14 => CharacterType::Pl2100,
            0x296471BE => CharacterType::Pl2200,
            0xBAD16E3B => CharacterType::Pl2300,
            0x1BB37EF0 => CharacterType::Pl2400,
            0x25D46F4B => CharacterType::Pl2500,
            0x9A8AF295 => CharacterType::Pl2600,
            0x9B15CFB1 => CharacterType::Pl2700,
            0x646C3168 => CharacterType::Pl2800,
            0x74DD4C79 => CharacterType::Pl2900,
            _ => CharacterType::Unknown(hash),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CharacterType;

    #[test]
    fn recognizes_game_2_characters() {
        assert_eq!(CharacterType::from_hash(0x4C714F77), CharacterType::Pl2400);
        assert_eq!(CharacterType::from_hash(0xE330418F), CharacterType::Pl2500);
        assert_eq!(CharacterType::from_hash(0xE3D1BE26), CharacterType::Pl2600);
        assert_eq!(CharacterType::from_hash(0x91418145), CharacterType::Pl2700);
        assert_eq!(CharacterType::from_hash(0x48ADDA36), CharacterType::Pl2800);
        assert_eq!(CharacterType::from_hash(0x0A58FB4D), CharacterType::Pl2900);
    }

    #[test]
    fn recognizes_equipment_character_keys() {
        assert_eq!(CharacterType::from_hash(0x079D_F0CC), CharacterType::Pl0300);
        assert_eq!(CharacterType::from_hash(0xDD7A_151E), CharacterType::Pl0500);
        assert_eq!(CharacterType::from_hash(0xE705_3919), CharacterType::Pl1400);
        assert_eq!(CharacterType::from_hash(0x9B15_CFB1), CharacterType::Pl2700);
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone, Copy, Display)]
pub enum EnemyType {
    #[strum(default)]
    Unknown(u32),
}

impl EnemyType {
    pub fn from_hash(hash: u32) -> Self {
        EnemyType::Unknown(hash)
    }
}

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum FerrySkillId {
    PetNormal = 65u32,
    BlausGespenst = 1100u32,
    Pendel = 1400u32,
    Strafe = 1500u32,
}

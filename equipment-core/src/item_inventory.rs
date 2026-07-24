use std::collections::{HashMap, HashSet};

use thiserror::Error;

pub const ITEM_RECORD_BYTES: usize = 0x30;
pub const ITEM_ID_OFFSET: usize = 0x00;
pub const ITEM_QUANTITY_OFFSET: usize = 0x04;
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
    #[error("item {0:#010x} appears more than once")]
    DuplicateItem(u32),
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("validated item record field"),
    )
}

fn has_verified_structure(record: &[u8]) -> bool {
    read_u32(record, 0x08) == 0x0c
        && read_u32(record, 0x10) == 0
        && read_u32(record, 0x14) == u32::MAX
        && read_u32(record, 0x18) == u32::MAX
        && read_u32(record, 0x1c) == u32::MAX
}

pub fn decode_item_inventory(
    bytes: &[u8],
    ordinary_item_ids: &HashSet<u32>,
) -> Result<Vec<OwnedItem>, ItemInventoryDecodeError> {
    let mut items = HashMap::new();
    for record in bytes.windows(ITEM_RECORD_BYTES).step_by(4) {
        if !has_verified_structure(record) {
            continue;
        }
        let item_id = read_u32(record, ITEM_ID_OFFSET);
        if !ordinary_item_ids.contains(&item_id) {
            continue;
        }
        let quantity = read_u32(record, ITEM_QUANTITY_OFFSET);
        if quantity > ITEM_MAX_QUANTITY {
            continue;
        }
        if items
            .insert(item_id, OwnedItem { item_id, quantity })
            .is_some()
        {
            return Err(ItemInventoryDecodeError::DuplicateItem(item_id));
        }
    }

    let mut items = items.into_values().collect::<Vec<_>>();
    items.sort_unstable_by_key(|item| (std::cmp::Reverse(item.quantity), item.item_id));
    Ok(items)
}

pub fn warning_items(items: &[OwnedItem]) -> Vec<OwnedItem> {
    items
        .iter()
        .copied()
        .filter(|item| item.quantity >= ITEM_WARNING_THRESHOLD)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{
        decode_item_inventory, warning_items, ItemInventoryDecodeError, OwnedItem, ITEM_ID_OFFSET,
        ITEM_MAX_QUANTITY, ITEM_QUANTITY_OFFSET, ITEM_RECORD_BYTES, ITEM_WARNING_THRESHOLD,
    };

    const ITEM_A: u32 = 0x6877_33c4;
    const ITEM_B: u32 = 0x2e94_d39a;
    const UNKNOWN_ITEM: u32 = 0xdead_beef;

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn record(item_id: u32, quantity: u32) -> Vec<u8> {
        let mut bytes = vec![0; ITEM_RECORD_BYTES];
        put_u32(&mut bytes, ITEM_ID_OFFSET, item_id);
        put_u32(&mut bytes, ITEM_QUANTITY_OFFSET, quantity);
        put_u32(&mut bytes, 0x08, 0x0c);
        put_u32(&mut bytes, 0x10, 0);
        put_u32(&mut bytes, 0x14, u32::MAX);
        put_u32(&mut bytes, 0x18, u32::MAX);
        put_u32(&mut bytes, 0x1c, u32::MAX);
        bytes
    }

    #[test]
    fn decodes_only_verified_ordinary_item_records() {
        let known = HashSet::from([ITEM_A, ITEM_B]);
        let bytes = [
            vec![0; 4],
            record(ITEM_A, 899),
            vec![0; 8],
            record(UNKNOWN_ITEM, 999),
            record(ITEM_B, 900),
        ]
        .concat();

        assert_eq!(
            decode_item_inventory(&bytes, &known).unwrap(),
            vec![
                OwnedItem {
                    item_id: ITEM_B,
                    quantity: 900,
                },
                OwnedItem {
                    item_id: ITEM_A,
                    quantity: 899,
                },
            ]
        );
    }

    #[test]
    fn filters_inclusive_warning_boundaries() {
        let items = vec![
            OwnedItem {
                item_id: 1,
                quantity: ITEM_WARNING_THRESHOLD - 1,
            },
            OwnedItem {
                item_id: 2,
                quantity: ITEM_WARNING_THRESHOLD,
            },
            OwnedItem {
                item_id: 3,
                quantity: ITEM_MAX_QUANTITY,
            },
        ];

        assert_eq!(warning_items(&items), items[1..]);
    }

    #[test]
    fn ignores_higher_cap_currency_and_rejects_duplicates() {
        let known = HashSet::from([ITEM_A]);
        assert_eq!(
            decode_item_inventory(&record(ITEM_A, ITEM_MAX_QUANTITY + 1), &known),
            Ok(Vec::new())
        );

        let duplicate = [record(ITEM_A, 1), record(ITEM_A, 2)].concat();
        assert_eq!(
            decode_item_inventory(&duplicate, &known),
            Err(ItemInventoryDecodeError::DuplicateItem(ITEM_A))
        );
    }
}

const RESET_PREFIX: &[u8] = &[
    0x48, 0x83, 0xB8, 0x08, 0xC1, 0x01, 0x00, 0x00, 0xC7, 0x80, 0x24, 0xC1, 0x01, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x0F, 0x84,
];
const RESET_SUFFIX: &[u8] = &[
    0xC6, 0x87, 0x24, 0x06, 0x00, 0x00, 0x00, 0x48, 0x8D, 0x8F, 0x28, 0x06, 0x00, 0x00, 0x31, 0xD2,
    0x45, 0x31, 0xC0, 0x44, 0x89, 0x01, 0x85, 0xDB, 0x75, 0x15,
];
const GETTER_PREFIX: &[u8] = &[
    0x48, 0x83, 0xC1, 0x15, 0xEB, 0x0C, 0xB9, 0x24, 0x06, 0x00, 0x00, 0x48, 0x03, 0x0D,
];
const GETTER_SUFFIX: &[u8] = &[0x0F, 0xB6, 0x01, 0x48, 0x83, 0xC4, 0x20];
const SIGNATURE_WILDCARD_BYTES: usize = 4;
const RESET_PATCH_OFFSET: usize = 0x28;
const GETTER_PATCH_OFFSET: usize = 0x12;
const RESET_ORIGINAL: [u8; 3] = [0x45, 0x31, 0xC0];
const RESET_PATCHED: [u8; 3] = [0x44, 0x8B, 0x01];
const GETTER_ORIGINAL: [u8; 3] = [0x0F, 0xB6, 0x01];
const GETTER_PATCHED: [u8; 3] = [0xB0, 0x01, 0x90];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PatchSiteName {
    Reset,
    Getter,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
enum RepeatQuestError {
    #[error("{site:?} signature count was {count}")]
    SignatureCount { site: PatchSiteName, count: usize },
    #[error("patch address overflow")]
    AddressOverflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PatchOffsets {
    reset: usize,
    getter: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SiteBytes {
    Original,
    Patched,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ObservedPatchState {
    Off,
    On,
    Mixed,
    Unknown,
}

fn classify_site(bytes: [u8; 3], original: [u8; 3], patched: [u8; 3]) -> SiteBytes {
    if bytes == original {
        SiteBytes::Original
    } else if bytes == patched {
        SiteBytes::Patched
    } else {
        SiteBytes::Unknown
    }
}

fn classify_pair(reset: [u8; 3], getter: [u8; 3]) -> ObservedPatchState {
    match (
        classify_site(reset, RESET_ORIGINAL, RESET_PATCHED),
        classify_site(getter, GETTER_ORIGINAL, GETTER_PATCHED),
    ) {
        (SiteBytes::Original, SiteBytes::Original) => ObservedPatchState::Off,
        (SiteBytes::Patched, SiteBytes::Patched) => ObservedPatchState::On,
        (SiteBytes::Unknown, _) | (_, SiteBytes::Unknown) => ObservedPatchState::Unknown,
        _ => ObservedPatchState::Mixed,
    }
}

fn unique_signature_offset(
    text: &[u8],
    site: PatchSiteName,
    prefix: &[u8],
    suffix: &[u8],
) -> Result<usize, RepeatQuestError> {
    let signature_len = prefix
        .len()
        .checked_add(SIGNATURE_WILDCARD_BYTES)
        .and_then(|len| len.checked_add(suffix.len()))
        .ok_or(RepeatQuestError::AddressOverflow)?;
    let mut found = None;
    let mut count = 0;
    for (offset, window) in text.windows(signature_len).enumerate() {
        if window.get(..prefix.len()) == Some(prefix)
            && window.get(prefix.len() + SIGNATURE_WILDCARD_BYTES..) == Some(suffix)
        {
            found = Some(offset);
            count += 1;
        }
    }
    if count == 1 {
        Ok(found.expect("one signature match has an offset"))
    } else {
        Err(RepeatQuestError::SignatureCount { site, count })
    }
}

fn find_patch_offsets(text: &[u8]) -> Result<PatchOffsets, RepeatQuestError> {
    let reset = unique_signature_offset(text, PatchSiteName::Reset, RESET_PREFIX, RESET_SUFFIX)?
        .checked_add(RESET_PATCH_OFFSET)
        .ok_or(RepeatQuestError::AddressOverflow)?;
    let getter =
        unique_signature_offset(text, PatchSiteName::Getter, GETTER_PREFIX, GETTER_SUFFIX)?
            .checked_add(GETTER_PATCH_OFFSET)
            .ok_or(RepeatQuestError::AddressOverflow)?;
    Ok(PatchOffsets { reset, getter })
}

#[cfg(test)]
mod tests {
    const RESET_SIGNATURE: &[u8] = &[
        0x48, 0x83, 0xB8, 0x08, 0xC1, 0x01, 0x00, 0x00, 0xC7, 0x80, 0x24, 0xC1, 0x01, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x0F, 0x84, 0x11, 0x22, 0x33, 0x44, 0xC6, 0x87, 0x24, 0x06, 0x00, 0x00,
        0x00, 0x48, 0x8D, 0x8F, 0x28, 0x06, 0x00, 0x00, 0x31, 0xD2, 0x45, 0x31, 0xC0, 0x44, 0x89,
        0x01, 0x85, 0xDB, 0x75, 0x15,
    ];
    const GETTER_SIGNATURE: &[u8] = &[
        0x48, 0x83, 0xC1, 0x15, 0xEB, 0x0C, 0xB9, 0x24, 0x06, 0x00, 0x00, 0x48, 0x03, 0x0D, 0x55,
        0x66, 0x77, 0x88, 0x0F, 0xB6, 0x01, 0x48, 0x83, 0xC4, 0x20,
    ];

    fn signature_fixture_with_counts(reset_count: usize, getter_count: usize) -> Vec<u8> {
        let mut text = vec![0u8; 0x700];
        for offset in [0x100, 0x300].into_iter().take(reset_count) {
            text[offset..offset + RESET_SIGNATURE.len()].copy_from_slice(RESET_SIGNATURE);
        }
        for offset in [0x200, 0x500].into_iter().take(getter_count) {
            text[offset..offset + GETTER_SIGNATURE.len()].copy_from_slice(GETTER_SIGNATURE);
        }
        text
    }

    fn signature_fixture() -> Vec<u8> {
        signature_fixture_with_counts(1, 1)
    }

    #[test]
    fn finds_each_repeat_quest_signature_once() {
        assert_eq!(
            super::find_patch_offsets(&signature_fixture()).unwrap(),
            super::PatchOffsets {
                reset: 0x128,
                getter: 0x212,
            }
        );
    }

    #[test]
    fn rejects_missing_or_duplicate_signatures() {
        assert_eq!(
            super::find_patch_offsets(&signature_fixture_with_counts(0, 1)),
            Err(super::RepeatQuestError::SignatureCount {
                site: super::PatchSiteName::Reset,
                count: 0,
            })
        );
        assert_eq!(
            super::find_patch_offsets(&signature_fixture_with_counts(1, 2)),
            Err(super::RepeatQuestError::SignatureCount {
                site: super::PatchSiteName::Getter,
                count: 2,
            })
        );
    }

    #[test]
    fn classifies_original_patched_mixed_and_unknown_bytes() {
        assert_eq!(
            super::classify_pair(super::RESET_ORIGINAL, super::GETTER_ORIGINAL),
            super::ObservedPatchState::Off
        );
        assert_eq!(
            super::classify_pair(super::RESET_PATCHED, super::GETTER_PATCHED),
            super::ObservedPatchState::On
        );
        assert_eq!(
            super::classify_pair(super::RESET_PATCHED, super::GETTER_ORIGINAL),
            super::ObservedPatchState::Mixed
        );
        assert_eq!(
            super::classify_pair([0x90; 3], super::GETTER_ORIGINAL),
            super::ObservedPatchState::Unknown
        );
    }
}

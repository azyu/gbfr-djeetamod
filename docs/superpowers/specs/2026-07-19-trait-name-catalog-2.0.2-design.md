# Granblue Fantasy: Relink 2.0.2 Trait Name Catalog Design

Date: 2026-07-19  
Target: Granblue Fantasy: Relink Endless Ragnarok 2.0.2 on Windows x64  
Scope: First delivery stage only—restore Korean and English equipment-trait names

## Problem

The equipment analysis currently reads the primary and secondary traits from the twelve equipped sigils, but some rows render only `알수없음`. This is not a hook decoding failure. The bundled trait-name resources were last updated from 1.3.x data, while the 2.0.2 trait-cap catalog contains newer Endless Ragnarok traits.

The current data sets contain:

- 261 distinct entries in the 2.0.2 `skill_status` cap catalog;
- 230 public symbolic keys in the form `SKILL_xxx_xx`;
- 31 raw eight-digit hash keys without a symbolic name relationship; and
- 165 entries in the current Korean and English trait-name resources.

Therefore 65 public symbolic traits need official 2.0.2 names added. The 31 raw hashes must not be given invented names.

## Delivery Roadmap

Equipment coverage will be expanded through four independent design, plan, implementation, and packaging cycles:

1. Restore the 2.0.2 Korean and English trait-name catalogs.
2. Capture and total weapon and equipped wrightstone traits.
3. Capture and total the traits from up to four equipped summons.
4. Capture master traits that directly participate in the ordinary trait-level total.

Each stage is complete only after automated verification and comparison with the in-game equipment UI. This document specifies stage 1 only. Until later stages pass their controlled tests, the product continues to describe the analysis as the primary and secondary traits of twelve equipped sigils.

## Safety and Data Provenance

- Extraction is read-only and uses the installed 2.0.2 game assets.
- The game executable hash must equal the version-pinned value already recorded by the project.
- Extracted `.tbl`, `.msg`, and intermediate SQLite files remain in a temporary directory and are never committed.
- The research documentation records SHA-256 values for the executable, extracted inputs, and generated outputs, plus the exact regeneration command.
- Names come only from the Korean and English game localization assets. No missing name is guessed or machine-translated.

The source inputs are:

- `system/table/skill_status.tbl`;
- `system/table/text/ko/text.msg`; and
- `system/table/text/en/text.msg`.

GBFRDataTools 2.0.0 can extract these files from the installed archive. `text.msg` is MessagePack data whose rows contain identifiers such as `TXT_SKILL_020_00` and their localized text.

## Catalog Generation

The existing `build_trait_caps` generator remains the single version-pinned trait catalog tool. It is extended to consume the Korean and English `text.msg` files in addition to the `skill_status` SQLite database.

For each distinct `skill_status.Key`:

1. A symbolic `SKILL_xxx_xx` key is hashed with the same custom XXHash32 algorithm already used for cap generation.
2. Its localized name is joined from the `TXT_<symbolic key>` row in each language's `text.msg`.
3. The existing JSON shape is emitted under the eight-digit lowercase trait ID:

```json
{
  "dc584f60": {
    "key": "SKILL_020_00",
    "text": "대미지 상한"
  }
}
```

The generated outputs are:

- `src-tauri/lang/ko/traits.json`;
- `src-tauri/lang/en/traits.json`; and
- the existing `src-tauri/assets/trait-caps.json`.

The two name catalogs contain all 230 public symbolic traits. Raw eight-digit `skill_status` keys are retained in the cap catalog but are not written to a name catalog because no verified symbolic-name relationship exists.

Generation fails before writing any output when:

- the game hash is wrong;
- either localization file is missing or malformed;
- a public symbolic trait lacks a Korean or English name;
- a name is empty;
- two symbolic keys resolve to the same trait ID; or
- output validation fails.

All outputs are computed and validated before any repository file is replaced, preventing partially updated language and cap catalogs.

## Runtime Presentation

`translateTraitId` keeps using the selected i18next trait namespace. When no localized record exists, it returns a diagnostic fallback that always includes the ID:

- Korean: `알 수 없는 특성 (0x0151cf9e)`
- English: `Unknown trait (0x0151cf9e)`

The analysis row continues to show the captured level and cap state. A captured ID without a verified name still uses its known cap when the cap catalog contains that ID. If the ID is absent from the cap catalog as well, the row remains explicit as `15 / —` and `최대치 미확인`; absence of a name never hides the numeric trait identity.

This stage does not change:

- hook signatures or memory offsets;
- the wire protocol;
- trait-level aggregation;
- the twelve-sigil capture scope; or
- the hexadecimal sigil item ID shown in source details.

## Component Boundaries

- `src-tauri/src/bin/build_trait_caps.rs` owns version-pinned extraction-input validation, MessagePack row decoding, trait ID derivation, joins, and atomic output preparation.
- `src-tauri/lang/{ko,en}/traits.json` are generated runtime name resources.
- `src-tauri/assets/trait-caps.json` remains the generated cap resource and provides the complete 261-ID classification set.
- `src/utils.ts` owns runtime name lookup and the ID-bearing fallback.
- `docs/research/` records provenance, hashes, counts, and reproduction commands without storing proprietary source data.

## Verification

Generator tests cover:

- symbolic key hashing and localized-name joining;
- `SKILL_020_00` mapping to trait ID `0xDC584F60`, Korean `대미지 상한`, English `Damage Cap`, and cap 65;
- at least one Endless Ragnarok symbolic trait absent from the 1.3.x catalog;
- rejection of missing Korean or English public names;
- rejection of empty names and trait-ID collisions;
- classification of raw eight-digit keys without inventing names; and
- validation-before-write behavior.

Catalog consistency tests require:

- 261 cap records;
- 230 Korean public-name records;
- 230 English public-name records;
- identical ID and symbolic-key sets between Korean and English;
- 31 cap-only raw hash entries; and
- no blank names or duplicate IDs.

Frontend tests require:

- a known trait to render its official localized name;
- a missing trait to render `알 수 없는 특성 (0x...)` in Korean;
- the same missing trait to render `Unknown trait (0x...)` in English; and
- the equipment analysis table to preserve its level and state values for a missing name.

The normal project format, lint, type-check, frontend test, production build, Rust test, release hook build, and MSI packaging gates remain mandatory. Manual stage acceptance is an observed formerly-unknown trait rendering either its official name or the ID-bearing fallback in an offline or private 2.0.2 session.

## Later Stages

Weapon, wrightstone, summon, and master-trait work starts only after this catalog stage is accepted. Each later stage must establish its memory layout with one-variable controlled equipment changes, retain per-source attribution, and compare the resulting total against the game UI. Unknown or unverified inputs are never converted to zero or silently reported as complete.

## References

- Cygames, Ver. 2.0.2 Update Information: https://relink-ragnarok.granbluefantasy.com/en/updates/381/
- Cygames, Endless Ragnarok Systems: https://relink-ragnarok.granbluefantasy.com/en/systems/
- Nenkai, GBFRDataTools: https://github.com/Nenkai/GBFRDataTools
- False Spring, gbfr-resources: https://github.com/false-spring/gbfr-resources
- Existing research: `docs/research/2026-07-18-gbfr-er-2.0.2-trait-overflow.md`

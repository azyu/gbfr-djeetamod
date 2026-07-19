# Equipment Analysis Scope Copy Design

## Goal

Make the equipment-analysis screen state both what is included and what is not included in the current total, so users do not mistake the twelve-sigil result for a complete build total.

## User-facing copy

Korean:

> 현재는 장착 진 12개의 주·보조 특성만 합산합니다. 무기·가호석·소환석·마스터 특성은 아직 포함되지 않습니다.

English:

> Currently, only primary and secondary traits from the 12 equipped sigils are totaled. Weapon and wrightstone, summons, and master traits are not included yet.

## Scope

- Change only the Korean and English `ui.equipment-analysis.scope` translations.
- Keep the existing placement and styling in `EquipmentAnalysis`.
- Add localization assertions that protect the full inclusion and exclusion wording.
- Do not change equipment capture, trait totals, or the README in this change.

## Verification

- First update the localization test so it fails against the old wording.
- Apply the two translation changes and make the focused test pass.
- Run the repository-required formatting, linting, type checking, frontend tests, production build, Rust build/tests, and MSI packaging.
- Verify that the release and bundled hook hashes match and refresh documented package hashes if the MSI changes.

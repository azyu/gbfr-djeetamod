# Bilingual README User Guide Design

## Goal

Add an end-user guide to `README.md` that explains how to operate Djeeta MOD in Korean and English without duplicating the existing developer build, artifact hash, license, or compatibility sections.

## Structure

The Korean guide appears first as a complete section. An English guide with the same scope and meaning follows it. Content is not interleaved by language because uninterrupted sections are easier to scan and link.

Each language section covers:

1. Installing the MSI, starting the game, and starting Djeeta MOD.
2. The management sidebar:
   - Damage Meter switch
   - Sigil Trait Cap Analysis
   - Battle Records
   - Settings
3. Moving the always-on-top meter by dragging its header.
4. Reading the sigil analysis totals, `MAX`, and overflow values.
5. Opening saved battle records.
6. Basic troubleshooting when the management window, meter, game connection, or equipment data is unavailable.
7. Why `hook.dll` is required.
8. The recommendation to test in an offline or private session first.

## Content Boundaries

- Describe only behavior currently implemented in the application.
- Keep the existing warning that game 2.0.2 compatibility is not confirmed until the manual smoke-test checklist is complete.
- Do not claim that every trait cap is verified. The guide explains that unknown caps are shown as unverified.
- Do not provide manual DLL injection instructions. Installation and normal application startup are the supported user workflow.
- Do not repeat source build commands, SHA-256 values, upstream credits, or license details inside the guides.

## Placement

Insert the guides after the existing behavior overview and before performance impact and safety warnings. This keeps basic product context and operation together while preserving warnings, hashes, developer instructions, and attribution below.

## Verification

- Confirm Korean and English headings and corresponding subsections exist.
- Check that menu names match the current localization resources.
- Check Markdown formatting and relative links.
- Run the repository-required format, lint, type-check, frontend test, frontend build, Rust build/test, and MSI packaging gates after the README change, as requested by the maintainer.

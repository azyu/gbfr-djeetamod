# Repeat Quest Disable Restoration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make disabling Unlimited Repeat Quest reliably restore the exact code sites enabled by Djeeta MOD.

**Architecture:** Signature discovery matches the complete stable instruction context while excluding only each three-byte patch target. The existing classifier and restoration transaction then accept only the known original/patched values, allowing explicit OFF and startup recovery to rediscover patched sites without weakening write safety.

**Tech Stack:** Rust nightly-2024-05-04, Tauri 1, Windows process-memory APIs, existing fake-memory Rust tests.

## Global Constraints

- Support only the pinned Granblue Fantasy: Relink Endless Ragnarok 2.0.2 executable.
- Never overwrite unknown target bytes.
- Do not persist process addresses or ON state across Djeeta MOD launches.
- Keep writable process access isolated to `src-tauri/src/repeat_quest.rs`.
- Preserve transactional rollback and read-back verification.

---

### Task 1: Rediscover and restore patched sites

**Files:**
- Modify: `src-tauri/src/repeat_quest.rs`
- Test: `src-tauri/src/repeat_quest.rs`

**Interfaces:**
- Produces: `find_patch_offsets` support for original, patched, and unknown target bytes within otherwise exact unique signatures.
- Consumes: existing signature discovery, `PatchMemory`, and strict target-byte classification.

- [x] **Step 1: Write the failing regression test**

Add a test that resolves the original fixture, replaces both target instructions
with patched bytes, rediscovers the same `PatchOffsets`, and restores through
the existing `restore_patch` transaction.

- [x] **Step 2: Run the focused test to verify RED**

Run: `cargo test --locked --package gbfr-logs repeat_quest::tests::rediscovers_and_restores_sites_after_enable_changes_the_target_instructions`

Expected: assertion failure because both original-only signatures disappear.

- [x] **Step 3: Implement stable-context signature matching**

Update signature matching to ignore only the three bytes at each known patch
offset while continuing to match every other fixed byte and the existing
four-byte displacement wildcard. Keep `restore_patch` unchanged.

- [x] **Step 4: Add safety regression coverage**

Cover an unknown target value with zero restoration writes, and prove bytes
immediately adjacent to each patch target remain part of the exact signature.

- [x] **Step 5: Verify focused and complete checks**

Run the repeat-quest Rust tests first, then `npm.cmd run format-check`,
`npm.cmd run lint`, `npm.cmd run tsc`, `npm.cmd test -- --run`,
`npm.cmd run build`, `cargo build --release --locked --package hook`, and
`cargo test --workspace --all-targets --locked`.

- [x] **Step 6: Review and commit the isolated fix**

Review `git diff --check`, ensure every changed line belongs to the restoration
bug, and create one implementation commit containing the regression tests,
backend fix, design, and plan.

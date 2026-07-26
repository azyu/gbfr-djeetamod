<!-- markdownlint-disable MD013 -->

# Granblue Fantasy: Relink 2.0.2 Conflux Auto Re-entry Probe

## Status

**External read-only capture is runnable; the injected observation probe is
still pending.** The result/re-entry native-candidate gate has a
reference-assisted static pass, while reward-ID mapping remains blocked; see
[`../research/2026-07-25-conflux-auto-reentry-candidates.md`](../research/2026-07-25-conflux-auto-reentry-candidates.md).

This checklist is the evidence contract for a future observation-only probe and
the later production capability. It is not evidence that Djeeta MOD is
compatible with the game.

## Pinned Target

| Property | Required value |
| --- | --- |
| Game version | `2.0.2` |
| Architecture | Windows x64 |
| SHA-256 | `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F` |
| Session | Offline or private |

## Probe Build Contract

TODO after the static gate passes:

```powershell
cargo build --release --locked --package hook --features conflux-retry-probe
```

The feature name and command are reserved by the implementation plan; they do
not exist yet. A future probe must:

- observe only and never invoke a game callback;
- emit fixed counters and bounded validation outcomes;
- omit object addresses, inventory contents, account data, and free-form
  memory dumps;
- remain absent from ordinary release builds;
- refuse to install on a version or hash mismatch;
- treat duplicate signature matches as unsupported.

## Observation Run

Record the PID only in local test notes. Do not commit it.

| Boundary/scenario | Date | Start counters | End counters | Validation outcome | PASS/MISMATCH | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| Route selection opens with two combat routes | 2026-07-25 | Portal candidates not present on prior Power screen | Two `BaEndlessPortal` objects and two `ControllerEndlessGateIcon` objects across all corresponding vtables | Object families and exact live pair count match the two visible combat portals | PASS | Route enum, slot field, pairing, and interaction callback remain unverified |
| Route selection shows rare-Power appearance-rate bonus | 2026-07-25 | Prior portal pair completed | Two new stable portal/icon object pairs; visible bonus was on the left choice | Live object pair matches the visible two-choice screen, but the modifier field is not mapped | MISMATCH | Expected selection is left; field and callback remain unverified |
| Route selection opens with one route | 2026-07-25 | Prior two-route screen completed | One `BaEndlessPortal` object but two `ControllerEndlessGateIcon` objects | Live portal count matches the sole visible route; icon count retains one stale/reused object | PASS | Sole portal wins; active-icon discriminator and interaction callback remain unverified |
| Post-mid-boss route offers continuation or return | 2026-07-25 | Mid-boss result and Power selection completed | User observed `공무의 경지` and `귀환하기` choices | Expected choice is always continuation, but destination enum is not mapped | MISMATCH | Continuation overrides modifier and route-type priority |
| Endless area-result information opens and closes | 2026-07-25 | One active-looking `ControllerEndlessEventResult`; one stable `ControllerEndlessBossResult` | Event-result fingerprint changed after manual OK; boss-result fingerprint stayed identical; Power screen appeared | Event-result family reflects the visible transition, but allocation count alone does not prove active state | MISMATCH | Active/ready field and exact OK callback remain unresolved |
| Mid-boss reward result leads to normal Power selection | 2026-07-25 | Prior boss route completed | User observed one reward-result OK screen, dismissed it, and then observed normal Power selection; event-result and boss-result fingerprints changed across the transition | Correct sequence is boss result acknowledgement followed by Power selection; no single-choice reward exists | MISMATCH | Boss active/ready field and exact OK callback remain unresolved |
| `공무의 경지` monk root opens | 2026-07-25 | Post-mid-boss continuation selected | One `EtEndlessModeShop`, one `ControllerEndlessShopDialog`, and one `ControllerEndlessShopTop`; two portal objects were already allocated | Shop relationship coexists with outgoing portals, so monk handling must have priority | MISMATCH | Active-state fields and monk interaction/root-entry callbacks remain unresolved |
| Monk CP confirmation opens | 2026-07-25 | `경지의 힘 획득` selected | Shop objects remained one each; confirmation displayed 1,000 CP cost and affordable balance | Exact object family observed, but affordability field and confirm callback are unresolved | MISMATCH | Do not parse localized text or assume the observed cost is invariant |
| Monk five-choice Power list opens | 2026-07-25 | CP use confirmed | Five visible vertical choices; `ShopDialog` and `ShopTop` fingerprints changed; ordinary `MenuEndlessResultReward` count stayed zero | This is distinct from the ordinary three-choice Power screen; Chaos was visible in choice five and expected target was index zero | MISMATCH | Visible vector, Chaos/grade fields, selected index, and callbacks remain unresolved |
| Monk Power acquisition opens and closes | 2026-07-25 | Five-choice index zero confirmed | `ControllerEndlessBuffAcquired` changed from zero to exactly one, then cleared after manual OK | Dedicated acquisition controller matches the visible OK boundary | MISMATCH | Active/ready field and exact OK callback remain unresolved |
| Monk purchase loop repeats while affordable | 2026-07-25 | Acquisition OK closed with at least 1,000 CP remaining | Game returned to the CP confirmation with the next projected balance | Repetition is game-driven and requires no root-menu re-entry | MISMATCH | Exact affordability and transition fields remain unresolved |
| Monk purchase loop ends when CP is insufficient | 2026-07-25 | Repeated purchase left 463 CP | Game automatically returned to the monk root menu; two portals remained allocated; no purchase-dialog cancel was used | Automation must wait for automatic subflow exit, close the root menu once, then permit portal selection | MISMATCH | Root active/inactive acknowledgement and close callback remain unresolved |
| Power selection opens with a top-grade tie | 2026-07-25 | Power menus `0` in battle | Three visible two-star choices; four primary menu objects plus four per secondary vtable | Type family identified, but live-object count includes a fourth hidden/template object and no controller vtable | MISMATCH | Selection rule is highest grade, then leftmost; fields/callbacks remain unresolved |
| Power selection opens with mixed grades | 2026-07-25 | Result popup dismissed | Visible grades `[1, 1, 2]`; four primary menu objects plus matching secondary vtables | Expected choice is visible index two, but grade-to-object mapping is unresolved | MISMATCH | Keep screen unpromoted until field mapping is independently verified |
| Power selection contains Chaos type | 2026-07-25 | Prior boss-result acknowledgement completed | User identified Chaos as present; four primary menu objects plus matching secondary vtables remained allocated | Expected choice is display index zero, but Chaos type-to-object mapping is unresolved | MISMATCH | Chaos overrides grade priority |
| Final-boss field return gate opens | 2026-07-25 | Floor-five final boss defeated | One `BaEndlessPortal`; two primary gate-icon objects; event, boss-result, and reward controllers all zero | Visible `귀환하기` field interaction is a distinct required boundary before final reward | MISMATCH | Final-boss phase/destination and interaction callback remain unresolved |
| Floor-four final reward screen opens | 2026-07-26 | Offline/private floor-four reward selection visible | One `ControllerEndlessResultReward`, one `MenuResultRewardTreasure`, two `MenuResultReward`, four Endless result/info owners, eight visible reward rows, and one retained `BaEndlessPortal`; ordinary Power and result-dialog owners zero | The core reward-screen owner structure matches floor five, while row count and available rewards are floor-dependent | PASS | [Captured screen](evidence/conflux-floor-4-reward-select-2026-07-26.jpg); internal reward IDs and selectable flags remain TODO |
| Floor-five final reward screen opens | 2026-07-25 | Final-boss field return gate manually activated | One `ControllerEndlessResultReward`, one `MenuResultRewardTreasure`, two `MenuResultReward`, four Endless result/info owners, and eleven visible reward rows; `MenuEndlessResultReward` zero | Actual final-reward owner is distinct from ordinary Power selection | PASS | Row vector, selectable flag, internal reward IDs, indices, and callbacks remain unresolved |
| Configured floor-five reward exists below first row | TODO | TODO | TODO | TODO | TODO | |
| Configured floor-five reward is absent or unselectable | TODO | TODO | TODO | TODO | TODO | |
| Acquisition-count display does not alter configured-ID selection | TODO | TODO | TODO | TODO | TODO | |
| Floor-five reward transitions directly to TOTAL RESULTS | 2026-07-25 | Final reward manually selected | No intermediate dialog; result-reward and dialog-reward controllers stayed zero; the final-reward/result owners remained allocated with changed fingerprints | The floor-five state machine must not wait for a speculative reward-conversion confirmation | PASS | Exact transition state remains unresolved |
| TOTAL RESULTS becomes ready | 2026-07-25 | Direct transition from final reward | Visible `TOTAL RESULTS` page with `다음으로`; one each of the four Endless result/info owners remained live | Visible readiness is confirmed, but the ready field and next callback/event are not mapped | MISMATCH | Capture dismissal and return-destination successor |
| Return-destination dialog opens | 2026-07-25 | `TOTAL RESULTS` manually advanced | Visible three-choice `결과 확인` dialog; thirteen `EndlessResultCitySelect` FSM objects allocated | Dialog transition is observed, but allocation count does not identify the active FSM | MISMATCH | Active-state discriminator and callback remain unresolved |
| Tredame Palace destination is focused | 2026-07-25 | Return-destination dialog open | `트르담 궁으로 돌아가기` was the focused first item above town and cancel | Expected target is visually confirmed, but the internal destination enum is not mapped | MISMATCH | Do not match localized text or assume index zero without enum corroboration |
| Tredame Palace loads with re-entry gate | 2026-07-25 | Tredame destination manually confirmed | One `BaEndlessPortal`; primary gate-icon, Endless-top/frame, and difficulty controllers zero; preset-party allocation one | Palace re-entry gate uses the portal family and is distinct from the presumed Endless-top owner | PASS | Place/phase discriminator, interaction callback, and range-independent behavior remain unresolved |
| Tredame Palace gate interaction opens | 2026-07-25 | Palace gate baseline captured | Manual interaction opened `탐사 파티 편성`; portal fingerprint changed; Endless top/frame/difficulty and dialog-reward families changed zero to one | Expected visible successor appeared | PASS | Native interaction callback and range-independent behavior remain unresolved |
| Party formation becomes confirmable | 2026-07-25 | Palace gate manually activated | Visible current party and `탐사 시작`; top/frame/difficulty and dialog-reward objects one; preset-party fingerprint unchanged from hidden baseline | Allocation of `ControllerPresetParty` alone is not the active-screen discriminator | MISMATCH | Active relationship and exact start callback remain unresolved |
| Current depth is focused | 2026-07-25 | `탐사 시작` manually confirmed | Visible `탐사 심도 선택` focused `제5층 숙성`; difficulty/top/frame fingerprints changed while supporting objects stayed stable | Screen transition and visible target are confirmed, but the internal selected-depth field is not mapped | MISMATCH | Corroborate depth value five and exact confirm callback |
| Depth confirmation re-enters battle | 2026-07-25 | Focused depth manually confirmed | Direct transition to floor-five battle; difficulty/top/frame, palace portal, and dialog-reward objects cleared to zero; preset-party remained allocated | No separate final-ready prompt exists in the observed flow | PASS | Exact depth-confirm callback and battle-successor discriminator remain unresolved |

## Negative Scenarios

Every negative scenario must leave every boundary-specific counter unchanged.
A counter change is a `MISMATCH`, even if no visible action occurs.

| Negative scenario | Date | Start counters | End counters | Validation outcome | PASS/MISMATCH | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| Ordinary battle | 2026-07-25 | Floor-five re-entry completed | No visible result UI; event-result and boss-result objects one each; all re-entry UI and portal objects zero | Result-controller allocation count alone is not an active-state signal | MISMATCH | Corroborate hidden/inactive fields before allowlisting OK actions |
| Ordinary quest results | TODO | TODO | TODO | TODO | TODO | |
| Unrelated confirmation dialog | TODO | TODO | TODO | TODO | TODO | |
| Unknown single-OK dialog during Conflux | TODO | TODO | TODO | TODO | TODO | Must remain untouched and fail closed if it blocks the expected transition |
| Fall recovery | TODO | TODO | TODO | TODO | TODO | |
| Boss mechanic prompt | TODO | TODO | TODO | TODO | TODO | |
| Unrelated town interaction | TODO | TODO | TODO | TODO | TODO | |
| Enter and leave an ordinary gate/menu | TODO | TODO | TODO | TODO | TODO | |
| Game process restart before enabling | TODO | TODO | TODO | TODO | TODO | |

## Promotion Rule

Any of the following blocks promotion to an action-capable capability:

- a missing positive call;
- more than one positive call for a single transition without a documented
  reason;
- an out-of-order boundary;
- any negative-case counter change;
- an invalid vtable or owner validation;
- an allowlisted dialog issuing more than one OK for one stable identity;
- an unknown or unrelated single-OK dialog being accepted;
- a version/hash mismatch;
- a signature with zero or multiple matches;
- an ABI, field offset, callback, or event value supported by only one
  uncorroborated observation.

After every row passes, copy the fixed evidence into the native-candidate
document, re-run its static gate, and review the diff before implementing any
game callback.

## Production Acceptance — Future

This section remains blocked until both the static and observation gates pass.

| Scenario | Date | Cycle/stage log | Result | PASS/MISMATCH | Notes |
| --- | --- | --- | --- | --- | --- |
| Full cycle 1: reward to re-entry | TODO | TODO | TODO | TODO | |
| Full cycle 2: reward to re-entry | TODO | TODO | TODO | TODO | |
| Manual cancellation disables the run | TODO | TODO | TODO | TODO | |
| Control-client disconnect fails closed | TODO | TODO | TODO | TODO | |
| Process restart starts OFF | TODO | TODO | TODO | TODO | |

Passing automated tests or builds does not complete this checklist.

## Minimal Timer Probe

The superseding minimal design uses the game's own unattended progression.
Run the read-only timer sampler immediately after a route, Power, or single-OK
screen appears with `극돈공소 무조작 시 설정` ON:

```powershell
$env:DJEETA_CONFLUX_TIMER_PROBE = '1'
cargo run --locked --package gbfr-logs --example probe_conflux_ui
```

The probe must report only target labels, object ordinals, relative offsets,
numeric kinds, and bounded values. Raw object addresses are prohibited.

| Scenario | Date | Objects | Candidate | Result | Notes |
| --- | --- | ---: | --- | --- | --- |
| Ordinary battle baseline | 2026-07-25 | 25 | none | PASS | No active unattended choice screen |
| Timer manager on unattended choice | 2026-07-26 | one global manager | mode `1`; initial `60.000`; current `13.818 -> 13.318`; defaults `[60,60,30,30,30,30,30,30,60,30,30]`; notice `3.000` | PASS | Independently corroborated by `EndlessAbandonedAutoTimer` and `ControllerEndlessAutoPlayHud` access sites |
| Route choice, game unattended ON | TODO | TODO | TODO | TODO | Visible about-two-second progression remains required |
| Power choice, game unattended ON | TODO | TODO | TODO | TODO | Corroborate the same duration boundary |
| Single-OK screen, game unattended ON | TODO | TODO | TODO | TODO | Corroborate the notice boundary |
| Reversible 1/2-second data patch | 2026-07-26 | one global manager | exact original -> fast -> original configuration with readback; waiting screen advanced to the floor-four battle | PASS | Explicit offline/private ignored test; post-transition read-only probe showed original notice/default configuration; code and transition timing were not patched |
| Management-page switch round trip | 2026-07-26 | one global manager | page OFF: notice `3.000`, original defaults; page ON: notice `1.000`, eleven `2.000` defaults; page OFF again: notice `3.000`, original defaults `[60,60,30,30,30,30,30,30,60,30,30]` | PASS | Verified against the live pinned game process after repairing debug startup and opening the built management page; the manager `mode` field was not used as the Djeeta switch state |
| Automation OFF restores original delay | 2026-07-26 | one global manager | shortened active screen advanced; next unattended screen reported `initial=60.000`, `current=47.333 -> 46.833`, and original defaults `[60,60,30,30,30,30,30,30,60,30,30]` | PASS | Confirmed by both the backend live round trip and the built management-page ON/OFF cycle |

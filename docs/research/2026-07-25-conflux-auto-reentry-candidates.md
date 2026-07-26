<!-- markdownlint-disable MD013 -->

# Conflux Auto Re-entry 2.0.2 Native Candidates

## Decision

**REFERENCE-ASSISTED STATIC PASS for the result/re-entry callbacks; reward-ID
mapping remains blocked.**

The installed executable is the expected Granblue Fantasy: Relink 2.0.2 image,
and several Endless-mode controllers can be identified independently through
MSVC RTTI and their complete-object locators. The final reward controller has a
unique update candidate that calls reward-menu methods directly.

The independent pass did not distinguish every required confirmation path.
After that gate failed, the pinned user-supplied reference archive was inspected
as allowed by the approved analysis ladder. Its relevant method names, masked
signatures, and bounded field hypotheses were then checked independently
against the pinned game executable. All twelve candidate function signatures
occur exactly once in executable code. This promotes observation-only work for
the final reward, TOTAL RESULTS, return dialog, Tredame gate, party, and
difficulty boundaries.

This does not yet promote configured reward selection: the 12-byte live reward
row has a verified favorite byte, but its internal reward ID and the versioned
Korean/English catalog still need a floor-five live cross-check.

## Research Input

| Property | Verified value |
| --- | --- |
| Product version | `2.0.2` |
| File version | `2.0.2` |
| File size | `123,522,016` bytes |
| SHA-256 | `63340832BCF731FBC97796F686B05C988418E83D451D4A49B2244A85D00E297F` |
| Image base | `0x140000000` |
| `.text` RVA | `0x00001000` |
| `.text` raw size | `0x049AFA00` |
| `.rdata` RVA | `0x049B1000` |
| `.data` RVA | `0x06B36000` |

The local installation path is deliberately omitted.

## Independently Recovered Type Evidence

The following RVAs were recovered by:

1. locating the exact decorated MSVC type-descriptor name;
2. resolving the x64 image-relative complete-object locator;
3. checking the locator's self RVA;
4. resolving the locator reference immediately before the vtable; and
5. requiring vtable entries to point into `.text`.

| Semantic type | Primary vtable RVA | Evidence status |
| --- | ---: | --- |
| `ui::component::ControllerEndlessResultReward` | `0x0607E1D8` | RTTI/vtable verified |
| `ui::component::MenuEndlessResultReward` | `0x06086DB0` | RTTI/vtable verified |
| `ui::component::ControllerResultReward` | `0x05C62128` | RTTI/vtable verified |
| `ui::component::MenuResultReward` | `0x0618A490` | RTTI/vtable verified |
| `ui::component::MenuResultRewardTreasure` | `0x0618A210` | RTTI/vtable verified |
| `ui::component::ControllerEndlessResultInfo` | `0x06083A58` | RTTI/vtable verified |
| `ui::component::ControllerEndlessResultRecord` | `0x0607DBC8` | RTTI/vtable verified |
| `ui::component::ControllerEndlessResultScore` | `0x060834C8` | RTTI/vtable verified |
| `ui::component::ControllerEndlessDifficulty` | `0x05C73688` | RTTI/vtable verified |
| `ui::component::ControllerEndlessTop` | `0x05C76808` | RTTI/vtable verified |
| `ui::component::ControllerEndlessTopFrame` | `0x05F91558` | RTTI/vtable verified |
| `ui::component::ControllerPresetParty` | `0x05F83148` | RTTI/vtable verified |
| `ui::action::fsm::EndlessResultCitySelect` | `0x058E0148` | RTTI/vtable verified |

RTTI/vtable recovery proves object identity, not the meaning of an individual
virtual method or permission to call it.

## Candidate Boundary Table

`CANDIDATE` means that the executable supports a concrete place to continue
analysis. It does not authorize a detour. `BLOCKED` means that one or more
static-gate requirements remain unmet.

| Boundary | Current independent evidence | Missing evidence | Status |
| --- | --- | --- | --- |
| Route portal selection | RTTI independently identifies `BaEndlessPortal` with primary vtable RVA `0x05C8E0F0` and eleven secondary vtables, plus `ControllerEndlessGateIcon` with primary vtable RVA `0x05A6DC28` and six secondary vtables. Two route screens had exactly two stable portal objects. The first pair had two combat (`쟁투의 경지`) variants; the second displayed `희귀한 힘 출현율 UP` on the left. A later single-route screen had exactly one portal object but retained two icon objects, proving icon allocation count is not the live-route count. The user later observed the post-mid-boss `공무의 경지`/`귀환하기` pair and required continuation. | Corroborate active portal state, continuation/return destination, the rare-Power modifier, route-type enum, and generator slot index; prove portal-to-active-icon pairing while excluding stale icons; identify the internal interaction function and acknowledgement; observe one-combat and no-combat pairs. | `CANDIDATE` |
| Final-boss field return gate | After the floor-five final boss, the field displayed one `귀환하기` interaction and the objective `게이트를 통해 귀환하기`. Exactly one `BaEndlessPortal` object was live while `ControllerEndlessEventResult`, `ControllerEndlessBossResult`, and all probed reward/result controllers were zero. Two primary gate-icon objects remained allocated, so icon count was again non-authoritative. Manual interaction opened the final reward screen. | Identify and corroborate the final-boss destination/phase discriminator, distinguish this gate from an ordinary sole route, identify the native interaction callback, and prove the reward-controller transition acknowledgement. | `CANDIDATE` |
| Endless area-result acknowledgement | RTTI independently identifies `ControllerEndlessEventResult` with primary vtable RVA `0x06080928` and five secondary vtables. The visible clear-time/reward/OK screen had one live instance for every vtable. After manual OK, its primary fingerprint changed while the next Power screen appeared. A `ControllerEndlessBossResult` instance remained byte-for-byte stable across both screens, so allocation alone is not an active-state discriminator. | Identify and corroborate the active/ready state; distinguish ordinary, event, and boss result variants; identify the exact decide callback; observe dismissal acknowledgement and unrelated-dialog negatives. | `CANDIDATE` |
| Mid-boss reward-result acknowledgement | The user corrected the observed sequence to `mid-boss defeated -> reward-result OK -> normal Power selection`; there is no single-choice boss reward. On the blocking reward result, `ControllerEndlessBossResult` and `ControllerEndlessEventResult` each had one live object and both fingerprints differed from their prior hidden/post-result states. After manual OK, the next Power screen appeared and both controller fingerprints changed again. Four `MenuEndlessResultReward` objects were already allocated around the transition and do not indicate a visible single choice. The final boss follows the separate field-return-gate path. | Corroborate boss active/ready state; identify the exact OK callback and dismissal acknowledgement; repeat on another mid-boss and a hidden boss-controller negative. | `CANDIDATE` |
| `공무의 경지` monk CP loop | Live observation identified one `EtEndlessModeShop`, one `ControllerEndlessShopDialog`, and one `ControllerEndlessShopTop`. Selecting `경지의 힘 획득` opened a CP confirmation, then a five-choice vertical Power list. With Chaos visible in the fifth choice, index zero was selected. `ControllerEndlessBuffAcquired` changed from zero to exactly one on the acquisition OK screen. Closing it returned to CP confirmation while affordable. After repeated purchases left 463 CP, the game automatically returned to the monk root menu; it did not require a purchase-dialog cancel. Two `BaEndlessPortal` objects and two primary gate-icon objects remained allocated while that root menu was open. | Corroborate active states, affordability and visible-choice fields; distinguish the five-choice list from ordinary three-choice Power UI; identify monk interaction, root-entry selection, CP-confirm, Power-select/confirm, acquisition-OK, and root-close callbacks; prove the automatic insufficient-CP transition and shop-inactive acknowledgement. | `MISMATCH` |
| Power/buff selection | The first visible screen had three two-star choices. A later screen had visible grades `[1, 1, 2]` from left to right, making index two the expected choice. The user later identified a Power screen containing the Chaos type and selected the index-zero policy for that variant. Four primary `MenuEndlessResultReward` objects and four instances of every secondary vtable remained live; allocation count and bounded fingerprints did not independently identify Chaos. No `ControllerEndlessResultReward` vtable was live. | Identify visible choices without counting hidden/template objects; locate and corroborate game-computed Chaos type, grade, and selected-index fields; identify selection and confirm callbacks; verify both Chaos-index-zero and mixed-grade expected choices against live object data. | `MISMATCH` |
| Floor-five final reward row/controller update | On the observed `REWARD SELECT` screen, exactly one `ControllerEndlessResultReward`, one `MenuResultRewardTreasure`, two `MenuResultReward`, and one each of `ControllerEndlessResultInfo`, `ControllerEndlessResultRecord`, `ControllerEndlessResultScore`, and `ControllerEndlessRecordInfo` were live. Eleven reward rows were visible. `MenuEndlessResultReward` and ordinary result-reward/dialog controllers were zero, separating this screen from Power selection and later confirmation. Acquisition-count text was visible on some rows but is excluded from the approved selection policy. | Identify the bounded row vector, selectable flag, internal reward ID, selected/current indices, menu-change path, and decide callback; independently map the version-pinned floor-five reward catalog; prove first-selectable fallback and that acquisition counts do not affect selection. | `CANDIDATE` |
| TOTAL RESULTS update and ready state | Manual floor-five reward selection transitioned directly to `TOTAL RESULTS`; no intermediate confirmation was observed. The same one `ControllerEndlessResultReward`, one `MenuResultRewardTreasure`, two `MenuResultReward`, and four Endless result/info owners remained allocated with changed fingerprints. `ControllerResultReward`, `DialogRewardResult`, and result-guide controllers were zero. The visible action was `다음으로`. | Determine which result owner and state discriminate `REWARD SELECT` from ready `TOTAL RESULTS`; corroborate the ready-state field from two access sites; identify the exact next callback/event and verify the return-destination transition. | `CANDIDATE` |
| Return-destination dialog update | `ui::action::fsm::EndlessResultCitySelect` has vtable RVA `0x058E0148`; virtual slot 9 is RVA `0x01CEE1C0`. After `TOTAL RESULTS` was advanced, the visible `결과 확인` dialog offered `트르담 궁으로 돌아가기`, `마을로 돌아가기`, and `취소`, with Tredame focused first. Thirteen FSM objects were allocated, so count alone cannot identify the active instance. The function advances a bounded value read from object offset `0x38`, but the target global and semantic enum values are not yet proven. | Identify the active-instance discriminator; tie the state mutation to the three visible choices; identify Tredame and cancel values independently; verify the update ABI, callback owner, and loading-transition acknowledgement. | `CANDIDATE` |
| Tredame Palace gate initialization/update | After the observed return to Tredame Palace, exactly one `BaEndlessPortal` object was live. Primary `ControllerEndlessGateIcon`, `ControllerEndlessTop`, `ControllerEndlessTopFrame`, and `ControllerEndlessDifficulty` counts were zero; one `ControllerPresetParty` object remained allocated but was not visibly active. Manual gate interaction opened `탐사 파티 편성` and changed the portal fingerprint. | Corroborate the palace place/phase discriminator against route and final-boss portals; identify the portal interaction callback and range-independent behavior; prove the party-screen successor relationship. | `CANDIDATE` |
| Party-formation update | On the visible `탐사 파티 편성` screen, `ControllerEndlessDifficulty`, `ControllerEndlessTop`, `ControllerEndlessTopFrame`, and every `DialogRewardResult` vtable changed from zero to one. `ControllerPresetParty` remained one with the same bounded fingerprint as the hidden town baseline, disproving allocation or fingerprint change as its active discriminator. The visible action was `탐사 시작`. The preset-party slot-24 candidate at RVA `0x03CBE870` still dispatches on a bounded state at object offset `0x418`, but semantic ownership is not established. | Identify the actual active-screen relationship and `탐사 시작` callback; determine why `DialogRewardResult` participates; corroborate any required preset-party field independently; exclude the hidden town baseline and party-edit/cancel paths. | `MISMATCH` |
| Difficulty menu update | `ControllerEndlessDifficulty` primary vtable is verified. Virtual slot 26 points to RVA `0x03194790`; its first 48 bytes occur once in `.text`. After manual `탐사 시작`, the visible `탐사 심도 선택` screen focused `제5층 숙성`. `ControllerEndlessDifficulty`, `ControllerEndlessTop`, and `ControllerEndlessTopFrame` remained one each but all three fingerprints changed from the party screen; portal, preset-party, and dialog-reward fingerprints remained stable. | Prove the selected-depth field and its bounds from two access sites; identify confirm/cancel callbacks; corroborate that the selected internal depth is five and that unchanged supporting allocations cannot authorize confirmation. | `CANDIDATE` |

## Provisional Unique Function Starts

These patterns are research anchors only. They are not approved production
signatures. Each listed byte sequence occurs exactly once in the pinned
executable's `.text` section.

### Final reward controller slot 31

RVA: `0x042E35E0`

```text
55 41 57 41 56 41 55 41 54 56 57 53
48 81 EC 68 05 00 00 48 8D AC 24 80 00 00 00
C5 78 29 85 D0 04 00 00 C5 F8 29 BD C0 04 00 00
```

Observed static relationships:

- calls reward-menu methods at RVAs `0x03C40A40`, `0x03C3EA70`, and
  `0x03C40920`;
- accesses the controller's child-object range at offsets `0x1A0` and `0x1A8`;
- checks several bounded menu counts before operating on child objects.

The function is large and stateful. Hooking it before its ABI and state fields
are proven would create false-positive and stale-object risks.

## Reference-assisted relocation result — 2026-07-26

Reference input was accepted only after its SHA-256 matched
`02FE3756F47118D5F957EE597C4C4776877AE906A9D373A913C1D0D9FADCBA71`.
The inspected artifacts remained in a temporary directory outside the
repository and were not executed or loaded.

| Boundary | Reference hypothesis | Independent relocation | Promotion |
| --- | --- | --- | --- |
| Final reward update | controller update receives one controller pointer; active controller/menu, callback ownership, row vector, and indices are validated before change/decide | masked prologue is unique at RVA `0x042DEB70`; change and decide prologues are unique at `0x029EFAE0` and `0x029EF990` | `STATIC PASS` for observation and index action; reward ID remains blocked |
| Single result/city dialog | dialog update receives one controller pointer and validates the exact controller/menu/callback owners | update and decision callback are unique at `0x03B8BB40` and `0x03B8BDA0` | `STATIC PASS` for allowlisted flow stages only |
| TOTAL RESULTS | controller state `2`, active UI root, and animation state `6`; next is published as event hash `0xB0D3541F` | update prologue is unique at RVA `0x042E35E0`; the independently recovered controller vtable is `0x0607E1D8` | `STATIC PASS` |
| Tredame gate | exact gate type, mode/state fields, initialization capture, and game-thread interaction | initialize, interact, and update prologues are unique at RVAs `0x036DDFD0`, `0x036DE0E0`, and `0x036DE520` | `STATIC PASS` subject to live positive/negative probe |
| Party confirmation | exact Endless-top controller, controller state `2`, active UI root, and live `Button01` entry; publish event hash `0xD78C07FC` | update prologue is unique at RVA `0x031C8C10`; vtable `0x05C76808` was independently recovered | `STATIC PASS` subject to live positive/negative probe |
| Difficulty confirmation | exact menu/update subobject vtables, active root, bounded count/current/selected indices, and dedicated zero-argument confirm function | update and confirm prologues are unique at RVAs `0x03CB4230` and `0x03CB4B80` | `STATIC PASS` subject to live positive/negative probe |

The associated unique signatures have lengths from 18 to 43 bytes. Every
wildcard covers an image-relative or branch displacement; fixed object-layout
operands remain part of the match. Production installation must still enforce
exactly one match and fail the optional capability without changing the core
meter status.

## Live Semantic Correction — 2026-07-25

An offline/private 2.0.2 session disproved the initial semantic inference from
the `EndlessResultReward` type name:

- ordinary battle reported zero live `MenuEndlessResultReward` instances;
- the visible Power/buff selection screen reported four stable primary menu
  instances and four corresponding instances for each secondary vtable;
- moving the visible selection by one horizontal choice did not change the
  bounded primary-menu fingerprints;
- no primary or secondary `ControllerEndlessResultReward` instance was live;
- the user confirmed that the visible choices were buffs/Powers, not the final
  treasure rewards described by the auto-reentry requirement.

This is a `MISMATCH`, not a final-reward positive observation. The Power screen
is now an in-scope required stage, but its four live menu objects do not map
one-to-one to the three visible choices. It remains blocked until the hidden
template is excluded and the grade, selected index, selection callback, and
confirmation callback are independently verified.

### Party-formation controller slot 24

RVA: `0x03CBE870`

```text
41 56 56 57 53 48 83 EC 28
8B 81 18 04 00 00 48 83 F8 03 0F 87 CE 03 00 00
48 89 CE
```

Observed static relationship: the function dispatches only when the value at
object offset `0x418` is in `0..=3`. The offset still requires independent
corroboration.

### Difficulty controller slot 26

RVA: `0x03194790`

```text
41 57 41 56 41 55 41 54 56 57 55 53
48 83 EC 48 49 89 CC
```

The longer prologue contains image-relative references and a relative call, so
an approved signature must mask those operands after the ABI is verified.

### Endless top controller slot 26

RVA: `0x031C8C50`

```text
55 41 57 41 56 41 55 41 54 56 57 53
48 81 EC F8 00 00 00 48 8D AC 24 80 00 00 00
```

The type identity is verified. Live observation disproved a separate
final-ready screen in the floor-five re-entry flow; this method is only a
provisional supporting-state candidate for the party/depth UI.

## Static Gate Result

The result/re-entry callback gate is met for an observation-only probe after
reference-assisted relocation. Action dispatch remains fail-closed until each
positive and hidden/stale negative is observed in the offline/private session.

The configured reward gate is not met:

- the reward row vector and selected/current indices are bounded, and the
  change/decide callbacks are uniquely relocated;
- the row's internal reward ID and selectable state are not yet independently
  mapped;
- the version-pinned Korean/English floor-five reward catalog is absent.

Accordingly, the optional observation detours and pure state machine may be
built, while configured reward selection and the public functional toggle stay
disabled until the catalog and live action gates pass.

## Minimal Autodrive Timer Investigation

The active product scope now leaves route, Power, monk, and ordinary OK
decisions to the game's unattended setting. Their candidate work above remains
TODO reference only.

The public text/config files in `GBFR-Fast-Conflux-v1.0.0.zip` document behavior
only: 2-second auto progress, 1-second notice, OFF on launch, and unchanged
fade/transition timing. The DLL was not extracted or decompiled.

Independent RTTI and access-site analysis resolved the timer boundary in the
pinned executable:

- `EndlessAbandonedAutoTimer` copies its instance flag to the global timer
  manager;
- `ControllerEndlessAutoPlayHud` compares the manager notice threshold with
  the current countdown;
- the manager contains the notice threshold, eleven per-mode defaults, Endless
  mode, initial duration, and current countdown.

A read-only live capture on an unattended-choice screen observed mode `1`,
notice `3.000`, initial `60.000`, current `13.818 -> 13.318` over 0.500 seconds,
and defaults `[60,60,30,30,30,30,30,30,60,30,30]`. The earlier ordinary-battle
object scan returned no countdown candidates because the authoritative timer
is manager-owned rather than controller-owned.

An explicitly opted-in offline/private test wrote only the manager timer data,
verified notice/default values `1/2`, clamped the active countdown to at most
two seconds, and restored the exact original configuration. No code, fade, or
transition field was modified. Visible about-two-second progression and OFF
restoration through the built page remain manual smoke-test items.

## TODO to Unblock

- [ ] Capture the relevant UI objects in an offline/private live session with a
      read-only object/vtable logger.
- [ ] Corroborate each portal's continuation/return destination, rare-Power
      modifier, game route-type enum, and generator slot index.
- [ ] Corroborate the final-boss return-gate phase/destination and its transition
      to the floor-five reward controller, excluding ordinary sole portals.
- [ ] Prove active portal-to-icon pairing, stale-icon exclusion, and the native
      interaction acknowledgement.
- [ ] Observe portal pairs containing exactly one and zero combat routes.
- [ ] Corroborate the Endless area-result active/ready field and exact OK
      callback, excluding the stable hidden boss-result object.
- [ ] Corroborate active boss-result state and exact OK callback, excluding the
      fixed four allocated Power menu objects and hidden boss-result state.
- [ ] Corroborate the monk shop active state, affordability, five-choice list,
      repeated-purchase transition, acquisition OK, automatic insufficient-CP
      exit, and root-menu close before portal selection.
- [ ] Identify and distinguish every monk-loop callback, including adjacent
      cancel paths that automation must not invoke during automatic exit.
- [ ] Distinguish the three visible Power choices from the fourth hidden or
      template `MenuEndlessResultReward` object.
- [ ] Corroborate the Power grade and selected-index fields and prove the
      maximum-grade, leftmost-tie rule against live UI.
- [ ] Corroborate the Chaos type field and prove that its presence selects
      display index zero without consulting grades.
- [ ] Map the floor-five reward row vector, selectable state, internal reward
      IDs, selected/current indices, selection callback, and confirmation
      callback.
- [ ] Prove configured-ID selection, first-selectable fallback, and
      acquisition-count independence against live reward screens.
- [ ] Record positive and negative vtable/object identities for every boundary.
- [ ] Trace the independently observed callbacks back to complete function
      starts and verify ABI through all callers.
- [ ] Corroborate every actionable field offset using a second access site or a
      constructor/update pair.
- [ ] Produce masked signatures and confirm exactly one `.text` match.
- [ ] Distinguish accept, cancel, unrelated dialog, and stale-object paths.
- [ ] For every initial blocking-dialog allowlist entry, prove the exact
      active/ready state, single-OK callback, dismissal acknowledgement, and
      permitted successor states; prove an unrelated single-OK dialog is not
      accepted.
- [ ] Re-run the static gate before creating Task 2's debug probe.

# Bilingual README User Guide Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add complete, matching Korean and English end-user instructions to `README.md` for operating Djeeta MOD, understanding the sigil cap display, and resolving common startup problems.

**Architecture:** This is a documentation-only product change. One Korean guide and one English guide are inserted after the existing behavior overview; existing performance, safety, artifact hash, source build, compatibility, and attribution sections remain authoritative and are not duplicated.

**Tech Stack:** GitHub-flavored Markdown, PowerShell verification, npm/Vitest/Vite, Rust/Cargo, Tauri/WiX MSI packaging.

## Global Constraints

- Target Granblue Fantasy: Relink Endless Ragnarok 2.0.2 on Windows x64.
- Do not claim game 2.0.2 compatibility before `docs/testing/game-2.0.2-smoke-test.md` is completed.
- Describe the current analysis scope as the primary and secondary traits of the 12 equipped sigils only.
- Explain that `hook.dll` is required, but do not document manual DLL injection.
- Keep Korean first and English second, with matching scope and meaning.
- Preserve existing build commands, SHA-256 records, license, and upstream credits.
- Do not touch or commit the runtime database `logs.db`.

---

### Task 1: Add the Korean and English user guides

**Files:**
- Modify: `README.md`

**Interfaces:**
- Consumes: Current sidebar labels from `src-tauri/lang/ko/ui.json` and `src-tauri/lang/en/ui.json`.
- Produces: Stable `## 사용자 가이드 (한국어)` and `## User Guide (English)` README anchors for end users.

- [ ] **Step 1: Verify the guide is absent before editing**

Run:

```powershell
$readme = Get-Content -LiteralPath README.md -Raw -Encoding utf8
if ($readme.Contains('## 사용자 가이드 (한국어)') -or $readme.Contains('## User Guide (English)')) {
  throw 'The bilingual guide already exists.'
}
```

Expected: command exits successfully with no output because neither guide exists.

- [ ] **Step 2: Insert the complete Korean guide after `## 표시와 동작`**

Replace the outdated installation step about resizing the meter with this current behavior:

```markdown
4. 관리 창의 왼쪽 메뉴에서 데미지 미터를 켜거나 끌 수 있고, 미터의 상단 제목 영역을 드래그하면 위치를 옮길 수 있습니다.
```

Add this Markdown immediately before `## 성능 영향`:

```markdown
## 사용자 가이드 (한국어)

### 시작하기

1. MSI를 설치한 뒤 Granblue Fantasy: Relink를 먼저 실행합니다.
2. Djeeta MOD를 실행합니다. 작업 표시줄에는 관리 창만 표시되고, 데미지 미터는 게임 위에 별도 창으로 표시됩니다.
3. 관리 창의 왼쪽 사이드바에서 기능을 선택합니다. 데스크톱에서는 사이드바가 기본으로 열려 있습니다.

### 왼쪽 메뉴

- **데미지 미터:** 스위치로 미터를 표시하거나 숨깁니다. 미터 상단의 `파티 데미지` 영역을 마우스로 드래그하면 위치를 옮길 수 있습니다. 미터는 항상 위에 표시되며 최대 4명의 누적 데미지와 DPS를 보여줍니다.
- **진 특성 상한 분석:** 게임에서 읽은 캐릭터별 장착 진 특성을 합산합니다. 현재는 장착 진 12개의 주·보조 특성만 포함합니다. 캐릭터 선택 상자에서 확인할 캐릭터를 바꿀 수 있습니다.
- **전투 기록:** 완료된 전투의 저장 기록을 열어 파티 데미지, DPS와 세부 기록을 확인합니다.
- **설정:** 언어, 미터 투명도, 표시 항목과 기타 미터 옵션을 변경합니다.

### 진 특성 상한 읽기

- `현재 / 최대`는 장착 진에서 합산한 현재 스킬 레벨과 확인된 상한을 뜻합니다.
- `정상`은 상한 미만, `최대`는 상한 도달, `N 초과`는 상한보다 N레벨 높다는 뜻입니다. 예를 들어 데미지 상한이 `70 / 65`이면 5레벨이 초과된 상태입니다.
- `최대치 미확인`은 해당 특성의 상한 데이터가 아직 검증되지 않았다는 뜻이며, 상한이 없다는 의미가 아닙니다.
- `기여 진`을 펼치면 합계에 포함된 각 진의 주·보조 특성 레벨을 확인할 수 있습니다.

### 문제가 있을 때

- 관리 창이 보이지 않으면 작업 표시줄 또는 시스템 트레이에서 Djeeta MOD를 확인하고, 계속 보이지 않으면 앱을 다시 실행합니다.
- 데미지 미터가 보이지 않으면 왼쪽 메뉴의 **데미지 미터** 스위치를 켭니다.
- `게임 연결 대기 중`이면 게임을 먼저 실행했는지 확인합니다.
- `게임 2.0.2 훅을 찾을 수 없습니다`가 표시되면 게임 버전이 2.0.2인지 확인하고, 백신이 `hook.dll`을 격리하지 않았는지 확인한 뒤 앱을 다시 설치합니다.
- 진 정보가 갱신되지 않으면 게임의 장비 편성 화면에서 확인할 캐릭터의 장비를 열거나 캐릭터를 다시 선택합니다.

`hook.dll`은 게임에서 전투 및 장착 진 정보를 읽어 Djeeta MOD로 전달하는 필수 구성요소입니다. 삭제하거나 격리하면 데미지 미터와 진 특성 분석이 동작하지 않습니다. 이 도구는 공식 허용 도구가 아니므로 먼저 오프라인 또는 비공개 환경에서 테스트하십시오.
```

- [ ] **Step 3: Insert the matching English guide after the Korean guide**

Add this Markdown immediately after the Korean guide and before `## 성능 영향`:

```markdown
## User Guide (English)

### Getting started

1. Install the MSI, then start Granblue Fantasy: Relink first.
2. Start Djeeta MOD. Only the management window appears on the taskbar; the damage meter is a separate overlay above the game.
3. Select a feature from the left sidebar in the management window. The sidebar is open by default on desktop.

### Left menu

- **Damage Meter:** Use the switch to show or hide the meter. Drag the `Party Damage` header to move it. The meter stays on top and shows cumulative damage and DPS for up to four players.
- **Sigil Trait Cap Analysis:** Totals the equipped sigil traits read from the game for each character. The current scope includes only primary and secondary traits from the 12 equipped sigils. Use the character selector to inspect another character.
- **Battle Records:** Opens saved encounters with party damage, DPS, and detailed records.
- **Settings:** Changes the language, meter transparency, displayed columns, and other meter options.

### Reading sigil trait caps

- `Current / Max` is the total current skill level from equipped sigils and the verified cap.
- `Normal` means below the cap, `Max` means the cap is reached, and `N over` means the total exceeds the cap by N levels. For example, Damage Cap at `70 / 65` is 5 levels over the cap.
- `Maximum unverified` means the cap data for that trait has not been verified yet; it does not mean the trait has no cap.
- Expand `Contributing sigils` to see the primary and secondary trait levels included in the total.

### Troubleshooting

- If the management window is missing, check the taskbar or system tray for Djeeta MOD. Restart the app if it still does not appear.
- If the damage meter is hidden, enable the **Damage Meter** switch in the left menu.
- If the app says `Waiting for the game`, confirm that the game was started first.
- If the app says `The game 2.0.2 hook was not found`, confirm that the game is version 2.0.2, check whether antivirus software quarantined `hook.dll`, and then reinstall the app.
- If sigil information does not update, open the equipment screen for that character in the game or select the character again.

`hook.dll` is required to read combat and equipped-sigil information from the game and deliver it to Djeeta MOD. Removing or quarantining it disables the damage meter and sigil trait analysis. This is not an officially approved tool, so test it in an offline or private session first.
```

- [ ] **Step 4: Verify structure, terminology, and Markdown**

Run:

```powershell
$readme = Get-Content -LiteralPath README.md -Raw -Encoding utf8
$required = @(
  '## 사용자 가이드 (한국어)',
  '## User Guide (English)',
  '**데미지 미터:**',
  '**진 특성 상한 분석:**',
  '**전투 기록:**',
  '**Damage Meter:**',
  '**Sigil Trait Cap Analysis:**',
  '**Battle Records:**',
  '70 / 65',
  'hook.dll'
)
foreach ($text in $required) {
  if (-not $readme.Contains($text)) { throw "Missing README text: $text" }
}
if ($readme.IndexOf('## 사용자 가이드 (한국어)') -gt $readme.IndexOf('## User Guide (English)')) {
  throw 'The Korean guide must appear before the English guide.'
}
git diff --check -- README.md
```

Expected: command exits successfully with no missing-text or ordering error, and `git diff --check` reports no whitespace errors.

- [ ] **Step 5: Commit the guide**

```powershell
git add -- README.md
git commit -m "docs: add bilingual user guide"
```

Expected: one commit containing only `README.md`.

### Task 2: Run the required verification and refresh package records

**Files:**
- Modify if the rebuilt hashes change: `README.md`
- Modify if the rebuilt hashes change: `docs/testing/game-2.0.2-smoke-test.md`
- Do not commit: `logs.db`

**Interfaces:**
- Consumes: The updated `README.md`, current frontend, Rust workspace, release hook, and Tauri MSI configuration.
- Produces: Verified frontend/Rust results, a fresh MSI, matching hook DLL hashes, and current documented artifact hashes.

- [ ] **Step 1: Run frontend dependency, formatting, lint, type, test, and build gates**

Run:

```powershell
npm ci
npm run format-check
npm run lint
npm run tsc
npm test -- --run
npm run build
```

Expected: every command exits 0; all Vitest files and tests pass. Existing dependency-audit and Vite chunk-size warnings may remain warnings only.

- [ ] **Step 2: Run Rust build and test gates**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" build --release --locked --package hook
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --workspace --all-targets --locked
```

Expected: both commands exit 0; existing dead-code warnings may remain warnings only.

- [ ] **Step 3: Synchronize the release hook and build the MSI**

First confirm `granblue_fantasy_relink.exe` is not running. Then run:

```powershell
Copy-Item -LiteralPath target\release\hook.dll -Destination src-tauri\hook.dll -Force
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
npm run tauri build -- --bundles msi
```

Expected: the copy succeeds and Tauri reports one MSI under `target/release/bundle/msi/`.

- [ ] **Step 4: Verify hashes and update both records if necessary**

Run:

```powershell
$msi = Get-ChildItem -LiteralPath target\release\bundle\msi -Filter *.msi |
  Sort-Object LastWriteTime -Descending |
  Select-Object -First 1
$releaseHook = (Get-FileHash -Algorithm SHA256 -LiteralPath target\release\hook.dll).Hash
$bundledHook = (Get-FileHash -Algorithm SHA256 -LiteralPath src-tauri\hook.dll).Hash
$msiHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $msi.FullName).Hash
if ($releaseHook -ne $bundledHook) { throw 'Release and bundled hook hashes differ.' }
[pscustomobject]@{ MSI = $msiHash; Hook = $releaseHook; Path = $msi.FullName }
```

Expected: hook hashes are equal and the command prints the current MSI path and SHA-256 values. If either value differs from the records, replace the MSI and `hook.dll` hashes in both `README.md` and `docs/testing/game-2.0.2-smoke-test.md` with the printed values.

- [ ] **Step 5: Check and commit only intentional final changes**

Run:

```powershell
git diff --check
git status --short
```

Expected: no whitespace errors. `logs.db` may appear only as an untracked runtime file. If artifact hash records changed, commit only the two documentation files:

```powershell
git add -- README.md docs/testing/game-2.0.2-smoke-test.md
git commit -m "docs: refresh package hashes"
```

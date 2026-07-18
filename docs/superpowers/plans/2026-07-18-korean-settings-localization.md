# Korean Settings Localization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 한국어 설정 화면에서 하드코딩 영어와 누락 번역을 제거하되 DPS, SBA, DMG, SPS 약어와 다른 언어 전환은 유지한다.

**Architecture:** `Settings.tsx`는 표시 문자열을 직접 소유하지 않고 기존 i18next `ui` 네임스페이스만 사용한다. 영어와 한국어 `ui.json`은 동일한 설정 키 계약을 제공하며, 작은 Vitest 계약 테스트가 한국어 값과 약어 보존 및 하드코딩 제거를 고정한다.

**Tech Stack:** React 18, TypeScript, react-i18next, Vitest, Tauri 1

## Global Constraints

- 변경 범위는 설정 화면 문구와 해당 번역 키에 한정한다.
- `DPS`, `SBA`, `DMG`, `SPS` 약어는 그대로 유지한다.
- 내부 enum, 저장 키, IPC 프로토콜과 설정 동작은 변경하지 않는다.
- 한국어를 직접 JSX에 하드코딩하지 않고 영어·한국어 번역 파일에 같은 키를 제공한다.
- 제품명은 `Djeeta MOD`, 패키지명은 `djeeta-mod`, 버전은 `0.1.0`을 유지한다.

---

### Task 1: 설정 화면 번역 계약과 UI 적용

**Files:**
- Create: `src/pages/Settings.localization.test.ts`
- Modify: `src/pages/Settings.tsx:77-190`
- Modify: `src-tauri/lang/en/ui.json`
- Modify: `src-tauri/lang/ko/ui.json`

**Interfaces:**
- Consumes: `useTranslation().t(key)`와 기존 `ui.meter-columns.*` 키 규칙
- Produces: `ui.color-placeholder`, `ui.customize-overlay-columns`, `ui.add-column`, `ui.remove-column` 및 설정 화면에서 사용하는 완전한 한국어 키 집합

- [ ] **Step 1: 실패하는 설정 번역 계약 테스트 작성**

```ts
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const readJson = (relativePath: string) =>
  JSON.parse(readFileSync(new URL(relativePath, import.meta.url), "utf8")) as {
    ui: Record<string, unknown>;
  };

describe("Korean settings localization", () => {
  const korean = readJson("../../src-tauri/lang/ko/ui.json").ui;
  const english = readJson("../../src-tauri/lang/en/ui.json").ui;

  it("provides the settings-only labels in both languages", () => {
    const expectedKorean = {
      "color-placeholder": "색상",
      "customize-overlay-columns": "오버레이 미터 열 설정",
      "add-column": "열 추가",
      "remove-column": "열 제거",
      "show-player-names": "플레이어 이름 표시",
      "streamer-mode": "스트리머 모드",
      "show-full-values": "전체 수치 표시",
      "use-condensed-skills": "축약 스킬명 사용",
      "open-log-on-save": "저장 후 로그 열기",
      "debug-mode": "디버그 모드",
    };

    for (const [key, value] of Object.entries(expectedKorean)) {
      expect(korean[key]).toBe(value);
      expect(english[key]).toEqual(expect.any(String));
    }
  });

  it("keeps the approved meter abbreviations", () => {
    const columns = korean["meter-columns"] as Record<string, string>;
    expect(columns).toMatchObject({ dps: "DPS", damage: "DMG", sba: "SBA", "stun-per-second": "SPS" });
  });

  it("does not hardcode the translated English labels in Settings.tsx", () => {
    const source = readFileSync(new URL("./Settings.tsx", import.meta.url), "utf8");
    for (const text of ["Customize Overlay Meter Columns", "Add column", "Remove column", 'placeholder="Color"']) {
      expect(source).not.toContain(text);
    }
  });
});
```

- [ ] **Step 2: 테스트가 현재 영어 하드코딩과 누락 키 때문에 실패하는지 확인**

Run: `npm test -- --run src/pages/Settings.localization.test.ts`

Expected: 새 한국어 키가 `undefined`이거나 `Settings.tsx`에 대상 영어가 남아 있어 FAIL.

- [ ] **Step 3: 영어와 한국어 번역 키 추가 및 기존 한국어 설정 키 보완**

`src-tauri/lang/en/ui.json`의 `ui` 객체에 다음 값을 추가한다.

```json
"color-placeholder": "Color",
"customize-overlay-columns": "Customize Overlay Meter Columns",
"add-column": "Add column",
"remove-column": "Remove column"
```

`src-tauri/lang/ko/ui.json`의 `ui` 객체에 다음 설정 값을 제공한다.

```json
"color-placeholder": "색상",
"customize-overlay-columns": "오버레이 미터 열 설정",
"add-column": "열 추가",
"remove-column": "열 제거",
"player-1-color": "바 색상 - 플레이어 1",
"player-2-color": "바 색상 - 플레이어 2",
"player-3-color": "바 색상 - 플레이어 3",
"player-4-color": "바 색상 - 플레이어 4",
"show-player-names": "플레이어 이름 표시",
"streamer-mode": "스트리머 모드",
"streamer-mode-description": "미터에 내 피해만 표시합니다.",
"show-full-values": "전체 수치 표시",
"show-full-values-description": "미터의 수치를 줄이지 않고 모두 표시합니다.",
"use-condensed-skills": "축약 스킬명 사용",
"use-condensed-skills-description": "여러 단계의 같은 스킬을 하나의 항목으로 묶습니다.",
"open-log-on-save": "저장 후 로그 열기",
"open-log-on-save-description": "전투 기록을 저장한 뒤 해당 로그를 자동으로 엽니다.",
"debug-mode": "디버그 모드",
"debug-mode-description": "원시 이벤트 데이터를 확인할 수 있도록 개발자 콘솔을 엽니다.",
"meter-columns": {
  "name": "이름",
  "dps": "DPS",
  "dps-description": "초당 데미지",
  "damage": "DMG",
  "damage-description": "누적 데미지",
  "damage-percentage": "%",
  "damage-percentage-description": "전체 데미지 비율",
  "sba": "SBA",
  "sba-description": "오의 게이지",
  "total-stun-value": "스턴",
  "total-stun-value-description": "누적 스턴 수치",
  "stun-per-second": "SPS",
  "stun-per-second-description": "초당 스턴 수치"
}
```

- [ ] **Step 4: Settings.tsx 하드코딩을 번역 키로 교체**

네 개의 `ColorInput`에 아래 값을 사용한다.

```tsx
placeholder={t("ui.color-placeholder")}
```

열 설정 제목, 추가 버튼과 제거 버튼 접근성 문구를 아래처럼 교체한다.

```tsx
<Text size="sm">{t("ui.customize-overlay-columns")}</Text>
<Button>{t("ui.add-column")}</Button>
<ActionIcon
  aria-label={t("ui.remove-column")}
  variant="transparent"
  color="gray"
  onClick={() => removeOverlayColumn(item)}
>
```

- [ ] **Step 5: 집중 테스트와 프런트엔드 검사를 실행**

Run:

```powershell
npm test -- --run src/pages/Settings.localization.test.ts
npm run format-check
npm run lint
npm run tsc
npm test -- --run
```

Expected: 설정 번역 테스트 3개와 기존 프런트엔드 테스트가 모두 PASS하며 포맷·린트·타입 오류가 없음.

- [ ] **Step 6: 설정 한국어화 커밋**

```powershell
git add src/pages/Settings.localization.test.ts src/pages/Settings.tsx src-tauri/lang/en/ui.json src-tauri/lang/ko/ui.json
git commit -m "feat: localize settings in Korean"
```

---

### Task 2: 전체 검증과 Djeeta MOD 설치 파일 갱신

**Files:**
- Modify: `README.md`
- Modify: `docs/testing/game-2.0.2-smoke-test.md`
- Generated: `src-tauri/hook.dll`
- Generated: `target/release/hook.dll`
- Generated: `target/release/bundle/msi/Djeeta MOD_0.1.0_x64_en-US.msi`

**Interfaces:**
- Consumes: Task 1의 번역된 프런트엔드와 현재 `hook.dll`
- Produces: 설정 한국어화가 포함된 MSI 및 실제 산출물과 일치하는 SHA-256 기록

- [ ] **Step 1: 잠금 파일 기반 전체 검증 및 빌드 실행**

```powershell
npm ci
npm run format-check
npm run lint
npm run tsc
npm test -- --run
npm run build
cargo build --release --locked --package hook
cargo test --workspace --all-targets --locked
Copy-Item -LiteralPath target/release/hook.dll -Destination src-tauri/hook.dll -Force
npm run tauri build -- --bundles msi
```

Expected: 모든 명령이 exit code 0이고 `Djeeta MOD_0.1.0_x64_en-US.msi`가 생성됨.

- [ ] **Step 2: DLL 동일성과 새 SHA-256 계산**

```powershell
$releaseHook = (Get-FileHash -Algorithm SHA256 -LiteralPath 'target/release/hook.dll').Hash
$packagedHook = (Get-FileHash -Algorithm SHA256 -LiteralPath 'src-tauri/hook.dll').Hash
if ($releaseHook -ne $packagedHook) { throw 'Hook DLL hashes differ.' }
Get-FileHash -Algorithm SHA256 -LiteralPath 'target/release/bundle/msi/Djeeta MOD_0.1.0_x64_en-US.msi'
Write-Output $releaseHook
```

Expected: 두 DLL 해시가 동일하며 MSI와 DLL의 새 SHA-256이 출력됨.

- [ ] **Step 3: 문서의 산출물 해시를 실제 값으로 교체**

`README.md`와 `docs/testing/game-2.0.2-smoke-test.md`의 MSI 및 hook SHA-256을 Step 2에서 출력된 값으로 정확히 교체한다. 수동 게임 2.0.2 호환성 상태는 계속 미검증으로 둔다.

- [ ] **Step 4: 최종 무결성 검사와 문서 커밋**

```powershell
git diff --check
git status --short
git add README.md docs/testing/game-2.0.2-smoke-test.md src-tauri/hook.dll
git commit -m "chore: package Korean settings update"
git status --short
```

Expected: 커밋 후 작업 트리가 비어 있음.

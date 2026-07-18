import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const readJson = (relativePath: string) =>
  JSON.parse(readFileSync(new URL(relativePath, import.meta.url), "utf8")) as {
    ui: Record<string, unknown>;
  };

const expectedEnglish = {
  language: "Language",
  "meter-settings": "Meter Settings",
  "color-placeholder": "Color",
  "customize-overlay-columns": "Customize Overlay Meter Columns",
  "add-column": "Add column",
  "remove-column": "Remove column",
  "player-1-color": "Bar Color - Player 1",
  "player-2-color": "Bar Color - Player 2",
  "player-3-color": "Bar Color - Player 3",
  "player-4-color": "Bar Color - Player 4",
  "meter-transparency": "Background Transparency",
  "show-player-names": "Show Player Names",
  "streamer-mode": "Streamer Mode",
  "streamer-mode-description": "Only shows your damage in the meter.",
  "show-full-values": "Show Full Values",
  "show-full-values-description": "Show full values in the meter.",
  "use-condensed-skills": "Use Condensed Skill Names",
  "use-condensed-skills-description":
    'Groups various skills into one entry (e.g. Attack 1, Attack 2 would condense into just "Attack")',
  "open-log-on-save": "Open Log on Save",
  "open-log-on-save-description": "Automatically open the log after saving an encounter.",
  "debug-mode": "Debug Mode",
  "debug-mode-description": "Opens the developer console to view all raw event data.",
};

const expectedKorean = {
  language: "언어",
  "meter-settings": "미터기 설정",
  "color-placeholder": "색상",
  "customize-overlay-columns": "오버레이 미터 열 설정",
  "add-column": "열 추가",
  "remove-column": "열 제거",
  "player-1-color": "바 색상 - 플레이어 1",
  "player-2-color": "바 색상 - 플레이어 2",
  "player-3-color": "바 색상 - 플레이어 3",
  "player-4-color": "바 색상 - 플레이어 4",
  "meter-transparency": "배경 투명도",
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
};

const expectedEnglishColumns = {
  name: "Name",
  dps: "DPS",
  "dps-description": "Damage Per Second",
  damage: "DMG",
  "damage-description": "Total Damage",
  "damage-percentage": "%",
  "damage-percentage-description": "Total Damage in Percentage",
  sba: "SBA",
  "sba-description": "Skybound Arts Gauge",
  "total-stun-value": "Stun",
  "total-stun-value-description": "Total Stun Value",
  "stun-per-second": "SPS",
  "stun-per-second-description": "Stun Per Second",
};

const expectedKoreanColumns = {
  name: "이름",
  dps: "DPS",
  "dps-description": "초당 데미지",
  damage: "DMG",
  "damage-description": "누적 데미지",
  "damage-percentage": "%",
  "damage-percentage-description": "전체 데미지 비율",
  sba: "SBA",
  "sba-description": "오의 게이지",
  "total-stun-value": "스턴",
  "total-stun-value-description": "누적 스턴 수치",
  "stun-per-second": "SPS",
  "stun-per-second-description": "초당 스턴 수치",
};

describe("Korean settings localization", () => {
  const korean = readJson("../../src-tauri/lang/ko/ui.json").ui;
  const english = readJson("../../src-tauri/lang/en/ui.json").ui;

  it("provides the complete approved settings labels in both languages", () => {
    expect(english).toMatchObject(expectedEnglish);
    expect(korean).toMatchObject(expectedKorean);
    expect(english["meter-columns"]).toEqual(expectedEnglishColumns);
    expect(korean["meter-columns"]).toEqual(expectedKoreanColumns);
  });

  it("uses translation keys instead of hardcoded settings labels", () => {
    const source = readFileSync(resolve(process.cwd(), "src/pages/Settings.tsx"), "utf8");

    for (const key of Object.keys(expectedKorean)) {
      expect(source).toContain(`t("ui.${key}")`);
    }
    expect(source).toContain("t(`ui.meter-columns.${item}`)");
    expect(source).toContain("t(`ui.meter-columns.${item}-description`)");
  });

  it("keeps the approved meter abbreviations in both languages", () => {
    const abbreviations = { dps: "DPS", damage: "DMG", sba: "SBA", "stun-per-second": "SPS" };
    expect(english["meter-columns"]).toMatchObject(abbreviations);
    expect(korean["meter-columns"]).toMatchObject(abbreviations);
  });
});

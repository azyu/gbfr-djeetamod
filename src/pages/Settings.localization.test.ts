import { readFileSync } from "node:fs";
import { resolve } from "node:path";
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
    const source = readFileSync(resolve(process.cwd(), "src/pages/Settings.tsx"), "utf8");
    for (const text of ["Customize Overlay Meter Columns", "Add column", "Remove column", 'placeholder="Color"']) {
      expect(source).not.toContain(text);
    }
  });
});

import { beforeEach, describe, expect, it, vi } from "vitest";

import enUi from "../src-tauri/lang/en/ui.json";
import koUi from "../src-tauri/lang/ko/ui.json";

const i18nextMocks = vi.hoisted(() => ({
  t: vi.fn(),
}));

vi.mock("i18next", () => ({
  t: i18nextMocks.t,
}));

import { getSkillTranslationKeys, toHash, toHashString, translateTraitId } from "./utils";

describe("utils", () => {
  beforeEach(() => {
    i18nextMocks.t.mockReset();
  });

  it("toHash", () => {
    expect(toHash(1)).toBe("1");
    expect(toHash(255)).toBe("ff");
  });

  it("toHashString", () => {
    expect(toHashString(1)).toBe("00000001");
    expect(toHashString(255)).toBe("000000ff");
  });

  it("falls back from a game 2 skill variant to its ability slot", () => {
    expect(getSkillTranslationKeys("Pl2800", 1110)).toEqual(["skills.Pl2800.1110", "skills.Pl2800.1100"]);
  });

  it("does not merge unknown or legacy character action IDs", () => {
    expect(getSkillTranslationKeys("Pl2300", 1510)).toEqual(["skills.Pl2300.1510"]);
    expect(getSkillTranslationKeys({ Unknown: 123 }, 1101)).toEqual([]);
  });

  it("passes an eight-digit trait ID to the unknown-trait fallback", () => {
    i18nextMocks.t.mockImplementation((keys, options) => {
      expect(keys).toEqual(["traits:0151cf9e.text", "ui.equipment-analysis.unknown-trait"]);
      expect(options).toEqual({ id: "0151cf9e" });
      return "알 수 없는 특성 (0x0151cf9e)";
    });

    expect(translateTraitId(0x0151cf9e)).toBe("알 수 없는 특성 (0x0151cf9e)");
  });

  it("defines ID-bearing unknown-trait fallbacks in both languages", () => {
    expect(koUi.ui["equipment-analysis"]["unknown-trait"]).toBe("알 수 없는 특성 (0x{{id}})");
    expect(enUi.ui["equipment-analysis"]["unknown-trait"]).toBe("Unknown trait (0x{{id}})");
  });
});

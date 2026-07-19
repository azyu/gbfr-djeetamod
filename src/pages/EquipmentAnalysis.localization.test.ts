import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, it } from "vitest";

const readScope = (language: "ko" | "en") => {
  const path = resolve(process.cwd(), `src-tauri/lang/${language}/ui.json`);
  const locale = JSON.parse(readFileSync(path, "utf8")) as {
    ui: { "equipment-analysis": { scope: string } };
  };

  return locale.ui["equipment-analysis"].scope;
};

it("explains the included and excluded equipment-analysis sources in both languages", () => {
  expect(readScope("ko")).toBe(
    "현재는 장착 진 12개의 주·보조 특성만 합산합니다. 무기·가호석·소환석·마스터 특성은 아직 포함되지 않습니다."
  );
  expect(readScope("en")).toBe(
    "Currently, only primary and secondary traits from the 12 equipped sigils are totaled. Weapon and wrightstone, summons, and master traits are not included yet."
  );
});

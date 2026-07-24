import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, it } from "vitest";

const REQUIRED_KEYS = [
  "title",
  "description",
  "refresh",
  "loading",
  "empty",
  "quantity",
  "tabs.inventory",
  "tabs.notifications",
  "notification.label",
  "notification.description",
  "notification.permission-denied",
  "error.ALREADY_RUNNING",
  "error.GAME_NOT_RUNNING",
  "error.UNSUPPORTED_GAME",
  "error.UNAVAILABLE",
  "error.UNSTABLE",
  "error.INTERNAL",
];

const get = (value: unknown, path: string): unknown =>
  path
    .split(".")
    .reduce<unknown>(
      (current, key) =>
        typeof current === "object" && current !== null ? (current as Record<string, unknown>)[key] : undefined,
      value
    );

it("provides matching Korean and English item-analysis copy", () => {
  for (const language of ["ko", "en"]) {
    const locale = JSON.parse(readFileSync(resolve(process.cwd(), `src-tauri/lang/${language}/ui.json`), "utf8")) as {
      ui: { "item-analysis"?: unknown };
    };
    for (const key of REQUIRED_KEYS) {
      expect(get(locale.ui["item-analysis"], key), `${language}:${key}`).toEqual(expect.any(String));
    }
  }
});

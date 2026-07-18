import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, it } from "vitest";

type WindowConfiguration = {
  label: string;
  title: string;
  alwaysOnTop?: boolean;
  skipTaskbar?: boolean;
};

it("keeps only the management window in the taskbar", () => {
  const path = resolve(process.cwd(), "src-tauri/tauri.conf.json");
  const config = JSON.parse(readFileSync(path, "utf8")) as {
    tauri: { windows: WindowConfiguration[] };
  };
  const meter = config.tauri.windows.find((window) => window.label === "main");
  const management = config.tauri.windows.find((window) => window.label === "logs");

  expect(meter?.alwaysOnTop).toBe(true);
  expect(meter?.skipTaskbar).toBe(true);
  expect(management?.title).toBe("Djeeta MOD");
  expect(management?.skipTaskbar ?? false).toBe(false);
});

it("reapplies the meter window policy after restored window state", () => {
  const path = resolve(process.cwd(), "src-tauri/src/main.rs");
  const source = readFileSync(path, "utf8");

  expect(source).toContain("window.set_skip_taskbar(true)?;");
  expect(source).toContain("window.set_always_on_top(true)?;");
});

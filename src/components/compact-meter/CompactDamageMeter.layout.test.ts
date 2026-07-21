import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, it } from "vitest";

it("fits four player rows inside the fixed-height meter", () => {
  const css = readFileSync(resolve(process.cwd(), "src/components/compact-meter/CompactDamageMeter.css"), "utf8");

  expect(css).toMatch(/\.compact-meter__header\s*\{[^}]*box-sizing:\s*border-box;/s);
  expect(css).toMatch(/\.compact-meter__row\s*\{[^}]*height:\s*28px;/s);
});

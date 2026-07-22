import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";

interface CargoTarget {
  kind: string[];
  name: string;
}

interface CargoPackage {
  name: string;
  targets: CargoTarget[];
}

interface CargoMetadata {
  packages: CargoPackage[];
}

it("keeps maintainer tools out of the application bundle targets", () => {
  const metadata = JSON.parse(
    execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1", "--locked", "--offline"], {
      cwd: process.cwd(),
      encoding: "utf8",
    })
  ) as CargoMetadata;
  const appPackage = metadata.packages.find(({ name }) => name === "gbfr-logs");

  expect(appPackage).toBeDefined();
  expect(appPackage?.targets.filter(({ kind }) => kind.includes("bin")).map(({ name }) => name)).toEqual(["gbfr-logs"]);
  expect(appPackage?.targets.find(({ name }) => name === "build_trait_caps")?.kind).toContain("example");
  expect(existsSync(join(process.cwd(), "src-tauri", "src", "bin", "build_trait_caps.rs"))).toBe(false);
});

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, it } from "vitest";

const readRepositoryFile = (path: string) => readFileSync(resolve(process.cwd(), path), "utf8");

it("runs the release application with the caller's privileges", () => {
  const manifest = readRepositoryFile("src-tauri/manifest.xml");

  expect(manifest).toContain('requestedExecutionLevel level="asInvoker"');
  expect(manifest).not.toContain("requireAdministrator");
});

it("keeps SmartScreen protection enabled in every WebView window", () => {
  const config = JSON.parse(readRepositoryFile("src-tauri/tauri.conf.json")) as {
    tauri: { windows: Array<{ additionalBrowserArgs?: string }> };
  };

  for (const window of config.tauri.windows) {
    expect(window.additionalBrowserArgs).toBe("--disable-features=msWebOOUI,msPdfOOUI --disable-gpu");
    expect(window.additionalBrowserArgs).not.toContain("msSmartScreenProtection");
  }
});

it("packages only a current-user NSIS installer", () => {
  const config = JSON.parse(readRepositoryFile("src-tauri/tauri.conf.json")) as {
    tauri: {
      bundle: {
        targets: string[];
        windows: { nsis?: { installMode?: string } };
      };
    };
  };

  expect(config.tauri.bundle.targets).toEqual(["nsis"]);
  expect(config.tauri.bundle.targets).not.toContain("msi");
  expect(config.tauri.bundle.windows.nsis?.installMode).toBe("currentUser");
});

it("exposes only the verified NSIS packaging command", () => {
  const packageJson = JSON.parse(readRepositoryFile("package.json")) as {
    scripts: Record<string, string>;
  };
  const packagingScript = readRepositoryFile("scripts/package.ps1");

  expect(packageJson.scripts["package:nsis"]).toBe(
    "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package.ps1",
  );
  expect(packageJson.scripts).not.toHaveProperty("package:msi");
  expect(packagingScript).toContain("'target\\release\\bundle\\nsis'");
  expect(packagingScript).toMatch(/'build',\s*'--bundles',\s*'nsis'/);
  expect(packagingScript).not.toMatch(/'build',\s*'--bundles',\s*'msi'/);
});

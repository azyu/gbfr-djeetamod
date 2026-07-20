import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, it, test } from "vitest";

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
    "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package.ps1"
  );
  expect(packageJson.scripts).not.toHaveProperty("package:msi");
  expect(packagingScript).toContain("'target\\release\\bundle\\nsis'");
  expect(packagingScript).toMatch(/'build',\s*'--bundles',\s*'nsis'/);
  expect(packagingScript).not.toMatch(/'build',\s*'--bundles',\s*'msi'/);
});

test("external equipment probe requests read-only process access", () => {
  const source = readRepositoryFile("src-tauri/src/equipment_probe/memory.rs");
  expect(source).toContain("PROCESS_VM_READ");
  expect(source).toContain("PROCESS_QUERY_INFORMATION");
  expect(source).not.toContain("PROCESS_QUERY_LIMITED_INFORMATION");
  expect(source).toContain("VirtualQueryEx");
  expect(source).toMatch(/if queried == 0 \{\s*return Err/);
  for (const forbidden of [
    "PROCESS_VM_WRITE",
    "PROCESS_VM_OPERATION",
    "PROCESS_CREATE_THREAD",
    "PROCESS_CREATE_PROCESS",
    "WriteProcessMemory",
    "VirtualAllocEx",
    "CreateRemoteThread",
  ]) {
    expect(source).not.toContain(forbidden);
  }
});

test("inventory probe stays read-only and release-gated", () => {
  const memory = readRepositoryFile("src-tauri/src/equipment_probe/memory.rs");
  const inventory = readRepositoryFile("src-tauri/src/equipment_probe/inventory.rs");
  expect(memory).toContain("PROCESS_QUERY_INFORMATION | PROCESS_VM_READ");
  expect(memory).toContain("VirtualQueryEx");
  expect(inventory).toContain('std::env::var("DJEETA_INVENTORY_PROBE")');
  expect(inventory).toContain("cfg!(debug_assertions)");
  for (const forbidden of [
    "PROCESS_VM_WRITE",
    "PROCESS_VM_OPERATION",
    "PROCESS_CREATE_THREAD",
    "PROCESS_CREATE_PROCESS",
    "WriteProcessMemory",
    "VirtualAllocEx",
    "CreateRemoteThread",
  ]) {
    expect(memory + inventory).not.toContain(forbidden);
  }
});

test("repeat quest writes are isolated from the read-only probes", () => {
  const repeatQuest = readRepositoryFile("src-tauri/src/repeat_quest.rs");
  const readOnlyProbes =
    readRepositoryFile("src-tauri/src/equipment_probe/memory.rs") +
    readRepositoryFile("src-tauri/src/equipment_probe/inventory.rs");

  for (const required of [
    "PROCESS_VM_WRITE",
    "PROCESS_VM_OPERATION",
    "WriteProcessMemory",
    "VirtualProtectEx",
    "FlushInstructionCache",
  ]) {
    expect(repeatQuest).toContain(required);
    expect(readOnlyProbes).not.toContain(required);
  }

  for (const forbidden of ["PROCESS_CREATE_THREAD", "VirtualAllocEx", "CreateRemoteThread"]) {
    expect(repeatQuest).not.toContain(forbidden);
  }
});

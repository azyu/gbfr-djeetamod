import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, it } from "vitest";

const readRepositoryFile = (path: string) => readFileSync(resolve(process.cwd(), path), "utf8");

it("delegates double-click release builds to the secure PowerShell wrapper", () => {
  const command = readRepositoryFile("build-release.cmd");

  expect(command).toContain(
    'powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\\build-release.ps1"'
  );
  expect(command).toContain('set "BUILD_EXIT_CODE=%ERRORLEVEL%"');
  expect(command).toContain("pause");
  expect(command).toContain("exit /b %BUILD_EXIT_CODE%");
});

it("keeps updater credentials process-scoped and always clears them", () => {
  const wrapper = readRepositoryFile("scripts/build-release.ps1");
  const clearPrivateKeyIndex = wrapper.indexOf("Remove-Item Env:TAURI_PRIVATE_KEY");
  const clearPasswordIndex = wrapper.indexOf("Remove-Item Env:TAURI_KEY_PASSWORD");
  const prepareIndex = wrapper.indexOf("& $npmPath run package:nsis");
  const setPrivateKeyIndex = wrapper.indexOf("$env:TAURI_PRIVATE_KEY =");
  const setPasswordIndex = wrapper.indexOf("$env:TAURI_KEY_PASSWORD =");
  const signIndex = wrapper.indexOf("& $npmPath run package:sign");

  expect(wrapper).toContain("[Environment]::GetFolderPath('UserProfile')");
  expect(wrapper).toContain("'.djeeta-mod\\updater.key'");
  expect(wrapper).toContain("Read-Host 'Updater key password' -AsSecureString");
  expect(wrapper).toContain("SecureStringToBSTR");
  expect(wrapper).toContain("PtrToStringBSTR");
  expect(wrapper).toContain("Get-Command npm.cmd");
  expect(clearPrivateKeyIndex).toBeGreaterThanOrEqual(0);
  expect(clearPasswordIndex).toBeGreaterThanOrEqual(0);
  expect(prepareIndex).toBeGreaterThan(clearPrivateKeyIndex);
  expect(prepareIndex).toBeGreaterThan(clearPasswordIndex);
  expect(setPrivateKeyIndex).toBeGreaterThan(prepareIndex);
  expect(setPasswordIndex).toBeGreaterThan(prepareIndex);
  expect(signIndex).toBeGreaterThan(setPrivateKeyIndex);
  expect(signIndex).toBeGreaterThan(setPasswordIndex);
  expect(wrapper).toContain("finally");
  expect(wrapper).toContain("ZeroFreeBSTR");
  expect(wrapper).not.toMatch(/--password|-p\s+['"]/);
});

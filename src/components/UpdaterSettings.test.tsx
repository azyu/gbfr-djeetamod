import { MantineProvider } from "@mantine/core";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import type { UpdaterState } from "@/pages/useUpdater";

const mocks = vi.hoisted(() => ({
  checkForUpdate: vi.fn(),
  dismissUpdate: vi.fn(),
  installAvailableUpdate: vi.fn(),
  state: {
    phase: "idle",
    currentVersion: "0.1.1",
    manifest: null,
    error: null,
  } as UpdaterState,
}));

vi.mock("@/pages/useUpdater", () => ({
  useUpdater: () => ({
    state: mocks.state,
    checkForUpdate: mocks.checkForUpdate,
    dismissUpdate: mocks.dismissUpdate,
    installAvailableUpdate: mocks.installAvailableUpdate,
  }),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, values?: { version?: string }) =>
      ({
        "ui.updater.title": "업데이트",
        "ui.updater.current": `현재 버전 v${values?.version}`,
        "ui.updater.check": "업데이트 확인",
        "ui.updater.checking": "업데이트를 확인하는 중입니다.",
        "ui.updater.preparing": "업데이트를 준비하는 중입니다.",
        "ui.updater.installing": "업데이트를 설치하는 중입니다.",
        "ui.updater.up-to-date": "최신 버전을 사용 중입니다.",
        "ui.updater.available": `새 버전 v${values?.version}을 사용할 수 있습니다.`,
        "ui.updater.check-failed": "업데이트를 확인하지 못했습니다.",
      })[key] ?? key,
  }),
}));

import { UpdaterSettings } from "./UpdaterSettings";

beforeEach(() => {
  vi.clearAllMocks();
  mocks.state = {
    phase: "idle",
    currentVersion: "0.1.1",
    manifest: null,
    error: null,
  };
  window.matchMedia = vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  }));
});

afterEach(cleanup);

const renderSettings = () =>
  render(
    <MantineProvider>
      <UpdaterSettings />
    </MantineProvider>
  );

it("shows the current version and starts a manual check", () => {
  renderSettings();

  expect(screen.getByText("현재 버전 v0.1.1")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "업데이트 확인" }));
  expect(mocks.checkForUpdate).toHaveBeenCalledTimes(1);
  expect(mocks.checkForUpdate).toHaveBeenCalledWith("manual");
});

it.each([
  ["checking", "업데이트를 확인하는 중입니다."],
  ["preparing", "업데이트를 준비하는 중입니다."],
  ["installing", "업데이트를 설치하는 중입니다."],
] as const)("disables the check button while phase is %s", (phase, message) => {
  mocks.state = { ...mocks.state, phase };
  renderSettings();

  expect(screen.getByText(message)).toBeTruthy();
  expect((screen.getByRole("button", { name: "업데이트 확인" }) as HTMLButtonElement).disabled).toBe(true);
});

it("shows available, up-to-date, and manual error status", () => {
  const { rerender } = renderSettings();

  mocks.state = { ...mocks.state, phase: "upToDate" };
  rerender(
    <MantineProvider>
      <UpdaterSettings />
    </MantineProvider>
  );
  expect(screen.getByText("최신 버전을 사용 중입니다.")).toBeTruthy();

  mocks.state = {
    ...mocks.state,
    phase: "available",
    manifest: { version: "0.1.2", date: "2026-07-22T00:00:00Z", body: "Signed update" },
  };
  rerender(
    <MantineProvider>
      <UpdaterSettings />
    </MantineProvider>
  );
  expect(screen.getByText("새 버전 v0.1.2을 사용할 수 있습니다.")).toBeTruthy();

  mocks.state = { ...mocks.state, phase: "error", error: "checkFailed" };
  rerender(
    <MantineProvider>
      <UpdaterSettings />
    </MantineProvider>
  );
  expect(screen.getByText("업데이트를 확인하지 못했습니다.")).toBeTruthy();
});

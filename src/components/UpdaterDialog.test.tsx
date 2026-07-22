import { MantineProvider } from "@mantine/core";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import type { UpdaterState } from "@/pages/useUpdater";

const mocks = vi.hoisted(() => ({
  checkForUpdate: vi.fn(),
  dismissUpdate: vi.fn(),
  installAvailableUpdate: vi.fn(),
  state: {
    phase: "available",
    currentVersion: "0.1.1",
    manifest: { version: "0.1.2", date: "2026-07-22T00:00:00Z", body: "Signed update" },
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
        "ui.updater.available": `새 버전 v${values?.version}을 사용할 수 있습니다.`,
        "ui.updater.notes": "릴리스 노트",
        "ui.updater.later": "나중에",
        "ui.updater.install": "업데이트",
        "ui.updater.preparing": "업데이트를 준비하는 중입니다.",
        "ui.updater.installing": "업데이트를 설치하는 중입니다.",
        "ui.updater.game-running": "게임을 종료한 후 다시 업데이트해 주세요.",
        "ui.updater.repeat-quest-restore-failed": "무한 퀘스트 반복 설정을 복구하지 못해 업데이트를 중단했습니다.",
        "ui.updater.install-failed": "업데이트를 설치하지 못했습니다.",
      })[key] ?? key,
  }),
}));

import { UpdaterDialog } from "./UpdaterDialog";

beforeEach(() => {
  vi.clearAllMocks();
  mocks.state = {
    phase: "available",
    currentVersion: "0.1.1",
    manifest: { version: "0.1.2", date: "2026-07-22T00:00:00Z", body: "Signed update" },
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

const renderDialog = () =>
  render(
    <MantineProvider>
      <UpdaterDialog />
    </MantineProvider>
  );

it("shows the available version and non-empty release notes", () => {
  renderDialog();

  expect(screen.getByText("새 버전 v0.1.2을 사용할 수 있습니다.")).toBeTruthy();
  expect(screen.getByText("릴리스 노트")).toBeTruthy();
  expect(screen.getByText("Signed update")).toBeTruthy();
});

it("dismisses the update when Later is selected", () => {
  renderDialog();

  fireEvent.click(screen.getByRole("button", { name: "나중에" }));
  expect(mocks.dismissUpdate).toHaveBeenCalledTimes(1);
});

it("starts installation only when Update is selected", () => {
  renderDialog();

  expect(mocks.installAvailableUpdate).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "업데이트" }));
  expect(mocks.installAvailableUpdate).toHaveBeenCalledTimes(1);
});

it("omits the release notes section when body is empty", () => {
  mocks.state = {
    ...mocks.state,
    manifest: { ...mocks.state.manifest!, body: "" },
  };
  renderDialog();

  expect(screen.queryByText("릴리스 노트")).toBeNull();
});

it("does not open an installation modal for checkFailed", () => {
  mocks.state = { ...mocks.state, phase: "error", error: "checkFailed" };
  renderDialog();

  expect(screen.queryByRole("dialog")).toBeNull();
});

it.each([
  ["preparing", null, "업데이트를 준비하는 중입니다."],
  ["installing", null, "업데이트를 설치하는 중입니다."],
  ["error", "gameRunning", "게임을 종료한 후 다시 업데이트해 주세요."],
  ["error", "repeatQuestRestoreFailed", "무한 퀘스트 반복 설정을 복구하지 못해 업데이트를 중단했습니다."],
  ["error", "installFailed", "업데이트를 설치하지 못했습니다."],
] as const)("shows %s state with %s error", (phase, error, message) => {
  mocks.state = { ...mocks.state, phase, error };
  renderDialog();

  expect(screen.getByText(message)).toBeTruthy();
});

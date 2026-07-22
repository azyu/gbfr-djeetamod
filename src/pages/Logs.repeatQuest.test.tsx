import { MantineProvider } from "@mantine/core";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import Layout from "./Logs";

const mocks = vi.hoisted(() => ({
  setRepeatEnabled: vi.fn(),
  repeatStatus: { state: "off", reason: null } as {
    state: "unavailable" | "off" | "on";
    reason: string | null;
  },
}));

vi.mock("@/hooks/getVersion", () => ({
  default: () => ({ version: "9.8.7" }),
}));

vi.mock("./useMeterVisibility", () => ({
  default: () => ({ meterEnabled: true, setMeterEnabled: vi.fn() }),
}));

vi.mock("./useRepeatQuest", () => ({
  default: () => ({
    status: mocks.repeatStatus,
    pending: false,
    setEnabled: mocks.setRepeatEnabled,
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => vi.fn()),
}));

vi.mock("@tauri-apps/api/app", () => ({
  getVersion: vi.fn(async () => "9.8.7"),
}));

vi.mock("@tauri-apps/api/updater", () => ({
  checkUpdate: vi.fn(async () => ({ shouldUpdate: false })),
  installUpdate: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) =>
      ({
        "ui.navigation.damage-meter": "데미지 미터",
        "ui.navigation.battle-records": "전투 기록",
        "ui.navigation.settings": "설정",
        "ui.equipment-analysis.title": "진 특성 상한 분석",
        "ui.game-features.repeat-quest.label": "무한 퀘스트 반복",
        "ui.game-features.repeat-quest.reason.gameNotRunning": "게임이 실행 중이 아닙니다.",
        "ui.game-features.repeat-quest.reason.accessDenied": "현재 권한으로 게임 코드를 변경할 수 없습니다.",
      })[key] ?? key,
  }),
}));

beforeEach(() => {
  mocks.repeatStatus = { state: "off", reason: null };
  mocks.setRepeatEnabled.mockReset();
  mocks.setRepeatEnabled.mockResolvedValue(undefined);
  globalThis.ResizeObserver = class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
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

function renderLayout() {
  return render(
    <MantineProvider>
      <MemoryRouter initialEntries={["/logs"]}>
        <Routes>
          <Route path="/logs" element={<Layout />}>
            <Route index element={<div>content</div>} />
          </Route>
        </Routes>
      </MemoryRouter>
    </MantineProvider>
  );
}

it("shows repeat quest immediately after the damage-meter control", () => {
  renderLayout();

  const meter = screen.getByRole("switch", { name: "데미지 미터" });
  const repeat = screen.getByRole("switch", { name: "무한 퀘스트 반복" });
  expect(meter.compareDocumentPosition(repeat) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
});

it("changes repeat quest only through its switch", () => {
  renderLayout();

  fireEvent.click(screen.getByRole("switch", { name: "무한 퀘스트 반복" }));

  expect(mocks.setRepeatEnabled).toHaveBeenCalledTimes(1);
  expect(mocks.setRepeatEnabled).toHaveBeenCalledWith(true);
});

it("leaves the common game-not-running state to the header", () => {
  mocks.repeatStatus = { state: "unavailable", reason: "gameNotRunning" };
  renderLayout();

  expect(screen.queryByText("게임이 실행 중이 아닙니다.")).toBeNull();
  expect((screen.getByRole("switch", { name: "무한 퀘스트 반복" }) as HTMLInputElement).disabled).toBe(true);
});

it("keeps a repeat-quest-specific failure below the switch", () => {
  mocks.repeatStatus = { state: "unavailable", reason: "accessDenied" };
  renderLayout();

  expect(screen.getByText("현재 권한으로 게임 코드를 변경할 수 없습니다.")).toBeTruthy();
});

it("keeps SettingsPage free of a duplicate repeat-quest control", () => {
  const settings = readFileSync(resolve(process.cwd(), "src/pages/Settings.tsx"), "utf8");
  const layout = readFileSync(resolve(process.cwd(), "src/pages/Logs.tsx"), "utf8");

  expect(settings).not.toContain("useRepeatQuest");
  expect(settings).not.toContain("ui.game-features.repeat-quest.label");
  expect(layout).toContain("ui.game-features.repeat-quest.label");
});

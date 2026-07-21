import { MantineProvider } from "@mantine/core";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import SettingsPage from "./Settings";

type Status = {
  state: "unavailable" | "off" | "on";
  reason: string | null;
};

const mocks = vi.hoisted(() => ({
  connectionState: "connected",
  status: { state: "off", reason: null } as Status,
  setResult: { state: "on", reason: null } as Status,
  setPromise: null as Promise<Status> | null,
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api", () => ({
  invoke: mocks.invoke,
}));

vi.mock("./useSettings", () => ({
  default: () => ({
    connectionState: mocks.connectionState,
    color_1: "#111111",
    color_2: "#222222",
    color_3: "#333333",
    color_4: "#444444",
    transparency: 0.5,
    show_display_names: true,
    streamer_mode: false,
    show_full_values: false,
    use_condensed_skills: false,
    open_log_on_save: false,
    setMeterSettings: vi.fn(),
    languages: [{ value: "ko", label: "한국어" }],
    handleLanguageChange: vi.fn(),
    overlay_columns: [],
    handleReorderOverlayColumns: vi.fn(),
    availableOverlayColumns: [],
    addOverlayColumn: vi.fn(),
    removeOverlayColumn: vi.fn(),
  }),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    i18n: { language: "ko" },
    t: (key: string) =>
      ({
        "ui.game-features.title": "게임 기능",
        "ui.game-features.repeat-quest.label": "무한 퀘스트 반복",
        "ui.game-features.repeat-quest.description": "퀘스트 반복 횟수 제한을 해제합니다.",
        "ui.game-features.repeat-quest.reason.gameNotRunning": "게임이 실행 중이 아닙니다.",
        "ui.game-features.repeat-quest.reason.unsupportedGame": "지원하는 게임 2.0.2 실행 파일이 아닙니다.",
        "ui.game-features.repeat-quest.reason.accessDenied": "현재 권한으로 게임 코드를 변경할 수 없습니다.",
      })[key] ?? key,
  }),
}));

beforeEach(() => {
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
  mocks.connectionState = "connected";
  mocks.status = { state: "off", reason: null };
  mocks.setResult = { state: "on", reason: null };
  mocks.setPromise = null;
  mocks.invoke.mockReset();
  mocks.invoke.mockImplementation((command: string) => {
    if (command === "get_repeat_quest_status") return Promise.resolve(mocks.status);
    if (command === "set_repeat_quest_enabled") return mocks.setPromise ?? Promise.resolve(mocks.setResult);
    return Promise.resolve(undefined);
  });
});

afterEach(cleanup);

function renderSettings() {
  return render(
    <MantineProvider>
      <SettingsPage />
    </MantineProvider>
  );
}

it("shows the release-visible repeat quest switch from backend state", async () => {
  renderSettings();

  const toggle = await screen.findByRole("switch", { name: "무한 퀘스트 반복" });
  expect((toggle as HTMLInputElement).checked).toBe(false);
  expect((toggle as HTMLInputElement).disabled).toBe(false);
});

it("locks while enabling and uses the backend observed result", async () => {
  let resolveSet!: (status: Status) => void;
  mocks.setPromise = new Promise((resolve) => {
    resolveSet = resolve;
  });
  renderSettings();
  const toggle = await screen.findByRole("switch", { name: "무한 퀘스트 반복" });

  fireEvent.click(toggle);
  expect((toggle as HTMLInputElement).disabled).toBe(true);
  expect(mocks.invoke).toHaveBeenCalledWith("set_repeat_quest_enabled", { enabled: true });

  await act(async () => resolveSet({ state: "on", reason: null }));
  expect((toggle as HTMLInputElement).checked).toBe(true);
});

it("keeps the observed state and shows the reason after a failed toggle", async () => {
  mocks.setResult = { state: "off", reason: "accessDenied" };
  renderSettings();

  fireEvent.click(await screen.findByRole("switch", { name: "무한 퀘스트 반복" }));

  expect(await screen.findByText("현재 권한으로 게임 코드를 변경할 수 없습니다.")).toBeTruthy();
  expect((screen.getByRole("switch", { name: "무한 퀘스트 반복" }) as HTMLInputElement).checked).toBe(false);
});

it.each([
  ["gameNotRunning", "게임이 실행 중이 아닙니다."],
  ["unsupportedGame", "지원하는 게임 2.0.2 실행 파일이 아닙니다."],
])("disables unavailable state with the %s reason", async (reason, message) => {
  mocks.status = { state: "unavailable", reason };
  renderSettings();

  const toggle = await screen.findByRole("switch", { name: "무한 퀘스트 반복" });
  expect((toggle as HTMLInputElement).disabled).toBe(true);
  expect(screen.getByText(message)).toBeTruthy();
});

it("refreshes status when the connection state changes", async () => {
  const view = renderSettings();
  await screen.findByRole("switch", { name: "무한 퀘스트 반복" });
  expect(mocks.invoke).toHaveBeenCalledTimes(1);

  mocks.connectionState = "disconnected";
  view.rerender(
    <MantineProvider>
      <SettingsPage />
    </MantineProvider>
  );

  await act(async () => undefined);
  expect(mocks.invoke).toHaveBeenCalledTimes(2);
  expect(mocks.invoke).toHaveBeenLastCalledWith("get_repeat_quest_status");
});

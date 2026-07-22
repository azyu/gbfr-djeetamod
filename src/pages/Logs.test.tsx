import { MantineProvider } from "@mantine/core";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { ConnectionState } from "@/types";
import Layout from "./Logs";

const mocks = vi.hoisted(() => ({
  connectionState: "searching" as ConnectionState,
  checkUpdate: vi.fn(),
  meterEnabled: true,
  setMeterEnabled: vi.fn(),
  invoke: vi.fn(),
  version: "9.8.7",
}));

vi.mock("@/hooks/getVersion", () => ({
  default: () => ({ version: mocks.version }),
}));

vi.mock("./useConnectionState", () => ({
  default: () => mocks.connectionState,
}));

vi.mock("./useMeterVisibility", () => ({
  default: () => ({
    meterEnabled: mocks.meterEnabled,
    setMeterEnabled: mocks.setMeterEnabled,
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => vi.fn()),
}));

vi.mock("@tauri-apps/api/app", () => ({
  getVersion: vi.fn(async () => "9.8.7"),
}));

vi.mock("@tauri-apps/api/updater", () => ({
  checkUpdate: mocks.checkUpdate,
  installUpdate: vi.fn(),
}));

vi.mock("@tauri-apps/api/tauri", () => ({
  invoke: mocks.invoke,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) =>
      ({
        "ui.navigation.damage-meter": "데미지 미터",
        "ui.navigation.battle-records": "전투 기록",
        "ui.navigation.settings": "설정",
        "ui.equipment-analysis.title": "진 특성 상한 분석",
        "ui.connection.searching": "게임을 찾는 중입니다",
        "ui.connection.connected": "게임에 연결되었습니다",
        "ui.connection.disconnected": "게임 실행 중이 아닙니다",
        "ui.connection.unsupported": "지원하지 않는 게임 버전입니다",
        "ui.connection.not-found": "게임을 찾지 못했습니다.",
        "ui.game-search.retry": "재시도",
        "ui.game-search.retry-label": "게임 다시 찾기",
      })[key] ?? key,
  }),
}));

beforeEach(() => {
  mocks.connectionState = "searching";
  mocks.meterEnabled = true;
  mocks.checkUpdate.mockReset();
  mocks.checkUpdate.mockResolvedValue({ shouldUpdate: false });
  mocks.setMeterEnabled.mockReset();
  mocks.setMeterEnabled.mockResolvedValue(undefined);
  mocks.invoke.mockReset();
  mocks.invoke.mockResolvedValue(true);
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

it("shows the management navigation with an enabled meter switch", () => {
  renderLayout();

  expect(screen.getByText("데미지 미터")).toBeTruthy();
  expect(screen.getByText("진 특성 상한 분석")).toBeTruthy();
  expect(screen.getByText("전투 기록")).toBeTruthy();
  expect(screen.getByText("설정")).toBeTruthy();
  expect((screen.getByRole("switch", { name: "데미지 미터" }) as HTMLInputElement).checked).toBe(true);
});

it("starts one non-blocking update check for the management window", async () => {
  renderLayout();

  await waitFor(() => expect(mocks.checkUpdate).toHaveBeenCalledTimes(1));
});

it("toggles once from either the row or the switch", () => {
  renderLayout();

  fireEvent.click(screen.getByText("데미지 미터"));
  expect(mocks.setMeterEnabled).toHaveBeenCalledTimes(1);
  expect(mocks.setMeterEnabled).toHaveBeenLastCalledWith(false);

  mocks.setMeterEnabled.mockClear();
  fireEvent.click(screen.getByRole("switch", { name: "데미지 미터" }));
  expect(mocks.setMeterEnabled).toHaveBeenCalledTimes(1);
  expect(mocks.setMeterEnabled).toHaveBeenLastCalledWith(false);
});

it("starts with mobile navigation closed and desktop navigation open", () => {
  const source = readFileSync(resolve(process.cwd(), "src/pages/Logs.tsx"), "utf8");

  expect(source).toMatch(/mobileOpened[\s\S]*useDisclosure\(\)/);
  expect(source).toMatch(/desktopOpened[\s\S]*useDisclosure\(true\)/);
});

it.each([
  ["searching", "게임을 찾는 중입니다"],
  ["connected", "게임에 연결되었습니다"],
  ["disconnected", "게임 실행 중이 아닙니다"],
  ["unsupported", "지원하지 않는 게임 버전입니다"],
  ["not-found", "게임을 찾지 못했습니다."],
] as const)("shows the %s game state in the management header", (state, label) => {
  mocks.connectionState = state;
  renderLayout();

  const header = screen.getByRole("banner");
  expect(within(header).getByText("Djeeta MOD (v9.8.7)")).toBeTruthy();
  expect(within(header).getByText(label)).toBeTruthy();
});

it("shows retry only after the game search is exhausted", () => {
  mocks.connectionState = "not-found";
  const { rerender } = renderLayout();

  expect(screen.getByRole("button", { name: "게임 다시 찾기" })).toBeTruthy();

  mocks.connectionState = "searching";
  rerender(
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
  expect(screen.queryByRole("button", { name: "게임 다시 찾기" })).toBeNull();
});

it("starts only one retry while the command is pending", async () => {
  let finishRetry: ((value: boolean) => void) | undefined;
  mocks.invoke.mockReturnValue(
    new Promise<boolean>((resolve) => {
      finishRetry = resolve;
    })
  );
  mocks.connectionState = "not-found";
  renderLayout();

  const retry = screen.getByRole("button", { name: "게임 다시 찾기" });
  fireEvent.click(retry);
  fireEvent.click(retry);

  expect(mocks.invoke).toHaveBeenCalledTimes(1);
  expect(mocks.invoke).toHaveBeenCalledWith("retry_game_search");
  expect((retry as HTMLButtonElement).disabled).toBe(true);

  finishRetry?.(true);
  await waitFor(() => expect((retry as HTMLButtonElement).disabled).toBe(false));
});

it("keeps the title and status at opposite sides of the header", () => {
  const source = readFileSync(resolve(process.cwd(), "src/pages/Logs.tsx"), "utf8");

  expect(source).toContain('<Group h="100%" px="sm" justify="space-between" wrap="nowrap">');
});

it("keeps settings below the scrollable navigation section", () => {
  const source = readFileSync(resolve(process.cwd(), "src/pages/Logs.tsx"), "utf8");

  expect(source).toContain("<AppShell.Section grow component={ScrollArea}>");
  expect(source.indexOf('to="/logs/settings"')).toBeGreaterThan(source.indexOf("</AppShell.Section>"));
});

it("gives management content its own vertical scrollbar", () => {
  const source = readFileSync(resolve(process.cwd(), "src/pages/Logs.tsx"), "utf8");
  const css = readFileSync(resolve(process.cwd(), "src/pages/Logs.css"), "utf8");

  expect(source).toContain('<AppShell.Main className="log-main">');
  expect(css).toMatch(/\.log-window\s*\{[^}]*height:\s*100vh;[^}]*overflow:\s*hidden;/s);
  expect(css).toMatch(/\.log-main\s*\{[^}]*height:\s*100vh;[^}]*overflow-y:\s*auto;/s);
});

import { MantineProvider } from "@mantine/core";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { ConnectionState } from "@/types";
import Layout from "./Logs";

const mocks = vi.hoisted(() => ({
  connectionState: "searching" as ConnectionState,
  meterEnabled: true,
  setMeterEnabled: vi.fn(),
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
      })[key] ?? key,
  }),
}));

beforeEach(() => {
  mocks.connectionState = "searching";
  mocks.meterEnabled = true;
  mocks.setMeterEnabled.mockReset();
  mocks.setMeterEnabled.mockResolvedValue(undefined);
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
] as const)("shows the %s game state in the management header", (state, label) => {
  mocks.connectionState = state;
  renderLayout();

  const header = screen.getByRole("banner");
  expect(within(header).getByText("Djeeta MOD")).toBeTruthy();
  expect(within(header).getByText(label)).toBeTruthy();
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

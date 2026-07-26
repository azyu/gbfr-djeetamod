import { MantineProvider } from "@mantine/core";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { Conflux } from "./Conflux";

const mocks = vi.hoisted(() => ({
  setEnabled: vi.fn(),
  pending: false,
  status: { state: "off", reason: null } as {
    state: "unavailable" | "off" | "on";
    reason: string | null;
  } | null,
}));

vi.mock("./useConfluxTimer", () => ({
  default: () => ({
    status: mocks.status,
    pending: mocks.pending,
    setEnabled: mocks.setEnabled,
  }),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) =>
      ({
        "ui.game-features.conflux.title": "극돈공소",
        "ui.game-features.conflux.description": "게임 자동 진행을 보조합니다.",
        "ui.game-features.conflux.requirement-title": "게임 설정 필요",
        "ui.game-features.conflux.requirement": "극돈공소 무조작 시 설정을 ON으로 켜세요.",
        "ui.game-features.conflux.auto-run": "자동 실행",
        "ui.game-features.conflux.timer-description": "자동 선택 대기 시간을 2초로 줄입니다.",
        "ui.game-features.conflux.todo": "보상 선택과 재진입은 후속 작업입니다.",
        "ui.game-features.conflux.reason.accessDenied": "게임 메모리에 접근할 수 없습니다.",
      })[key] ?? key,
  }),
}));

beforeEach(() => {
  mocks.status = { state: "off", reason: null };
  mocks.pending = false;
  mocks.setEnabled.mockReset();
  mocks.setEnabled.mockResolvedValue(undefined);
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

function renderPage() {
  return render(
    <MantineProvider>
      <Conflux />
    </MantineProvider>
  );
}

it("explains the game setting and keeps follow-up work explicit", () => {
  renderPage();

  expect(screen.getByRole("heading", { name: "극돈공소" })).toBeTruthy();
  expect(screen.getByText("극돈공소 무조작 시 설정을 ON으로 켜세요.")).toBeTruthy();
  expect(screen.getByText("보상 선택과 재진입은 후속 작업입니다.")).toBeTruthy();
});

it("enables timer shortening through the page switch", () => {
  renderPage();

  fireEvent.click(screen.getByRole("switch"));

  expect(mocks.setEnabled).toHaveBeenCalledOnce();
  expect(mocks.setEnabled).toHaveBeenCalledWith(true);
});

it("reflects an enabled backend status", () => {
  mocks.status = { state: "on", reason: null };
  renderPage();

  expect((screen.getByRole("switch") as HTMLInputElement).checked).toBe(true);
});

it("disables unavailable control and shows a feature-specific reason", () => {
  mocks.status = { state: "unavailable", reason: "accessDenied" };
  renderPage();

  expect((screen.getByRole("switch") as HTMLInputElement).disabled).toBe(true);
  expect(screen.getByText("게임 메모리에 접근할 수 없습니다.")).toBeTruthy();
});

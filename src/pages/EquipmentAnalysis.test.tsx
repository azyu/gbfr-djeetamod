import { MantineProvider } from "@mantine/core";
import { act, cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { useEquipmentAnalysisStore } from "@/stores/useEquipmentAnalysisStore";
import { EquipmentAnalysisResponse } from "@/types";

import { EquipmentAnalysis } from "./EquipmentAnalysis";

const mocks = vi.hoisted(() => ({
  response: null as EquipmentAnalysisResponse | null,
  listeners: new Map<string, (event: { payload: EquipmentAnalysisResponse }) => void>(),
}));

vi.mock("@tauri-apps/api", () => ({
  invoke: vi.fn(async () => mocks.response),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (name: string, callback: (event: { payload: EquipmentAnalysisResponse }) => void) => {
    mocks.listeners.set(name, callback);
    return vi.fn();
  }),
}));

vi.mock("@/utils", () => ({
  translateTraitId: (id: number) => (id === 1 ? "대미지 상한" : `특성 ${id}`),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) =>
      ({
        "ui.equipment-analysis.title": "진 특성 상한 분석",
        "ui.equipment-analysis.scope": "장착 진 12개만 합산",
        "ui.equipment-analysis.select-character": "캐릭터 선택",
        "ui.equipment-analysis.overflow": "초과",
        "ui.equipment-analysis.capped": "최대",
        "ui.equipment-analysis.below": "정상",
        "ui.equipment-analysis.unknown": "최대치 미확인",
        "ui.equipment-analysis.unsupported": "장비 정보 미지원",
        "ui.equipment-analysis.waiting-game": "게임 연결 대기 중",
        "ui.equipment-analysis.waiting-equipment": "장비 정보 대기 중",
        "ui.equipment-analysis.sources": "기여 장비",
        "characters:Pl2400": "갈란차",
        "characters:Pl2500": "마길라프릴라",
      })[key] ?? key,
  }),
}));

const completeResponse: EquipmentAnalysisResponse = {
  connected: true,
  characters: [
    {
      characterType: "Pl2400",
      status: "complete",
      traits: [
        {
          traitId: 2,
          totalLevel: 15,
          maxLevel: null,
          overflowLevel: 0,
          state: "unknown",
          sources: [],
        },
        {
          traitId: 1,
          totalLevel: 72,
          maxLevel: 65,
          overflowLevel: 7,
          state: "overflow",
          sources: [],
        },
      ],
    },
  ],
};

beforeEach(() => {
  globalThis.ResizeObserver = class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
  window.matchMedia = vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }));
  mocks.response = completeResponse;
  mocks.listeners.clear();
  useEquipmentAnalysisStore.getState().reset();
});

afterEach(cleanup);

function renderPage() {
  return render(
    <MantineProvider>
      <EquipmentAnalysis />
    </MantineProvider>
  );
}

it("shows overflow first and preserves unknown caps", async () => {
  renderPage();

  expect(await screen.findByText("72 / 65")).toBeTruthy();
  expect(screen.getByText("7 초과")).toBeTruthy();
  expect(screen.getByText("최대치 미확인")).toBeTruthy();
  const rows = screen.getAllByTestId("trait-row");
  expect(within(rows[0]).getByText("대미지 상한")).toBeTruthy();
});

it("preserves a selected character while it remains in an update", () => {
  const store = useEquipmentAnalysisStore.getState();
  store.loadResponse({
    connected: true,
    characters: [
      { characterType: "Pl2400", status: "complete", traits: [] },
      { characterType: "Pl2500", status: "complete", traits: [] },
    ],
  });
  store.selectCharacter("Pl2500");
  store.loadResponse({
    connected: true,
    characters: [
      { characterType: "Pl2500", status: "complete", traits: [] },
      { characterType: "Pl2400", status: "complete", traits: [] },
    ],
  });

  expect(useEquipmentAnalysisStore.getState().selectedCharacter).toBe("Pl2500");
});

it("shows disconnected and unsupported states without numeric traits", async () => {
  mocks.response = { connected: false, characters: [] };
  const view = renderPage();
  expect(await screen.findByText("게임 연결 대기 중")).toBeTruthy();

  act(() => {
    mocks.listeners.get("equipment-analysis-update")?.({
      payload: {
        connected: true,
        characters: [{ characterType: "Pl2400", status: "unsupported", traits: [] }],
      },
    });
  });
  expect(await screen.findByText("장비 정보 미지원")).toBeTruthy();
  expect(view.queryByText(/\d+ \/ \d+/)).toBeNull();
});

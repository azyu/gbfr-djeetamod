import { MantineProvider } from "@mantine/core";
import { act, cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import equipmentFixture from "@/fixtures/equipment-analysis-response.json";
import { useEquipmentAnalysisStore } from "@/stores/useEquipmentAnalysisStore";

import { EquipmentAnalysis } from "./EquipmentAnalysis";

const mocks = vi.hoisted(() => ({
  response: null as unknown,
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
}));

vi.mock("@tauri-apps/api", () => ({
  invoke: vi.fn(async () => mocks.response),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (name: string, callback: (event: { payload: unknown }) => void) => {
    mocks.listeners.set(name, callback);
    return vi.fn();
  }),
}));

vi.mock("@/utils", () => ({
  translateTraitId: (id: number) =>
    id === 3696775008 ? "데미지 상한" : `알 수 없는 특성 (0x${id.toString(16).padStart(8, "0")})`,
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
        "ui.equipment-analysis.source.sigilPrimary": "진 주 특성",
        "ui.equipment-analysis.source.sigilSecondary": "진 보조 특성",
        "characters:Pl1400": "나루메아",
        "characters:Pl2400": "갈란차",
        "characters:Pl2500": "마길라프릴라",
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
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }));
  mocks.response = equipmentFixture;
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

it("renders the shared Narmaya overflow contract", async () => {
  renderPage();

  expect(await screen.findByText("70 / 65")).toBeTruthy();
  expect(screen.getByText("5 초과")).toBeTruthy();
  expect(screen.getByText("나루메아")).toBeTruthy();
  expect(screen.getAllByText(/\+15$/).length).toBeGreaterThan(0);
  const rows = screen.getAllByTestId("trait-row");
  expect(within(rows[0]).getByText("데미지 상한")).toBeTruthy();
});

it("does not blank the page when a stale source uses snake-case fields", async () => {
  const staleResponse = structuredClone(equipmentFixture) as unknown as {
    characters: Array<{ traits: Array<{ sources: unknown[] }> }>;
  };
  staleResponse.characters[0].traits[0].sources = [
    {
      kind: "sigilPrimary",
      slot: 0,
      item_id: 123,
      trait_id: 3696775008,
      trait_level: 15,
    },
  ];
  mocks.response = staleResponse;

  renderPage();

  expect(await screen.findByText("70 / 65")).toBeTruthy();
  expect(screen.getByText("5 초과")).toBeTruthy();
  expect(screen.queryByText(/0x0000007b/)).toBeNull();
});

it("keeps level and cap state visible for an unresolved trait name", async () => {
  mocks.response = {
    connected: true,
    characters: [
      {
        characterType: "Pl1400",
        status: "complete",
        traits: [
          {
            traitId: 0x0151cf9e,
            totalLevel: 15,
            maxLevel: null,
            overflowLevel: 0,
            state: "unknown",
            sources: [],
          },
        ],
      },
    ],
  };

  renderPage();

  expect(await screen.findByText("알 수 없는 특성 (0x0151cf9e)")).toBeTruthy();
  expect(screen.getByText("15 / —")).toBeTruthy();
  expect(screen.getByText("최대치 미확인")).toBeTruthy();
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

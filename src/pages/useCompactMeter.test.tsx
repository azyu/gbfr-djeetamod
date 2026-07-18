import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { EncounterState } from "@/types";

import useCompactMeter from "./useCompactMeter";

const mocks = vi.hoisted(() => ({
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (name: string, callback: (event: { payload: unknown }) => void) => {
    mocks.listeners.set(name, callback);
    return vi.fn();
  }),
}));

vi.mock("@/stores/useMeterSettingsStore", () => ({
  useMeterSettingsStore: (selector: (state: { transparency: number }) => unknown) => selector({ transparency: 0.72 }),
}));

const activeEncounter: EncounterState = {
  totalDamage: 1_000,
  dps: 250,
  startTime: 1_000,
  endTime: 5_000,
  status: "InProgress",
  targets: {},
  party: {
    7: {
      index: 7,
      characterType: "Pl1400",
      totalDamage: 1_000,
      dps: 250,
      sba: 0,
      totalStunValue: 0,
      stunPerSecond: 0,
      lastDamageTime: 5_000,
      skillBreakdown: [],
    },
  },
};

beforeEach(() => {
  vi.useFakeTimers();
  mocks.listeners.clear();
});

afterEach(() => {
  vi.useRealTimers();
});

it("publishes only the newest encounter every 250ms and clears on disconnect", async () => {
  const { result } = renderHook(() => useCompactMeter());
  await act(async () => Promise.resolve());

  act(() => mocks.listeners.get("connection-state")?.({ payload: "connected" }));
  act(() => mocks.listeners.get("encounter-update")?.({ payload: activeEncounter }));
  expect(result.current.encounterState.totalDamage).toBe(0);

  act(() => vi.advanceTimersByTime(250));
  expect(result.current.encounterState.totalDamage).toBe(1_000);
  expect(result.current.rows).toHaveLength(1);

  act(() => mocks.listeners.get("connection-state")?.({ payload: "disconnected" }));
  act(() => vi.advanceTimersByTime(250));
  expect(result.current.rows).toEqual([]);
});

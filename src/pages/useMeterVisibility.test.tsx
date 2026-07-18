import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";

import { useMeterSettingsStore } from "@/stores/useMeterSettingsStore";

import useMeterVisibility from "./useMeterVisibility";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api", () => ({
  invoke: mocks.invoke,
}));

beforeEach(() => {
  mocks.invoke.mockReset();
  mocks.invoke.mockResolvedValue(undefined);
  useMeterSettingsStore.persist.clearStorage();
  useMeterSettingsStore.setState({ meter_enabled: true });
});

it("defaults the meter to enabled and synchronizes it on mount", async () => {
  expect(useMeterSettingsStore.getState().meter_enabled).toBe(true);

  const { result } = renderHook(() => useMeterVisibility());

  expect(result.current.meterEnabled).toBe(true);
  await waitFor(() =>
    expect(mocks.invoke).toHaveBeenCalledWith("set_meter_enabled", {
      enabled: true,
    })
  );
});

it("persists a visibility change after the window command succeeds", async () => {
  const { result } = renderHook(() => useMeterVisibility());
  await waitFor(() => expect(mocks.invoke).toHaveBeenCalledTimes(1));
  mocks.invoke.mockClear();

  await act(async () => result.current.setMeterEnabled(false));

  expect(mocks.invoke).toHaveBeenCalledWith("set_meter_enabled", {
    enabled: false,
  });
  expect(useMeterSettingsStore.getState().meter_enabled).toBe(false);
});

it("keeps the persisted state when the window command fails", async () => {
  const { result } = renderHook(() => useMeterVisibility());
  await waitFor(() => expect(mocks.invoke).toHaveBeenCalledTimes(1));
  mocks.invoke.mockRejectedValueOnce(new Error("window unavailable"));

  await act(async () => {
    await expect(result.current.setMeterEnabled(false)).rejects.toThrow("window unavailable");
  });

  expect(useMeterSettingsStore.getState().meter_enabled).toBe(true);
});

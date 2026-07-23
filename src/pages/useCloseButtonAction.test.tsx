import { useMeterSettingsStore } from "@/stores/useMeterSettingsStore";
import { invoke } from "@tauri-apps/api";
import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import useCloseButtonAction from "./useCloseButtonAction";

vi.mock("@tauri-apps/api", () => ({ invoke: vi.fn().mockResolvedValue(undefined) }));

beforeEach(() => {
  localStorage.clear();
  useMeterSettingsStore.setState({ close_button_action: "minimize-to-tray" });
  vi.mocked(invoke).mockClear();
});

it("defaults to minimizing the management window to the tray", async () => {
  renderHook(() => useCloseButtonAction());

  await waitFor(() => expect(invoke).toHaveBeenCalledWith("set_close_to_tray", { enabled: true }));
});

it("synchronizes a persisted quit selection", async () => {
  renderHook(() => useCloseButtonAction());

  act(() => useMeterSettingsStore.getState().set({ close_button_action: "quit" }));

  await waitFor(() => expect(invoke).toHaveBeenLastCalledWith("set_close_to_tray", { enabled: false }));
  expect(useMeterSettingsStore.getState().close_button_action).toBe("quit");
});

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";

import useConfluxTimer, { ConfluxTimerStatus } from "./useConfluxTimer";

const mocks = vi.hoisted(() => ({
  status: { state: "off", reason: null } as ConfluxTimerStatus,
  setResult: { state: "on", reason: null } as ConfluxTimerStatus,
  rejectSet: false,
  invoke: vi.fn(),
  listeners: new Map<string, (event: { payload: string }) => void>(),
}));

vi.mock("@tauri-apps/api", () => ({ invoke: mocks.invoke }));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (name: string, callback: (event: { payload: string }) => void) => {
    mocks.listeners.set(name, callback);
    return vi.fn();
  }),
}));

beforeEach(() => {
  mocks.status = { state: "off", reason: null };
  mocks.setResult = { state: "on", reason: null };
  mocks.rejectSet = false;
  mocks.listeners.clear();
  mocks.invoke.mockReset();
  mocks.invoke.mockImplementation(async (command: string) => {
    if (command === "get_conflux_timer_status") return mocks.status;
    if (command === "set_conflux_timer_enabled") {
      if (mocks.rejectSet) throw new Error("invoke failed");
      return mocks.setResult;
    }
    throw new Error(`unexpected command: ${command}`);
  });
});

it("loads the backend-observed timer status", async () => {
  const { result } = renderHook(() => useConfluxTimer());

  expect(result.current.status).toBeNull();
  expect(result.current.pending).toBe(true);
  await waitFor(() => expect(result.current.status).toEqual({ state: "off", reason: null }));
});

it("refreshes the timer status on a connection-state event", async () => {
  const { result } = renderHook(() => useConfluxTimer());
  await waitFor(() => expect(result.current.status?.state).toBe("off"));

  mocks.status = { state: "on", reason: null };
  await act(async () => mocks.listeners.get("connection-state")?.({ payload: "connected" }));

  await waitFor(() => expect(result.current.status?.state).toBe("on"));
  expect(mocks.invoke).toHaveBeenLastCalledWith("get_conflux_timer_status");
});

it("uses the backend-observed enable result", async () => {
  const { result } = renderHook(() => useConfluxTimer());
  await waitFor(() => expect(result.current.status?.state).toBe("off"));

  await act(async () => result.current.setEnabled(true));

  expect(mocks.invoke).toHaveBeenLastCalledWith("set_conflux_timer_enabled", { enabled: true });
  expect(result.current.status).toEqual({ state: "on", reason: null });
});

it("preserves the observed state when an enable invoke rejects", async () => {
  const { result } = renderHook(() => useConfluxTimer());
  await waitFor(() => expect(result.current.status?.state).toBe("off"));
  mocks.rejectSet = true;

  await act(async () => result.current.setEnabled(true));

  expect(result.current.status).toEqual({ state: "off", reason: "internal" });
  expect(result.current.pending).toBe(false);
});

import { ConnectionState } from "@/types";
import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";

import useConnectionState from "./useConnectionState";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  listeners: new Map<string, (event: { payload: ConnectionState }) => void>(),
  unlisten: vi.fn(),
}));

vi.mock("@tauri-apps/api", () => ({
  invoke: mocks.invoke,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: mocks.listen,
}));

const deferred = <T,>() => {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
};

beforeEach(() => {
  mocks.listeners.clear();
  mocks.invoke.mockReset();
  mocks.invoke.mockResolvedValue("connected");
  mocks.unlisten.mockReset();
  mocks.listen.mockReset();
  mocks.listen.mockImplementation(
    async (name: string, callback: (event: { payload: ConnectionState }) => void) => {
      mocks.listeners.set(name, callback);
      return mocks.unlisten;
    }
  );
});

it("reads the current state after registering the listener", async () => {
  const { result } = renderHook(() => useConnectionState());

  expect(result.current).toBe("searching");
  await waitFor(() => expect(result.current).toBe("connected"));
  expect(mocks.listen).toHaveBeenCalledWith("connection-state", expect.any(Function));
  expect(mocks.invoke).toHaveBeenCalledWith("get_connection_state");
  expect(mocks.listen.mock.invocationCallOrder[0]).toBeLessThan(mocks.invoke.mock.invocationCallOrder[0]);
});

it("uses an event that arrives before the initial read resolves", async () => {
  const initial = deferred<ConnectionState>();
  mocks.invoke.mockReturnValueOnce(initial.promise);
  const { result } = renderHook(() => useConnectionState());
  await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("get_connection_state"));

  act(() => mocks.listeners.get("connection-state")?.({ payload: "disconnected" }));
  initial.resolve("connected");

  await waitFor(() => expect(result.current).toBe("disconnected"));
});

it("keeps searching when the initial read fails", async () => {
  mocks.invoke.mockRejectedValueOnce(new Error("invoke failed"));
  const { result } = renderHook(() => useConnectionState());

  await waitFor(() => expect(mocks.invoke).toHaveBeenCalled());
  expect(result.current).toBe("searching");
});

it("unsubscribes and ignores late initial results after unmount", async () => {
  const initial = deferred<ConnectionState>();
  mocks.invoke.mockReturnValueOnce(initial.promise);
  const { unmount } = renderHook(() => useConnectionState());
  await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("get_connection_state"));

  unmount();
  initial.resolve("connected");

  await waitFor(() => expect(mocks.unlisten).toHaveBeenCalledTimes(1));
});

import { act, renderHook, waitFor } from "@testing-library/react";
import type { PropsWithChildren } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  checkUpdate: vi.fn(),
  getVersion: vi.fn(),
  installUpdate: vi.fn(),
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/app", () => ({ getVersion: mocks.getVersion }));
vi.mock("@tauri-apps/api/tauri", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/updater", () => ({
  checkUpdate: mocks.checkUpdate,
  installUpdate: mocks.installUpdate,
}));

import { UpdaterProvider, useUpdater } from "./useUpdater";

const wrapper = ({ children }: PropsWithChildren) => <UpdaterProvider>{children}</UpdaterProvider>;

describe("UpdaterProvider", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.getVersion.mockResolvedValue("0.1.1");
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("checks once on provider mount and stays idle when the automatic check fails", async () => {
    mocks.checkUpdate.mockRejectedValue(new Error("offline"));
    const warning = vi.spyOn(console, "warn").mockImplementation(() => undefined);

    const { result } = renderHook(() => useUpdater(), { wrapper });

    await waitFor(() => expect(mocks.checkUpdate).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(result.current.state.currentVersion).toBe("0.1.1"));
    expect(result.current.state).toEqual({
      phase: "idle",
      currentVersion: "0.1.1",
      manifest: null,
      error: null,
    });
    expect(warning).toHaveBeenCalledTimes(1);
  });

  it("reports upToDate after a successful manual check", async () => {
    mocks.checkUpdate.mockResolvedValue({ shouldUpdate: false });
    const { result } = renderHook(() => useUpdater(), { wrapper });
    await waitFor(() => expect(mocks.checkUpdate).toHaveBeenCalledTimes(1));

    await act(() => result.current.checkForUpdate("manual"));

    expect(mocks.checkUpdate).toHaveBeenCalledTimes(2);
    expect(result.current.state.phase).toBe("upToDate");
    expect(result.current.state.manifest).toBeNull();
  });

  it("retains the complete manifest when a newer version is available", async () => {
    const manifest = {
      version: "0.1.2",
      date: "2026-07-22T00:00:00Z",
      body: "Signed update",
    };
    mocks.checkUpdate.mockResolvedValue({ shouldUpdate: true, manifest });

    const { result } = renderHook(() => useUpdater(), { wrapper });

    await waitFor(() => expect(result.current.state.phase).toBe("available"));
    expect(result.current.state.manifest).toEqual(manifest);
  });

  it("shares one operation for concurrent checks", async () => {
    mocks.checkUpdate.mockResolvedValueOnce({ shouldUpdate: false });
    const { result } = renderHook(() => useUpdater(), { wrapper });
    await waitFor(() => expect(mocks.checkUpdate).toHaveBeenCalledTimes(1));

    let resolveCheck: ((value: { shouldUpdate: boolean }) => void) | undefined;
    mocks.checkUpdate.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveCheck = resolve;
        })
    );
    mocks.checkUpdate.mockResolvedValue({ shouldUpdate: false });

    let first!: Promise<void>;
    let second!: Promise<void>;
    act(() => {
      first = result.current.checkForUpdate("manual");
      second = result.current.checkForUpdate("manual");
    });

    expect(mocks.checkUpdate).toHaveBeenCalledTimes(2);
    resolveCheck?.({ shouldUpdate: false });
    await act(() => Promise.all([first, second]));
    expect(result.current.state.phase).toBe("upToDate");
  });

  it("reports a manual check failure without throwing", async () => {
    mocks.checkUpdate.mockResolvedValueOnce({ shouldUpdate: false });
    const { result } = renderHook(() => useUpdater(), { wrapper });
    await waitFor(() => expect(mocks.checkUpdate).toHaveBeenCalledTimes(1));
    mocks.checkUpdate.mockRejectedValueOnce(new Error("offline"));

    await act(() => result.current.checkForUpdate("manual"));

    expect(result.current.state.phase).toBe("error");
    expect(result.current.state.error).toBe("checkFailed");
  });

  it("restores repeat quest and blocks install while the game is running", async () => {
    mocks.checkUpdate.mockResolvedValue({
      shouldUpdate: true,
      manifest: { version: "0.1.2", date: "2026-07-22T00:00:00Z", body: "Signed update" },
    });
    mocks.invoke.mockResolvedValue("gameRunning");
    const { result } = renderHook(() => useUpdater(), { wrapper });
    await waitFor(() => expect(result.current.state.phase).toBe("available"));

    await act(() => result.current.installAvailableUpdate());

    expect(mocks.invoke).toHaveBeenCalledWith("prepare_update_install");
    expect(mocks.installUpdate).not.toHaveBeenCalled();
    expect(result.current.state.phase).toBe("error");
    expect(result.current.state.error).toBe("gameRunning");
    expect(result.current.state.manifest?.version).toBe("0.1.2");
  });

  it("calls installUpdate only after backend readiness is ready", async () => {
    mocks.checkUpdate.mockResolvedValue({
      shouldUpdate: true,
      manifest: { version: "0.1.2", date: "2026-07-22T00:00:00Z", body: "Signed update" },
    });
    mocks.invoke.mockResolvedValue("ready");
    mocks.installUpdate.mockResolvedValue(undefined);
    const { result } = renderHook(() => useUpdater(), { wrapper });
    await waitFor(() => expect(result.current.state.phase).toBe("available"));

    await act(() => result.current.installAvailableUpdate());

    expect(mocks.invoke).toHaveBeenCalledWith("prepare_update_install");
    expect(mocks.installUpdate).toHaveBeenCalledTimes(1);
    expect(result.current.state.phase).toBe("installing");
  });

  it("reports repeatQuestRestoreFailed without calling installUpdate", async () => {
    mocks.checkUpdate.mockResolvedValue({
      shouldUpdate: true,
      manifest: { version: "0.1.2", date: "2026-07-22T00:00:00Z", body: "Signed update" },
    });
    mocks.invoke.mockResolvedValue("repeatQuestRestoreFailed");
    const { result } = renderHook(() => useUpdater(), { wrapper });
    await waitFor(() => expect(result.current.state.phase).toBe("available"));

    await act(() => result.current.installAvailableUpdate());

    expect(mocks.installUpdate).not.toHaveBeenCalled();
    expect(result.current.state.phase).toBe("error");
    expect(result.current.state.error).toBe("repeatQuestRestoreFailed");
  });

  it("reports installFailed when installUpdate rejects", async () => {
    mocks.checkUpdate.mockResolvedValue({
      shouldUpdate: true,
      manifest: { version: "0.1.2", date: "2026-07-22T00:00:00Z", body: "Signed update" },
    });
    mocks.invoke.mockResolvedValue("ready");
    mocks.installUpdate.mockRejectedValue(new Error("invalid signature"));
    const { result } = renderHook(() => useUpdater(), { wrapper });
    await waitFor(() => expect(result.current.state.phase).toBe("available"));

    await act(() => result.current.installAvailableUpdate());

    expect(result.current.state.phase).toBe("error");
    expect(result.current.state.error).toBe("installFailed");
    expect(result.current.state.manifest?.version).toBe("0.1.2");
  });

  it("shares one operation for concurrent installs", async () => {
    mocks.checkUpdate.mockResolvedValue({
      shouldUpdate: true,
      manifest: { version: "0.1.2", date: "2026-07-22T00:00:00Z", body: "Signed update" },
    });
    mocks.invoke.mockResolvedValue("ready");
    let resolveInstall: (() => void) | undefined;
    mocks.installUpdate.mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          resolveInstall = resolve;
        })
    );
    const { result } = renderHook(() => useUpdater(), { wrapper });
    await waitFor(() => expect(result.current.state.phase).toBe("available"));

    let first!: Promise<void>;
    let second!: Promise<void>;
    act(() => {
      first = result.current.installAvailableUpdate();
      second = result.current.installAvailableUpdate();
    });
    await waitFor(() => expect(mocks.installUpdate).toHaveBeenCalled());

    expect(mocks.invoke).toHaveBeenCalledTimes(1);
    expect(mocks.installUpdate).toHaveBeenCalledTimes(1);
    resolveInstall?.();
    await act(() => Promise.all([first, second]));
  });

  it("dismisses an available update for the current process", async () => {
    mocks.checkUpdate.mockResolvedValue({
      shouldUpdate: true,
      manifest: { version: "0.1.2", date: "2026-07-22T00:00:00Z", body: "Signed update" },
    });
    const { result } = renderHook(() => useUpdater(), { wrapper });
    await waitFor(() => expect(result.current.state.phase).toBe("available"));

    act(() => result.current.dismissUpdate());

    expect(result.current.state).toEqual({
      phase: "idle",
      currentVersion: "0.1.1",
      manifest: null,
      error: null,
    });
  });
});

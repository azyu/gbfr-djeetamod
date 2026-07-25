import { act, renderHook, waitFor } from "@testing-library/react";
import type { PropsWithChildren } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  checkUpdate: vi.fn(),
  getVersion: vi.fn(),
  invoke: vi.fn(),
  listen: vi.fn(),
  listeners: new Map<string, (event: { payload: { chunkLength: number; contentLength: number | null } }) => void>(),
}));

vi.mock("@tauri-apps/api/app", () => ({ getVersion: mocks.getVersion }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: mocks.listen,
  TauriEvent: { DOWNLOAD_PROGRESS: "tauri://update-download-progress" },
}));
vi.mock("@tauri-apps/api/tauri", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/updater", () => ({
  checkUpdate: mocks.checkUpdate,
}));

import { UpdaterProvider, useUpdater } from "./useUpdater";

const wrapper = ({ children }: PropsWithChildren) => <UpdaterProvider>{children}</UpdaterProvider>;

describe("UpdaterProvider", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.getVersion.mockResolvedValue("0.1.1");
    mocks.listeners.clear();
    mocks.listen.mockImplementation(
      async (
        event: string,
        handler: (event: { payload: { chunkLength: number; contentLength: number | null } }) => void
      ) => {
        mocks.listeners.set(event, handler);
        return vi.fn();
      }
    );
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
      downloadProgress: null,
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
    expect(result.current.state.phase).toBe("error");
    expect(result.current.state.error).toBe("gameRunning");
    expect(result.current.state.manifest?.version).toBe("0.1.2");
  });

  it("publishes cumulative native download progress while installing", async () => {
    mocks.checkUpdate.mockResolvedValue({
      shouldUpdate: true,
      manifest: { version: "0.1.2", date: "2026-07-22T00:00:00Z", body: "Signed update" },
    });
    let resolveInstall: (() => void) | undefined;
    mocks.invoke.mockResolvedValueOnce("ready").mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          resolveInstall = resolve;
        })
    );
    const { result } = renderHook(() => useUpdater(), { wrapper });
    await waitFor(() => expect(result.current.state.phase).toBe("available"));

    let installation!: Promise<void>;
    act(() => {
      installation = result.current.installAvailableUpdate();
    });
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledTimes(2));

    act(() => {
      mocks.listeners.get("tauri://update-download-progress")?.({
        payload: { chunkLength: 2048, contentLength: 8192 },
      });
      mocks.listeners.get("tauri://update-download-progress")?.({
        payload: { chunkLength: 1024, contentLength: 8192 },
      });
    });

    expect(result.current.state.downloadProgress).toEqual({
      downloadedBytes: 3072,
      totalBytes: 8192,
    });
    resolveInstall?.();
    await act(() => installation);
  });

  it("uses the timeout-controlled backend installer after readiness succeeds", async () => {
    mocks.checkUpdate.mockResolvedValue({
      shouldUpdate: true,
      manifest: { version: "0.1.2", date: "2026-07-22T00:00:00Z", body: "Signed update" },
    });
    mocks.invoke.mockResolvedValueOnce("ready").mockResolvedValueOnce(undefined);
    const { result } = renderHook(() => useUpdater(), { wrapper });
    await waitFor(() => expect(result.current.state.phase).toBe("available"));

    await act(() => result.current.installAvailableUpdate());

    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "prepare_update_install");
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "install_available_update");
  });

  it("reports installFailed when backend preparation rejects", async () => {
    mocks.checkUpdate.mockResolvedValue({
      shouldUpdate: true,
      manifest: { version: "0.1.2", date: "2026-07-22T00:00:00Z", body: "Signed update" },
    });
    mocks.invoke.mockRejectedValue(new Error("backend unavailable"));
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    const { result } = renderHook(() => useUpdater(), { wrapper });
    await waitFor(() => expect(result.current.state.phase).toBe("available"));

    await act(() => result.current.installAvailableUpdate());

    expect(result.current.state.phase).toBe("error");
    expect(result.current.state.error).toBe("installFailed");
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

    expect(result.current.state.phase).toBe("error");
    expect(result.current.state.error).toBe("repeatQuestRestoreFailed");
  });

  it("reports installFailed when the timeout-controlled backend installer rejects", async () => {
    mocks.checkUpdate.mockResolvedValue({
      shouldUpdate: true,
      manifest: { version: "0.1.2", date: "2026-07-22T00:00:00Z", body: "Signed update" },
    });
    mocks.invoke.mockResolvedValueOnce("ready").mockRejectedValueOnce(new Error("request timed out"));
    vi.spyOn(console, "error").mockImplementation(() => undefined);
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
    let resolveInstall: (() => void) | undefined;
    mocks.invoke.mockResolvedValueOnce("ready").mockImplementationOnce(
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
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledTimes(2));

    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "prepare_update_install");
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "install_available_update");
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
      downloadProgress: null,
    });
  });
});

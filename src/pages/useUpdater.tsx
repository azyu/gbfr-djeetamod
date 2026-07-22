import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/tauri";
import type { UpdateManifest } from "@tauri-apps/api/updater";
import { checkUpdate, installUpdate } from "@tauri-apps/api/updater";
import { createContext, useCallback, useContext, useEffect, useRef, useState, type PropsWithChildren } from "react";

export type UpdaterPhase = "idle" | "checking" | "upToDate" | "available" | "preparing" | "installing" | "error";

export type UpdaterError = "checkFailed" | "gameRunning" | "repeatQuestRestoreFailed" | "installFailed";

export type UpdaterState = {
  phase: UpdaterPhase;
  currentVersion: string;
  manifest: UpdateManifest | null;
  error: UpdaterError | null;
};

type CheckMode = "automatic" | "manual";
type UpdateInstallReadiness = "ready" | "gameRunning" | "repeatQuestRestoreFailed";

export type UpdaterController = {
  state: UpdaterState;
  checkForUpdate: (mode: CheckMode) => Promise<void>;
  installAvailableUpdate: () => Promise<void>;
  dismissUpdate: () => void;
};

const UpdaterContext = createContext<UpdaterController | null>(null);

export const useUpdater = () => {
  const value = useContext(UpdaterContext);
  if (!value) throw new Error("useUpdater must be used inside UpdaterProvider");
  return value;
};

export const UpdaterProvider = ({ children }: PropsWithChildren) => {
  const [state, setState] = useState<UpdaterState>({
    phase: "idle",
    currentVersion: "",
    manifest: null,
    error: null,
  });
  const checkPromiseRef = useRef<Promise<void> | null>(null);
  const installPromiseRef = useRef<Promise<void> | null>(null);

  const checkForUpdate = useCallback((mode: CheckMode) => {
    if (checkPromiseRef.current) return checkPromiseRef.current;

    const operation = (async () => {
      setState((current) => ({ ...current, phase: "checking", error: null }));
      try {
        const result = await checkUpdate();
        setState((current) => ({
          ...current,
          phase: result.shouldUpdate && result.manifest ? "available" : "upToDate",
          manifest: result.shouldUpdate && result.manifest ? result.manifest : null,
          error: null,
        }));
      } catch (error) {
        if (mode === "automatic") {
          console.warn("Automatic update check failed", error);
          setState((current) => ({ ...current, phase: "idle", manifest: null, error: null }));
          return;
        }
        setState((current) => ({ ...current, phase: "error", error: "checkFailed" }));
      }
    })();
    checkPromiseRef.current = operation;
    void operation.then(
      () => {
        if (checkPromiseRef.current === operation) checkPromiseRef.current = null;
      },
      () => {
        if (checkPromiseRef.current === operation) checkPromiseRef.current = null;
      }
    );
    return operation;
  }, []);

  const installAvailableUpdate = useCallback(() => {
    if (installPromiseRef.current) return installPromiseRef.current;
    if (!state.manifest) return Promise.resolve();

    const operation = (async () => {
      setState((current) => ({ ...current, phase: "preparing", error: null }));
      const readiness = await invoke<UpdateInstallReadiness>("prepare_update_install");
      if (readiness === "gameRunning") {
        setState((current) => ({ ...current, phase: "error", error: "gameRunning" }));
        return;
      }
      if (readiness === "repeatQuestRestoreFailed") {
        setState((current) => ({ ...current, phase: "error", error: "repeatQuestRestoreFailed" }));
        return;
      }

      setState((current) => ({ ...current, phase: "installing", error: null }));
      try {
        await installUpdate();
      } catch {
        setState((current) => ({ ...current, phase: "error", error: "installFailed" }));
      }
    })();
    installPromiseRef.current = operation;
    void operation.then(
      () => {
        if (installPromiseRef.current === operation) installPromiseRef.current = null;
      },
      () => {
        if (installPromiseRef.current === operation) installPromiseRef.current = null;
      }
    );
    return operation;
  }, [state.manifest]);

  const dismissUpdate = useCallback(() => {
    setState((current) => ({ ...current, phase: "idle", manifest: null, error: null }));
  }, []);

  useEffect(() => {
    let disposed = false;

    void getVersion().then((currentVersion) => {
      if (!disposed) setState((current) => ({ ...current, currentVersion }));
    });

    void checkForUpdate("automatic");

    return () => {
      disposed = true;
    };
  }, [checkForUpdate]);

  return (
    <UpdaterContext.Provider value={{ state, checkForUpdate, installAvailableUpdate, dismissUpdate }}>
      {children}
    </UpdaterContext.Provider>
  );
};

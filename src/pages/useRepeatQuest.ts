import { invoke } from "@tauri-apps/api";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";

export type RepeatQuestReason =
  | "busy"
  | "gameNotRunning"
  | "unsupportedGame"
  | "signatureMissing"
  | "signatureAmbiguous"
  | "unexpectedBytes"
  | "accessDenied"
  | "patchFailed"
  | "restoreFailed"
  | "internal";

export type RepeatQuestStatus = {
  state: "unavailable" | "off" | "on";
  reason: RepeatQuestReason | null;
};

export default function useRepeatQuest() {
  const [status, setStatus] = useState<RepeatQuestStatus | null>(null);
  const [pending, setPending] = useState(true);

  useEffect(() => {
    let disposed = false;
    let unsubscribe: (() => void) | undefined;

    const refresh = async () => {
      setPending(true);
      try {
        const nextStatus = await invoke<RepeatQuestStatus>("get_repeat_quest_status");
        if (!disposed) setStatus(nextStatus);
      } catch {
        if (!disposed) setStatus({ state: "unavailable", reason: "internal" });
      } finally {
        if (!disposed) setPending(false);
      }
    };

    void refresh();
    void listen("connection-state", () => void refresh()).then((unlisten) => {
      if (disposed) {
        unlisten();
      } else {
        unsubscribe = unlisten;
      }
    });

    return () => {
      disposed = true;
      unsubscribe?.();
    };
  }, []);

  const setEnabled = useCallback(async (enabled: boolean) => {
    setPending(true);
    try {
      const nextStatus = await invoke<RepeatQuestStatus>("set_repeat_quest_enabled", { enabled });
      setStatus(nextStatus);
    } catch {
      setStatus((previous) =>
        previous ? { ...previous, reason: "internal" } : { state: "unavailable", reason: "internal" }
      );
    } finally {
      setPending(false);
    }
  }, []);

  return { status, pending, setEnabled };
}

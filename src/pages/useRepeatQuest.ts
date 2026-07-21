import { invoke } from "@tauri-apps/api";
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

export default function useRepeatQuest(connectionState: string) {
  const [status, setStatus] = useState<RepeatQuestStatus | null>(null);
  const [pending, setPending] = useState(true);

  useEffect(() => {
    let active = true;
    setPending(true);
    void invoke<RepeatQuestStatus>("get_repeat_quest_status")
      .then((nextStatus) => {
        if (active) setStatus(nextStatus);
      })
      .catch(() => {
        if (active) setStatus({ state: "unavailable", reason: "internal" });
      })
      .finally(() => {
        if (active) setPending(false);
      });

    return () => {
      active = false;
    };
  }, [connectionState]);

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

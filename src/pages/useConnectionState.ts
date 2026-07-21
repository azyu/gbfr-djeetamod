import { ConnectionState } from "@/types";
import { invoke } from "@tauri-apps/api";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

export default function useConnectionState(): ConnectionState {
  const [connectionState, setConnectionState] = useState<ConnectionState>("searching");

  useEffect(() => {
    let disposed = false;
    let eventSeen = false;
    let unsubscribe: (() => void) | undefined;

    void listen<ConnectionState>("connection-state", (event) => {
      eventSeen = true;
      if (!disposed) setConnectionState(event.payload);
    })
      .then(async (unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        unsubscribe = unlisten;

        try {
          const currentState = await invoke<ConnectionState>("get_connection_state");
          if (!disposed && !eventSeen) setConnectionState(currentState);
        } catch {
          // Keep searching until a backend event arrives.
        }
      })
      .catch(() => undefined);

    return () => {
      disposed = true;
      unsubscribe?.();
    };
  }, []);

  return connectionState;
}

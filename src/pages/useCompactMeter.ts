import { invoke } from "@tauri-apps/api";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";

import { buildCompactMeterRows } from "@/components/compact-meter/compactMeterModel";
import { useMeterSettingsStore } from "@/stores/useMeterSettingsStore";
import { ConnectionState, EncounterState } from "@/types";

export const UPDATE_INTERVAL_MS = 250;
export const DEFAULT_ENCOUNTER_STATE: EncounterState = {
  totalDamage: 0,
  dps: 0,
  startTime: 0,
  endTime: 0,
  party: {},
  targets: {},
  status: "Waiting",
};

export default function useCompactMeter() {
  const [encounterState, setEncounterState] = useState(DEFAULT_ENCOUNTER_STATE);
  const [connectionState, setConnectionState] = useState<ConnectionState>("searching");
  const pendingEncounter = useRef(DEFAULT_ENCOUNTER_STATE);
  const { transparency, geometry_initialized, setMeterSettings } = useMeterSettingsStore((state) => ({
    transparency: state.transparency,
    geometry_initialized: state.geometry_initialized,
    setMeterSettings: state.set,
  }));

  useEffect(() => {
    if (geometry_initialized) return;
    void invoke("reset_meter_geometry").then(() => setMeterSettings({ geometry_initialized: true }));
  }, [geometry_initialized, setMeterSettings]);

  useEffect(() => {
    const subscriptions = [
      listen<EncounterState>("encounter-update", (event) => {
        pendingEncounter.current = event.payload;
      }),
      listen<EncounterState>("on-area-enter", () => {
        pendingEncounter.current = DEFAULT_ENCOUNTER_STATE;
      }),
    ];
    const interval = window.setInterval(() => setEncounterState(pendingEncounter.current), UPDATE_INTERVAL_MS);

    return () => {
      window.clearInterval(interval);
      void Promise.all(subscriptions).then((unlisten) => unlisten.forEach((dispose) => dispose()));
    };
  }, []);

  useEffect(() => {
    let disposed = false;
    let unsubscribe: (() => void) | undefined;
    const updateConnection = (state: ConnectionState) => {
      setConnectionState(state);
      if (state !== "connected") pendingEncounter.current = DEFAULT_ENCOUNTER_STATE;
    };

    void listen<ConnectionState>("connection-state", (event) => updateConnection(event.payload)).then(
      async (unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        unsubscribe = unlisten;
        const currentState = await invoke<ConnectionState>("get_connection_state");
        if (!disposed) updateConnection(currentState);
      }
    );

    return () => {
      disposed = true;
      unsubscribe?.();
    };
  }, []);

  const visible = connectionState === "connected" && encounterState.status === "InProgress";
  const rows = visible ? buildCompactMeterRows(encounterState) : [];
  return { encounterState, connectionState, rows, transparency };
}

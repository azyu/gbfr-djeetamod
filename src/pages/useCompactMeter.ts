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
  const transparency = useMeterSettingsStore((state) => state.transparency);

  useEffect(() => {
    const subscriptions = [
      listen<EncounterState>("encounter-update", (event) => {
        pendingEncounter.current = event.payload;
      }),
      listen<EncounterState>("on-area-enter", () => {
        pendingEncounter.current = DEFAULT_ENCOUNTER_STATE;
      }),
      listen<ConnectionState>("connection-state", (event) => {
        setConnectionState(event.payload);
        if (event.payload !== "connected") pendingEncounter.current = DEFAULT_ENCOUNTER_STATE;
      }),
    ];
    const interval = window.setInterval(() => setEncounterState(pendingEncounter.current), UPDATE_INTERVAL_MS);

    return () => {
      window.clearInterval(interval);
      void Promise.all(subscriptions).then((unlisten) => unlisten.forEach((dispose) => dispose()));
    };
  }, []);

  const visible = connectionState === "connected" && encounterState.status === "InProgress";
  const rows = visible ? buildCompactMeterRows(encounterState) : [];
  return { encounterState, connectionState, rows, transparency };
}

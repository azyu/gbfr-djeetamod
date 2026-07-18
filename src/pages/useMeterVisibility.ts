import { useMeterSettingsStore } from "@/stores/useMeterSettingsStore";
import { invoke } from "@tauri-apps/api";
import { useCallback, useEffect, useRef } from "react";

export default function useMeterVisibility() {
  const meterEnabled = useMeterSettingsStore((state) => state.meter_enabled);
  const setMeterSettings = useMeterSettingsStore((state) => state.set);
  const initialMeterEnabled = useRef(meterEnabled);

  useEffect(() => {
    void invoke("set_meter_enabled", {
      enabled: initialMeterEnabled.current,
    }).catch(() => undefined);
  }, []);

  const setMeterEnabled = useCallback(
    async (enabled: boolean) => {
      await invoke("set_meter_enabled", { enabled });
      setMeterSettings({ meter_enabled: enabled });
    },
    [setMeterSettings]
  );

  return { meterEnabled, setMeterEnabled };
}

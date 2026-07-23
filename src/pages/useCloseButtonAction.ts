import { useMeterSettingsStore } from "@/stores/useMeterSettingsStore";
import { invoke } from "@tauri-apps/api";
import { useEffect } from "react";

export default function useCloseButtonAction() {
  const closeButtonAction = useMeterSettingsStore((state) => state.close_button_action);

  useEffect(() => {
    invoke("set_close_to_tray", {
      enabled: closeButtonAction === "minimize-to-tray",
    }).catch((error) => console.error("Failed to synchronize close button action:", error));
  }, [closeButtonAction]);
}

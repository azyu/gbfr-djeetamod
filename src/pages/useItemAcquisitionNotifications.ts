import { listen } from "@tauri-apps/api/event";
import { sendNotification } from "@tauri-apps/api/notification";
import { invoke } from "@tauri-apps/api/tauri";
import { useCallback, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";

import { findAcquiredWarningItems, formatAcquisitionNotificationBody } from "@/itemAcquisitionNotification";
import { normalizeItemInventorySnapshotResponse } from "@/itemAnalysisContract";
import { hasItemNotificationPermission } from "@/itemNotificationPermission";
import { useItemNotificationStore } from "@/stores/useItemNotificationStore";
import { ItemAnalysisEntry } from "@/types";
import { translateItemId } from "@/utils";

const POST_BATTLE_DELAY_MS = 5_000;

export const useItemAcquisitionNotifications = () => {
  const { t } = useTranslation();
  const enabled = useItemNotificationStore((state) => state.enabled);
  const setEnabled = useItemNotificationStore((state) => state.setEnabled);
  const setPermissionDenied = useItemNotificationStore((state) => state.setPermissionDenied);
  const baselineRef = useRef<ItemAnalysisEntry[] | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const translationRef = useRef(t);
  translationRef.current = t;

  const fetchSnapshot = useCallback(async () => {
    const value = await invoke<unknown>("fetch_item_inventory_snapshot");
    return normalizeItemInventorySnapshotResponse(value).items;
  }, []);

  const establishBaseline = useCallback(async () => {
    try {
      baselineRef.current = await fetchSnapshot();
    } catch {
      baselineRef.current = null;
    }
  }, [fetchSnapshot]);

  const inspectAfterBattle = useCallback(async () => {
    try {
      const current = await fetchSnapshot();
      const previous = baselineRef.current;
      baselineRef.current = current;
      if (previous === null) return;

      const acquired = findAcquiredWarningItems(previous, current, 900);
      if (acquired.length === 0) return;

      const translate = translationRef.current;
      sendNotification({
        title: translate("ui.item-analysis.notification.title"),
        body: formatAcquisitionNotificationBody(acquired, translateItemId, (count) =>
          translate("ui.item-analysis.notification.remaining", { count })
        ),
      });
    } catch {
      // Preserve the last valid baseline after read failures and stay silent.
    }
  }, [fetchSnapshot]);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;

    const clearPending = () => {
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    };

    if (!enabled) {
      clearPending();
      baselineRef.current = null;
      return () => {
        active = false;
      };
    }

    const start = async () => {
      let granted = false;
      try {
        granted = await hasItemNotificationPermission();
      } catch {
        granted = false;
      }
      if (!active) return;
      if (!granted) {
        baselineRef.current = null;
        setPermissionDenied(true);
        setEnabled(false);
        return;
      }

      setPermissionDenied(false);
      await establishBaseline();
      if (!active) return;

      const dispose = await listen("battle-ended", () => {
        clearPending();
        timerRef.current = setTimeout(() => {
          timerRef.current = null;
          void inspectAfterBattle();
        }, POST_BATTLE_DELAY_MS);
      });
      if (!active) {
        dispose();
        return;
      }
      unlisten = dispose;
    };

    void start();
    return () => {
      active = false;
      clearPending();
      unlisten?.();
      baselineRef.current = null;
    };
  }, [enabled, establishBaseline, inspectAfterBattle, setEnabled, setPermissionDenied]);
};

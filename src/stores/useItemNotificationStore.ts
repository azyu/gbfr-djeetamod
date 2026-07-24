import { create } from "zustand";
import { persist } from "zustand/middleware";

type ItemNotificationState = {
  enabled: boolean;
  permissionDenied: boolean;
  setEnabled: (enabled: boolean) => void;
  setPermissionDenied: (permissionDenied: boolean) => void;
};

export const useItemNotificationStore = create<ItemNotificationState>()(
  persist(
    (set) => ({
      enabled: false,
      permissionDenied: false,
      setEnabled: (enabled) => set({ enabled }),
      setPermissionDenied: (permissionDenied) => set({ permissionDenied }),
    }),
    {
      name: "item-notification-settings",
      partialize: (state) => ({ enabled: state.enabled }),
    }
  )
);

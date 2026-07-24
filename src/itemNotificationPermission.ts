import { isPermissionGranted, requestPermission } from "@tauri-apps/api/notification";

export const hasItemNotificationPermission = async (): Promise<boolean> => isPermissionGranted();

export const requestItemNotificationPermission = async (): Promise<boolean> => {
  if (await isPermissionGranted()) return true;
  return (await requestPermission()) === "granted";
};

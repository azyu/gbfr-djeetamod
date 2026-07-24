import { beforeEach, expect, it } from "vitest";

import { useItemNotificationStore } from "./useItemNotificationStore";

beforeEach(() => {
  localStorage.clear();
  useItemNotificationStore.persist.clearStorage();
  useItemNotificationStore.setState({
    enabled: false,
    permissionDenied: false,
  });
});

it("defaults off and persists only the enabled preference", () => {
  expect(useItemNotificationStore.getState().enabled).toBe(false);

  useItemNotificationStore.getState().setEnabled(true);

  const saved = JSON.parse(localStorage.getItem("item-notification-settings") ?? "{}") as {
    state?: Record<string, unknown>;
  };
  expect(saved.state).toEqual({ enabled: true });
  expect(saved.state).not.toHaveProperty("permissionDenied");
});

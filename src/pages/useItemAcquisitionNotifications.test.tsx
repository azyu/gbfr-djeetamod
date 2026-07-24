import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { useItemNotificationStore } from "@/stores/useItemNotificationStore";
import { useItemAcquisitionNotifications } from "./useItemAcquisitionNotifications";

const ITEM_A = 0x11111111;
const ITEM_B = 0x22222222;

const mocks = vi.hoisted(() => ({
  eventCallback: null as (() => void) | null,
  invoke: vi.fn(),
  isPermissionGranted: vi.fn(),
  sendNotification: vi.fn(),
  unlisten: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (_event: string, callback: () => void) => {
    mocks.eventCallback = callback;
    return mocks.unlisten;
  }),
}));

vi.mock("@tauri-apps/api/notification", () => ({
  isPermissionGranted: mocks.isPermissionGranted,
  requestPermission: vi.fn(),
  sendNotification: mocks.sendNotification,
}));

vi.mock("@tauri-apps/api/tauri", () => ({
  invoke: mocks.invoke,
}));

vi.mock("@/utils", () => ({
  translateItemId: (itemId: number) => (itemId === ITEM_B ? "아이템 B" : "아이템 A"),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: { count?: number }) =>
      key === "ui.item-analysis.notification.title"
        ? "Djeeta MOD · 아이템 분석"
        : key === "ui.item-analysis.notification.remaining"
          ? `외 ${options?.count ?? 0}개`
          : key,
  }),
}));

const snapshot = (items: Array<{ itemId: number; quantity: number }>) => ({
  inspectedAtMs: 123,
  items,
});

const flushMicrotasks = async () => {
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
  });
};

beforeEach(() => {
  vi.useFakeTimers();
  localStorage.clear();
  useItemNotificationStore.persist.clearStorage();
  useItemNotificationStore.setState({
    enabled: true,
    permissionDenied: false,
  });
  mocks.eventCallback = null;
  mocks.invoke.mockReset();
  mocks.isPermissionGranted.mockReset();
  mocks.isPermissionGranted.mockResolvedValue(true);
  mocks.sendNotification.mockReset();
  mocks.unlisten.mockReset();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

it("uses the first successful snapshot only as a baseline", async () => {
  mocks.invoke.mockResolvedValueOnce(snapshot([{ itemId: ITEM_A, quantity: 899 }]));

  renderHook(() => useItemAcquisitionNotifications());
  await flushMicrotasks();

  expect(mocks.invoke).toHaveBeenCalledTimes(1);
  expect(mocks.sendNotification).not.toHaveBeenCalled();
});

it("waits five seconds after battle-ended and sends one grouped notification", async () => {
  mocks.invoke
    .mockResolvedValueOnce(
      snapshot([
        { itemId: ITEM_A, quantity: 899 },
        { itemId: ITEM_B, quantity: 950 },
      ])
    )
    .mockResolvedValueOnce(
      snapshot([
        { itemId: ITEM_A, quantity: 900 },
        { itemId: ITEM_B, quantity: 953 },
      ])
    );
  renderHook(() => useItemAcquisitionNotifications());
  await flushMicrotasks();

  act(() => mocks.eventCallback?.());
  act(() => vi.advanceTimersByTime(4_999));
  expect(mocks.invoke).toHaveBeenCalledTimes(1);

  act(() => vi.advanceTimersByTime(1));
  await flushMicrotasks();

  expect(mocks.invoke).toHaveBeenCalledTimes(2);
  expect(mocks.sendNotification).toHaveBeenCalledTimes(1);
  expect(mocks.sendNotification).toHaveBeenCalledWith({
    title: "Djeeta MOD · 아이템 분석",
    body: "아이템 B 953 (+3) 외 1개",
  });
});

it("debounces repeated battle-ended events into one scan", async () => {
  mocks.invoke
    .mockResolvedValueOnce(snapshot([{ itemId: ITEM_A, quantity: 899 }]))
    .mockResolvedValueOnce(snapshot([{ itemId: ITEM_A, quantity: 900 }]));
  renderHook(() => useItemAcquisitionNotifications());
  await flushMicrotasks();

  act(() => mocks.eventCallback?.());
  act(() => vi.advanceTimersByTime(4_000));
  act(() => mocks.eventCallback?.());
  act(() => vi.advanceTimersByTime(4_999));
  expect(mocks.invoke).toHaveBeenCalledTimes(1);

  act(() => vi.advanceTimersByTime(1));
  await flushMicrotasks();
  expect(mocks.invoke).toHaveBeenCalledTimes(2);
});

it("keeps the previous baseline after a failed scan", async () => {
  mocks.invoke
    .mockResolvedValueOnce(snapshot([{ itemId: ITEM_A, quantity: 899 }]))
    .mockRejectedValueOnce("UNSTABLE")
    .mockResolvedValueOnce(snapshot([{ itemId: ITEM_A, quantity: 901 }]));
  renderHook(() => useItemAcquisitionNotifications());
  await flushMicrotasks();

  act(() => mocks.eventCallback?.());
  act(() => vi.advanceTimersByTime(5_000));
  await flushMicrotasks();
  expect(mocks.sendNotification).not.toHaveBeenCalled();

  act(() => mocks.eventCallback?.());
  act(() => vi.advanceTimersByTime(5_000));
  await flushMicrotasks();
  expect(mocks.sendNotification).toHaveBeenCalledWith({
    title: "Djeeta MOD · 아이템 분석",
    body: "아이템 A 901 (+2)",
  });
});

it("cancels the pending scan and clears the baseline when disabled", async () => {
  mocks.invoke.mockResolvedValueOnce(snapshot([{ itemId: ITEM_A, quantity: 899 }]));
  renderHook(() => useItemAcquisitionNotifications());
  await flushMicrotasks();

  act(() => mocks.eventCallback?.());
  act(() => useItemNotificationStore.getState().setEnabled(false));
  act(() => vi.advanceTimersByTime(5_000));

  expect(mocks.invoke).toHaveBeenCalledTimes(1);
  expect(mocks.sendNotification).not.toHaveBeenCalled();
});

it("disables a restored setting when notification permission is unavailable", async () => {
  mocks.isPermissionGranted.mockResolvedValueOnce(false);

  renderHook(() => useItemAcquisitionNotifications());
  await flushMicrotasks();

  expect(useItemNotificationStore.getState()).toMatchObject({
    enabled: false,
    permissionDenied: true,
  });
  expect(mocks.invoke).not.toHaveBeenCalled();
});

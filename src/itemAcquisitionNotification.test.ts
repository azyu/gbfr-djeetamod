import { expect, it } from "vitest";

import { findAcquiredWarningItems, formatAcquisitionNotificationBody } from "./itemAcquisitionNotification";

const entry = (itemId: number, quantity: number) => ({ itemId, quantity });

it("selects every increase whose resulting quantity is at least 900", () => {
  expect(
    findAcquiredWarningItems(
      [entry(1, 899), entry(2, 900), entry(3, 950), entry(4, 999)],
      [entry(1, 900), entry(2, 901), entry(3, 951), entry(4, 998)],
      900
    )
  ).toEqual([
    { itemId: 3, quantity: 951, increase: 1 },
    { itemId: 2, quantity: 901, increase: 1 },
    { itemId: 1, quantity: 900, increase: 1 },
  ]);
});

it("ignores unchanged, decreased, below-threshold, and baseline-missing results", () => {
  expect(
    findAcquiredWarningItems(
      [entry(1, 899), entry(2, 900), entry(3, 950)],
      [entry(1, 899), entry(2, 899), entry(3, 950), entry(4, 999)],
      900
    )
  ).toEqual([]);
});

it("formats one system notification body for all qualifying items", () => {
  const items = [
    { itemId: 1, quantity: 918, increase: 3 },
    { itemId: 2, quantity: 905, increase: 1 },
    { itemId: 3, quantity: 900, increase: 2 },
  ];

  expect(
    formatAcquisitionNotificationBody(
      items,
      (itemId) => (itemId === 1 ? "궁극의 증표" : String(itemId)),
      (remaining) => `외 ${remaining}개`
    )
  ).toBe("궁극의 증표 918 (+3) 외 2개");
});

it("omits the remaining count for a single item", () => {
  expect(
    formatAcquisitionNotificationBody(
      [{ itemId: 1, quantity: 900, increase: 1 }],
      () => "테스트 아이템",
      (remaining) => `외 ${remaining}개`
    )
  ).toBe("테스트 아이템 900 (+1)");
});

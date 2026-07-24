import { ItemAnalysisEntry } from "@/types";

export type AcquiredWarningItem = ItemAnalysisEntry & {
  increase: number;
};

export const findAcquiredWarningItems = (
  previous: ItemAnalysisEntry[],
  current: ItemAnalysisEntry[],
  threshold = 900
): AcquiredWarningItem[] => {
  const previousQuantities = new Map(previous.map((item) => [item.itemId, item.quantity]));
  return current
    .flatMap((item): AcquiredWarningItem[] => {
      const previousQuantity = previousQuantities.get(item.itemId);
      if (previousQuantity === undefined || item.quantity <= previousQuantity || item.quantity < threshold) {
        return [];
      }
      return [{ ...item, increase: item.quantity - previousQuantity }];
    })
    .sort((left, right) => right.quantity - left.quantity || left.itemId - right.itemId);
};

export const formatAcquisitionNotificationBody = (
  items: AcquiredWarningItem[],
  translateItem: (itemId: number) => string,
  formatRemaining: (remaining: number) => string
): string => {
  const first = items[0];
  if (!first) return "";
  const summary = `${translateItem(first.itemId)} ${first.quantity} (+${first.increase})`;
  return items.length === 1 ? summary : `${summary} ${formatRemaining(items.length - 1)}`;
};

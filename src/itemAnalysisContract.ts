import { ItemAnalysisEntry, ItemAnalysisResponse, ItemInventorySnapshotResponse } from "@/types";

export const normalizeItemAnalysisResponse = (value: unknown): ItemAnalysisResponse => {
  if (
    !isRecord(value) ||
    !isNonNegativeSafeInteger(value.inspectedAtMs) ||
    value.threshold !== 900 ||
    value.maximum !== 999 ||
    !Array.isArray(value.items)
  ) {
    throw new Error("invalid item analysis response");
  }

  return {
    inspectedAtMs: value.inspectedAtMs,
    threshold: 900,
    maximum: 999,
    items: normalizeItems(value.items, 900),
  };
};

export const normalizeItemInventorySnapshotResponse = (value: unknown): ItemInventorySnapshotResponse => {
  if (!isRecord(value) || !isNonNegativeSafeInteger(value.inspectedAtMs) || !Array.isArray(value.items)) {
    throw new Error("invalid item inventory snapshot response");
  }

  return {
    inspectedAtMs: value.inspectedAtMs,
    items: normalizeItems(value.items, 0),
  };
};

function normalizeItems(value: unknown[], minimumQuantity: number): ItemAnalysisEntry[] {
  const seen = new Set<number>();
  return value.flatMap((entry): ItemAnalysisEntry[] => {
    if (
      !isRecord(entry) ||
      !isUint32(entry.itemId) ||
      !isNonNegativeSafeInteger(entry.quantity) ||
      entry.quantity < minimumQuantity ||
      entry.quantity > 999 ||
      seen.has(entry.itemId)
    ) {
      return [];
    }
    seen.add(entry.itemId);
    return [{ itemId: entry.itemId, quantity: entry.quantity }];
  });
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isNonNegativeSafeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function isUint32(value: unknown): value is number {
  return isNonNegativeSafeInteger(value) && value <= 0xffff_ffff;
}

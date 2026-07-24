import { describe, expect, it } from "vitest";

import { normalizeItemAnalysisResponse } from "./itemAnalysisContract";

describe("normalizeItemAnalysisResponse", () => {
  it("keeps only valid warning entries and removes duplicate ids", () => {
    expect(
      normalizeItemAnalysisResponse({
        inspectedAtMs: 123,
        threshold: 900,
        maximum: 999,
        items: [
          { itemId: 0x2e94d39a, quantity: 918 },
          { itemId: 0x2e94d39a, quantity: 917 },
          { itemId: 0x687733c4, quantity: 899 },
          { itemId: -1, quantity: 999 },
        ],
      })
    ).toEqual({
      inspectedAtMs: 123,
      threshold: 900,
      maximum: 999,
      items: [{ itemId: 0x2e94d39a, quantity: 918 }],
    });
  });

  it("rejects an invalid top-level contract", () => {
    expect(() =>
      normalizeItemAnalysisResponse({
        inspectedAtMs: 123,
        threshold: 899,
        maximum: 999,
        items: [],
      })
    ).toThrow("invalid item analysis response");
  });
});

import { expect, it, vi } from "vitest";

import fixture from "@/fixtures/equipment-analysis-response.json";

import { normalizeEquipmentAnalysisResponse } from "./equipmentAnalysisContract";

it("preserves the shared equipment analysis contract", () => {
  expect(normalizeEquipmentAnalysisResponse(fixture as unknown)).toEqual(fixture);
  expect(normalizeEquipmentAnalysisResponse(null)).toEqual({ connected: false, characters: [] });
});

it("keeps a valid trait while dropping malformed sources", () => {
  const malformed = structuredClone(fixture) as unknown as {
    characters: Array<{ traits: Array<{ sources: unknown[] }> }>;
  };
  malformed.characters[0].traits[0].sources = [
    { kind: "sigilPrimary", slot: 0, item_id: 1, trait_id: 3696775008, trait_level: 15 },
    null,
  ];

  const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
  const normalized = normalizeEquipmentAnalysisResponse(malformed);

  expect(normalized.characters[0].traits[0].sources).toEqual([]);
  expect(warn).not.toHaveBeenCalled();
  warn.mockRestore();
});

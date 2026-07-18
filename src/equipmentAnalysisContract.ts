import {
  CharacterEquipmentAnalysis,
  CharacterEquipmentStatus,
  CharacterType,
  EquipmentAnalysisResponse,
  EquipmentSourceKind,
  EquippedTraitSource,
  TraitAnalysis,
  TraitAnalysisState,
} from "@/types";

const CHARACTER_STATUSES: readonly CharacterEquipmentStatus[] = ["complete", "unsupported"];
const TRAIT_STATES: readonly TraitAnalysisState[] = ["overflow", "capped", "below", "unknown"];
const SOURCE_KINDS: readonly EquipmentSourceKind[] = [
  "sigilPrimary",
  "sigilSecondary",
  "weapon",
  "wrightstone",
  "masterTrait",
  "summon",
];

export const normalizeEquipmentAnalysisResponse = (value: unknown): EquipmentAnalysisResponse => {
  if (!isRecord(value)) return { connected: false, characters: [] };
  return {
    connected: value.connected === true,
    characters: Array.isArray(value.characters) ? value.characters.flatMap(normalizeCharacter) : [],
  };
};

function normalizeCharacter(value: unknown): CharacterEquipmentAnalysis[] {
  if (!isRecord(value) || !isCharacterType(value.characterType) || !isOneOf(value.status, CHARACTER_STATUSES)) {
    return [];
  }
  return [
    {
      characterType: value.characterType,
      status: value.status,
      traits: Array.isArray(value.traits) ? value.traits.flatMap(normalizeTrait) : [],
    },
  ];
}

function normalizeTrait(value: unknown): TraitAnalysis[] {
  if (
    !isRecord(value) ||
    !isNonNegativeInteger(value.traitId) ||
    !isNonNegativeInteger(value.totalLevel) ||
    !(value.maxLevel === null || isNonNegativeInteger(value.maxLevel)) ||
    !isNonNegativeInteger(value.overflowLevel) ||
    !isOneOf(value.state, TRAIT_STATES)
  ) {
    return [];
  }
  return [
    {
      traitId: value.traitId,
      totalLevel: value.totalLevel,
      maxLevel: value.maxLevel,
      overflowLevel: value.overflowLevel,
      state: value.state,
      sources: Array.isArray(value.sources) ? value.sources.flatMap(normalizeSource) : [],
    },
  ];
}

function normalizeSource(value: unknown): EquippedTraitSource[] {
  if (
    !isRecord(value) ||
    !isOneOf(value.kind, SOURCE_KINDS) ||
    !isNonNegativeInteger(value.slot) ||
    !isNonNegativeInteger(value.itemId) ||
    !isNonNegativeInteger(value.traitId) ||
    !isNonNegativeInteger(value.traitLevel)
  ) {
    return [];
  }
  return [
    {
      kind: value.kind,
      slot: value.slot,
      itemId: value.itemId,
      traitId: value.traitId,
      traitLevel: value.traitLevel,
    },
  ];
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isNonNegativeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}

function isCharacterType(value: unknown): value is CharacterType {
  return (
    (typeof value === "string" && value.length > 0) ||
    (isRecord(value) && isNonNegativeInteger(value.Unknown))
  );
}

function isOneOf<T extends string>(value: unknown, allowed: readonly T[]): value is T {
  return typeof value === "string" && allowed.includes(value as T);
}

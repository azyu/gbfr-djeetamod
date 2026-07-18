import { CharacterType, EncounterState } from "@/types";

export type CompactMeterRow = {
  actorIndex: number;
  characterType: CharacterType;
  totalDamage: number;
  dps: number;
  barPercent: number;
};

export const buildCompactMeterRows = (encounter: EncounterState): CompactMeterRow[] => {
  const players = Object.values(encounter.party)
    .sort((left, right) => right.totalDamage - left.totalDamage || left.index - right.index)
    .slice(0, 4);
  const highestDamage = Math.max(0, ...players.map((player) => player.totalDamage));

  return players.map((player) => ({
    actorIndex: player.index,
    characterType: player.characterType,
    totalDamage: player.totalDamage,
    dps: Math.round(player.dps),
    barPercent: highestDamage === 0 ? 0 : (player.totalDamage / highestDamage) * 100,
  }));
};

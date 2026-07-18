import { describe, expect, it } from "vitest";

import { EncounterState, PlayerState } from "@/types";

import { buildCompactMeterRows } from "./compactMeterModel";

const player = (index: number, totalDamage: number, dps: number): PlayerState => ({
  index,
  characterType: `Pl${index.toString().padStart(4, "0")}`,
  totalDamage,
  dps,
  sba: 0,
  totalStunValue: 0,
  stunPerSecond: 0,
  lastDamageTime: 0,
  skillBreakdown: [],
});

const encounter = (players: PlayerState[]): EncounterState => ({
  totalDamage: players.reduce((sum, item) => sum + item.totalDamage, 0),
  dps: players.reduce((sum, item) => sum + item.dps, 0),
  startTime: 1_000,
  endTime: 5_000,
  party: Object.fromEntries(players.map((item) => [item.index, item])),
  targets: {},
  status: "InProgress",
});

describe("buildCompactMeterRows", () => {
  it("sorts descending, limits four rows, and makes the leader 100 percent", () => {
    const rows = buildCompactMeterRows(
      encounter([player(1, 100, 25), player(2, 400, 100), player(3, 200, 50), player(4, 300, 75), player(5, 50, 12.5)])
    );

    expect(rows.map((row) => row.actorIndex)).toEqual([2, 4, 3, 1]);
    expect(rows.map((row) => row.barPercent)).toEqual([100, 75, 50, 25]);
  });

  it("returns finite zero-width bars when every total is zero", () => {
    const rows = buildCompactMeterRows(encounter([player(1, 0, 0), player(2, 0, 0)]));

    expect(rows.every((row) => row.barPercent === 0)).toBe(true);
    expect(rows.every((row) => Number.isFinite(row.barPercent))).toBe(true);
  });
});

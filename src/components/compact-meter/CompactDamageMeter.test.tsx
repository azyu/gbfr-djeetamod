import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";

import { CompactDamageMeter } from "./CompactDamageMeter";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) =>
      ({
        "ui.compact-meter.title": "파티 데미지",
        "characters:Pl1400": "나루메아",
        "ui.compact-meter.unknown-character": "알 수 없는 캐릭터",
      })[key] ?? key,
  }),
}));

afterEach(cleanup);

it("shows Korean character names, full totals, DPS, and relative bars", () => {
  const { container } = render(
    <CompactDamageMeter
      transparency={0.72}
      rows={[
        {
          actorIndex: 1,
          characterType: "Pl1400",
          totalDamage: 1_234_567,
          dps: 12_345,
          barPercent: 100,
        },
        {
          actorIndex: 2,
          characterType: { Unknown: 1 },
          totalDamage: 617_284,
          dps: 6_172,
          barPercent: 50,
        },
      ]}
    />
  );

  expect(screen.getByText("파티 데미지")).toBeTruthy();
  expect(screen.getByText("나루메아")).toBeTruthy();
  expect(screen.getByText("알 수 없는 캐릭터")).toBeTruthy();
  expect(screen.getByText("1,234,567")).toBeTruthy();
  expect(screen.getByText("12,345 DPS")).toBeTruthy();
  expect(container.querySelectorAll<HTMLElement>(".compact-meter__bar")[1].style.width).toBe("50%");
});

it("keeps a draggable header visible while waiting for combat", () => {
  const { container } = render(<CompactDamageMeter transparency={0.2} rows={[]} />);

  expect(screen.getByText("파티 데미지")).toBeTruthy();
  expect(container.querySelector(".compact-meter__header")?.hasAttribute("data-tauri-drag-region")).toBe(true);
});

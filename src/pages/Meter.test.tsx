import { render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";

import { Meter } from "./Meter";

vi.mock("react-i18next", async (importOriginal) => {
  const actual = await importOriginal<typeof import("react-i18next")>();
  return {
    ...actual,
    useTranslation: () => ({
      t: (key: string) => (key === "ui.compact-meter.title" ? "파티 데미지" : key),
    }),
  };
});

vi.mock("./useCompactMeter", () => ({
  default: () => ({ rows: [], transparency: 0.5 }),
}));

it("keeps the draggable meter header visible while waiting", () => {
  render(<Meter />);

  expect(screen.getByText("파티 데미지")).toBeTruthy();
});

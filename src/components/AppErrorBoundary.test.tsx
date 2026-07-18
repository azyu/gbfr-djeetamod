import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";

import { AppErrorBoundary } from "./AppErrorBoundary";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

it("shows a reloadable fallback when rendering fails", () => {
  vi.spyOn(console, "error").mockImplementation(() => undefined);
  const reload = vi.fn();
  const Broken = () => {
    throw new Error("render failed");
  };

  render(
    <AppErrorBoundary onReload={reload}>
      <Broken />
    </AppErrorBoundary>
  );

  expect(screen.getByText("Djeeta MOD")).toBeTruthy();
  expect(screen.getByText("화면을 표시할 수 없습니다")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "다시 불러오기" }));
  expect(reload).toHaveBeenCalledTimes(1);
});

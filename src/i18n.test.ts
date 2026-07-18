import { beforeEach, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/fs", () => ({ readTextFile: vi.fn(async () => "{}") }));
vi.mock("@tauri-apps/api/path", () => ({ resolveResource: vi.fn(async (path: string) => path) }));

beforeEach(() => {
  localStorage.clear();
  vi.resetModules();
});

it("uses Korean for a fresh profile", async () => {
  const { default: i18n, SUPPORTED_LANGUAGES } = await import("./i18n");

  await vi.waitFor(() => expect(i18n.language).toBe("ko"));
  expect(SUPPORTED_LANGUAGES.ko).toBe("한국어");
});

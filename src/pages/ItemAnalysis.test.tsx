import { MantineProvider } from "@mantine/core";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { useItemNotificationStore } from "@/stores/useItemNotificationStore";
import { ItemAnalysis } from "./ItemAnalysis";

const mocks = vi.hoisted(() => ({
  responses: [] as Array<unknown>,
  errors: [] as string[],
  pending: null as Promise<unknown> | null,
}));

const invoke = vi.hoisted(() =>
  vi.fn(async (command: string) => {
    if (command !== "fetch_item_analysis") throw new Error(`unexpected command: ${command}`);
    if (mocks.pending) return mocks.pending;
    const error = mocks.errors.shift();
    if (error) throw error;
    return mocks.responses.shift();
  })
);

const notification = vi.hoisted(() => ({
  isPermissionGranted: vi.fn(async () => false),
  requestPermission: vi.fn(async (): Promise<"granted" | "denied"> => "denied"),
}));

vi.mock("@tauri-apps/api", () => ({ invoke }));
vi.mock("@tauri-apps/api/notification", () => notification);
vi.mock("@/utils", () => ({
  translateItemId: (id: number) =>
    id === 0x2e94d39a ? "궁극의 증표" : `알 수 없는 아이템 (0x${id.toString(16).padStart(8, "0")})`,
}));
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) =>
      ({
        "ui.item-analysis.title": "아이템 분석",
        "ui.item-analysis.description": "보유 한도 999개에 가까운 일반 아이템만 표시합니다.",
        "ui.item-analysis.refresh": "새로고침",
        "ui.item-analysis.loading": "아이템 정보를 확인하는 중입니다.",
        "ui.item-analysis.empty": "보유 한도에 가까운 아이템이 없습니다.",
        "ui.item-analysis.quantity": "수량",
        "ui.item-analysis.tabs.inventory": "보유 현황",
        "ui.item-analysis.tabs.notifications": "알림 설정",
        "ui.item-analysis.notification.label": "아이템 획득 시, 900개 이상일 경우 알림",
        "ui.item-analysis.notification.description":
          "전투 종료 후 증가한 일반 아이템을 확인해 Windows 알림으로 알려줍니다.",
        "ui.item-analysis.notification.permission-denied": "Windows 알림 권한을 허용해야 사용할 수 있습니다.",
        "ui.item-analysis.error.UNSTABLE": "아이템 정보가 변경되어 다시 읽어야 합니다.",
        "ui.item-analysis.error.INTERNAL": "아이템 정보를 확인하지 못했습니다.",
      })[key] ?? key,
  }),
}));

beforeEach(() => {
  globalThis.ResizeObserver = class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
  window.matchMedia = vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }));
  mocks.responses = [];
  mocks.errors = [];
  mocks.pending = null;
  invoke.mockClear();
  notification.isPermissionGranted.mockReset();
  notification.isPermissionGranted.mockResolvedValue(false);
  notification.requestPermission.mockReset();
  notification.requestPermission.mockResolvedValue("denied");
  localStorage.clear();
  useItemNotificationStore.persist.clearStorage();
  useItemNotificationStore.setState({
    enabled: false,
    permissionDenied: false,
  });
});

afterEach(cleanup);

const response = (quantity = 918) => ({
  inspectedAtMs: 123,
  threshold: 900,
  maximum: 999,
  items: [{ itemId: 0x2e94d39a, quantity }],
});

function renderPage() {
  return render(
    <MantineProvider>
      <ItemAnalysis />
    </MantineProvider>
  );
}

it("loads once on mount and renders warning quantities", async () => {
  mocks.responses.push(response());
  renderPage();

  expect(await screen.findByText("궁극의 증표")).toBeTruthy();
  expect(screen.getByText("918 / 999")).toBeTruthy();
  expect(invoke).toHaveBeenCalledTimes(1);
});

it("keeps the last successful rows when refresh fails", async () => {
  mocks.responses.push(response());
  renderPage();
  expect(await screen.findByText("918 / 999")).toBeTruthy();

  mocks.errors.push("UNSTABLE");
  fireEvent.click(screen.getByRole("button", { name: "새로고침" }));

  const alert = await screen.findByRole("alert");
  expect(within(alert).getByText("아이템 정보가 변경되어 다시 읽어야 합니다.")).toBeTruthy();
  expect(screen.getByText("918 / 999")).toBeTruthy();
});

it("disables refresh while a request is running", async () => {
  let resolve!: (value: unknown) => void;
  mocks.pending = new Promise((done) => {
    resolve = done;
  });
  renderPage();

  const button = screen.getByRole("button", { name: "새로고침" });
  expect((button as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(button);
  expect(invoke).toHaveBeenCalledTimes(1);

  await act(async () => resolve(response()));
  expect(await screen.findByText("918 / 999")).toBeTruthy();
});

it("keeps the notification switch off when permission is denied", async () => {
  mocks.responses.push(response());
  renderPage();

  fireEvent.click(await screen.findByRole("tab", { name: "알림 설정" }));
  const toggle = screen.getByRole("switch", { name: /^아이템 획득 시, 900개 이상일 경우 알림/ });
  fireEvent.click(toggle);

  expect((await screen.findByRole("alert")).textContent).toContain("Windows 알림 권한을 허용해야 사용할 수 있습니다.");
  expect((toggle as HTMLInputElement).checked).toBe(false);
});

it("persists the enabled setting only after permission is granted", async () => {
  notification.requestPermission.mockResolvedValueOnce("granted");
  mocks.responses.push(response());
  renderPage();

  fireEvent.click(await screen.findByRole("tab", { name: "알림 설정" }));
  const toggle = screen.getByRole("switch", { name: /^아이템 획득 시, 900개 이상일 경우 알림/ });
  fireEvent.click(toggle);

  await waitFor(() => expect((toggle as HTMLInputElement).checked).toBe(true));
  expect(useItemNotificationStore.getState().enabled).toBe(true);
});

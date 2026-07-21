import { useMeterSettingsStore } from "@/stores/useMeterSettingsStore";
import "./Logs.css";

import { AppShell, Burger, Group, NavLink, ScrollArea, Switch, Text } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { ChartBar, Gauge, Gear, House } from "@phosphor-icons/react";
import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { Toaster } from "react-hot-toast";
import { useTranslation } from "react-i18next";
import { Link, Outlet, useNavigate } from "react-router-dom";
import useMeterVisibility from "./useMeterVisibility";
import useRepeatQuest from "./useRepeatQuest";

const Layout = () => {
  const { t } = useTranslation();
  const [mobileOpened, { toggle: toggleMobile }] = useDisclosure();
  const [desktopOpened, { toggle: toggleDesktop }] = useDisclosure(true);
  const { open_log_on_save } = useMeterSettingsStore((state) => ({ open_log_on_save: state.open_log_on_save }));
  const { meterEnabled, setMeterEnabled } = useMeterVisibility();
  const repeatQuest = useRepeatQuest();

  const navigate = useNavigate();

  useEffect(() => {
    const debugListener = listen("debug-event", (event: { payload: unknown }) => {
      console.info(JSON.stringify(event.payload));
    });

    const saveListener = listen("encounter-saved", (event: { payload: number | null }) => {
      if (event.payload && open_log_on_save) {
        navigate(`/logs/${event.payload}`);
      }
    });

    return () => {
      debugListener.then((f) => f());
      saveListener.then((f) => f());
    };
  }, [open_log_on_save]);

  return (
    <div className="log-window">
      <AppShell
        header={{ height: 50 }}
        navbar={{
          width: 300,
          breakpoint: "sm",
          collapsed: { mobile: !mobileOpened, desktop: !desktopOpened },
        }}
        padding="sm"
      >
        <AppShell.Header>
          <Group h="100%" px="sm">
            <Burger opened={mobileOpened} onClick={toggleMobile} hiddenFrom="sm" size="sm" />
            <Burger opened={desktopOpened} onClick={toggleDesktop} visibleFrom="sm" size="sm" />
            <Text>Djeeta MOD</Text>
          </Group>
        </AppShell.Header>
        <AppShell.Navbar p="sm">
          <AppShell.Section grow component={ScrollArea}>
            <NavLink
              label={t("ui.navigation.damage-meter")}
              leftSection={<Gauge size="1rem" />}
              rightSection={
                <Switch
                  aria-label={t("ui.navigation.damage-meter")}
                  checked={meterEnabled}
                  onClick={(event) => event.stopPropagation()}
                  onChange={(event) => void setMeterEnabled(event.currentTarget.checked).catch(() => undefined)}
                />
              }
              onClick={() => void setMeterEnabled(!meterEnabled).catch(() => undefined)}
            />
            <NavLink
              label={t("ui.game-features.repeat-quest.label")}
              rightSection={
                <Switch
                  aria-label={t("ui.game-features.repeat-quest.label")}
                  checked={repeatQuest.status?.state === "on"}
                  disabled={
                    repeatQuest.pending || repeatQuest.status === null || repeatQuest.status.state === "unavailable"
                  }
                  onClick={(event) => event.stopPropagation()}
                  onChange={(event) => void repeatQuest.setEnabled(event.currentTarget.checked)}
                />
              }
            />
            {repeatQuest.status?.reason && (
              <Text size="xs" c="red" px="sm" pb="xs">
                {t(`ui.game-features.repeat-quest.reason.${repeatQuest.status.reason}`)}
              </Text>
            )}
            <NavLink
              label={t("ui.equipment-analysis.title")}
              leftSection={<ChartBar size="1rem" />}
              component={Link}
              to="/logs/equipment"
            />
            <NavLink
              label={t("ui.navigation.battle-records")}
              leftSection={<House size="1rem" />}
              component={Link}
              to="/logs"
            />
          </AppShell.Section>
          <AppShell.Section>
            <NavLink
              label={t("ui.navigation.settings")}
              leftSection={<Gear size="1rem" />}
              component={Link}
              to="/logs/settings"
            />
          </AppShell.Section>
        </AppShell.Navbar>
        <AppShell.Main className="log-main">
          <Outlet />
        </AppShell.Main>
      </AppShell>
      <Toaster
        position="bottom-center"
        toastOptions={{
          style: {
            borderRadius: "10px",
            backgroundColor: "#252525",
            color: "#fff",
            fontSize: "14px",
          },
        }}
      />
    </div>
  );
};

export default Layout;

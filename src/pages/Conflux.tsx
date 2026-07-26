import { Alert, Paper, Stack, Switch, Text, Title } from "@mantine/core";
import { Info } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import useConfluxTimer from "./useConfluxTimer";

export const Conflux = () => {
  const { t } = useTranslation();
  const timer = useConfluxTimer();
  const unavailable = timer.status?.state === "unavailable";

  return (
    <Stack maw={720}>
      <div>
        <Title order={2}>{t("ui.game-features.conflux.title")}</Title>
        <Text c="dimmed">{t("ui.game-features.conflux.description")}</Text>
      </div>

      <Alert icon={<Info size="1rem" />} title={t("ui.game-features.conflux.requirement-title")}>
        {t("ui.game-features.conflux.requirement")}
      </Alert>

      <Paper withBorder p="md">
        <Switch
          label={t("ui.game-features.conflux.auto-run")}
          description={t("ui.game-features.conflux.timer-description")}
          checked={timer.status?.state === "on"}
          disabled={timer.pending || timer.status === null || unavailable}
          onChange={(event) => void timer.setEnabled(event.currentTarget.checked)}
        />
        {timer.status?.reason && timer.status.reason !== "gameNotRunning" && (
          <Text size="sm" c="red" mt="sm">
            {t(`ui.game-features.conflux.reason.${timer.status.reason}`)}
          </Text>
        )}
      </Paper>

      <Text size="sm" c="dimmed">
        {t("ui.game-features.conflux.todo")}
      </Text>
    </Stack>
  );
};

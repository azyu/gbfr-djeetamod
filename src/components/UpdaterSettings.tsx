import { Button, Fieldset, Stack, Text } from "@mantine/core";
import { useTranslation } from "react-i18next";

import { useUpdater, type UpdaterError } from "@/pages/useUpdater";

const errorTranslationKeys: Record<UpdaterError, string> = {
  checkFailed: "check-failed",
  gameRunning: "game-running",
  repeatQuestRestoreFailed: "repeat-quest-restore-failed",
  installFailed: "install-failed",
};

export const UpdaterSettings = () => {
  const { t } = useTranslation();
  const { state, checkForUpdate } = useUpdater();
  const pending = state.phase === "checking" || state.phase === "preparing" || state.phase === "installing";

  let status: string | null = null;
  if (state.phase === "checking" || state.phase === "preparing" || state.phase === "installing") {
    status = t(`ui.updater.${state.phase}`);
  } else if (state.phase === "upToDate") {
    status = t("ui.updater.up-to-date");
  } else if (state.phase === "available" && state.manifest) {
    status = t("ui.updater.available", { version: state.manifest.version });
  } else if (state.phase === "error" && state.error) {
    status = t(`ui.updater.${errorTranslationKeys[state.error]}`);
  }

  return (
    <Fieldset legend={t("ui.updater.title")}>
      <Stack>
        <Text size="sm">{t("ui.updater.current", { version: state.currentVersion })}</Text>
        {status && <Text size="sm">{status}</Text>}
        <Button disabled={pending} loading={state.phase === "checking"} onClick={() => void checkForUpdate("manual")}>
          {t("ui.updater.check")}
        </Button>
      </Stack>
    </Fieldset>
  );
};

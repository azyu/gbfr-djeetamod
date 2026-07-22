import { Button, Group, Modal, Stack, Text } from "@mantine/core";
import { useTranslation } from "react-i18next";

import { useUpdater, type UpdaterError } from "@/pages/useUpdater";

const blockingErrorTranslationKeys: Partial<Record<UpdaterError, string>> = {
  gameRunning: "game-running",
  repeatQuestRestoreFailed: "repeat-quest-restore-failed",
  installFailed: "install-failed",
};

export const UpdaterDialog = () => {
  const { t } = useTranslation();
  const { state, dismissUpdate, installAvailableUpdate } = useUpdater();
  const errorKey = state.error ? blockingErrorTranslationKeys[state.error] : undefined;
  const pending = state.phase === "preparing" || state.phase === "installing";
  const opened = state.phase === "available" || pending || (state.phase === "error" && Boolean(errorKey));
  const releaseNotes = state.manifest?.body?.trim();

  let message: string | null = null;
  if (state.phase === "available" && state.manifest) {
    message = t("ui.updater.available", { version: state.manifest.version });
  } else if (state.phase === "preparing" || state.phase === "installing") {
    message = t(`ui.updater.${state.phase}`);
  } else if (state.phase === "error" && errorKey) {
    message = t(`ui.updater.${errorKey}`);
  }

  return (
    <Modal
      opened={opened}
      onClose={dismissUpdate}
      title={t("ui.updater.title")}
      closeOnClickOutside={!pending}
      closeOnEscape={!pending}
      withCloseButton={!pending}
    >
      <Stack>
        {message && <Text size="sm">{message}</Text>}
        {state.phase === "available" && releaseNotes && (
          <Stack gap="xs">
            <Text fw={600} size="sm">
              {t("ui.updater.notes")}
            </Text>
            <Text size="sm" style={{ whiteSpace: "pre-wrap" }}>
              {releaseNotes}
            </Text>
          </Stack>
        )}
        <Group justify="flex-end">
          <Button variant="default" disabled={pending} onClick={dismissUpdate}>
            {t("ui.updater.later")}
          </Button>
          <Button loading={pending} onClick={() => void installAvailableUpdate()}>
            {t("ui.updater.install")}
          </Button>
        </Group>
      </Stack>
    </Modal>
  );
};

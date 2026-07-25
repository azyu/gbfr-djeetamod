import { Button, Group, Modal, Progress, Stack, Text } from "@mantine/core";
import { open } from "@tauri-apps/api/shell";
import { useTranslation } from "react-i18next";

import { useUpdater, type UpdaterError } from "@/pages/useUpdater";
const MANUAL_UPDATE_URL = "https://github.com/azyu/gbfr-djeetamod/releases/latest";

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
  const progress =
    state.downloadProgress?.totalBytes && state.downloadProgress.totalBytes > 0
      ? Math.min((state.downloadProgress.downloadedBytes / state.downloadProgress.totalBytes) * 100, 100)
      : null;

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
        {state.phase === "installing" && progress !== null && (
          <Stack gap={4}>
            <Progress value={progress} aria-label={t("ui.updater.download-progress-label")} />
            <Text c="dimmed" size="xs">
              {t("ui.updater.download-progress", { percent: Math.floor(progress) })}
            </Text>
          </Stack>
        )}
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
          {state.phase === "error" && state.error === "installFailed" && (
            <Button variant="default" onClick={() => void open(MANUAL_UPDATE_URL)}>
              {t("ui.updater.manual-install")}
            </Button>
          )}
          <Button variant="default" disabled={pending} onClick={dismissUpdate}>
            {t("ui.updater.later")}
          </Button>
          <Button loading={pending} onClick={() => void installAvailableUpdate()}>
            {t(state.phase === "error" ? "ui.updater.retry" : "ui.updater.install")}
          </Button>
        </Group>
      </Stack>
    </Modal>
  );
};

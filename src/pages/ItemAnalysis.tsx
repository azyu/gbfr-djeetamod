import { Alert, Button, Group, Stack, Table, Text, Title } from "@mantine/core";
import { invoke } from "@tauri-apps/api";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { normalizeItemAnalysisResponse } from "@/itemAnalysisContract";
import { ItemAnalysisResponse } from "@/types";
import { translateItemId } from "@/utils";

const ERROR_CODES = new Set([
  "ALREADY_RUNNING",
  "GAME_NOT_RUNNING",
  "UNSUPPORTED_GAME",
  "UNAVAILABLE",
  "UNSTABLE",
  "INTERNAL",
]);

export const ItemAnalysis = () => {
  const { t } = useTranslation();
  const [response, setResponse] = useState<ItemAnalysisResponse | null>(null);
  const [pending, setPending] = useState(true);
  const [errorCode, setErrorCode] = useState<string | null>(null);
  const runningRef = useRef(false);

  const refresh = useCallback(async () => {
    if (runningRef.current) return;
    runningRef.current = true;
    setPending(true);
    setErrorCode(null);
    try {
      const value = await invoke<unknown>("fetch_item_analysis");
      setResponse(normalizeItemAnalysisResponse(value));
    } catch (error) {
      setErrorCode(typeof error === "string" && ERROR_CODES.has(error) ? error : "INTERNAL");
    } finally {
      runningRef.current = false;
      setPending(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const items = useMemo(
    () =>
      [...(response?.items ?? [])].sort(
        (left, right) =>
          right.quantity - left.quantity || translateItemId(left.itemId).localeCompare(translateItemId(right.itemId))
      ),
    [response]
  );

  return (
    <Stack gap="md">
      <Group justify="space-between" align="flex-start">
        <div>
          <Title order={2}>{t("ui.item-analysis.title")}</Title>
          <Text c="dimmed" size="sm">
            {t("ui.item-analysis.description")}
          </Text>
        </div>
        <Button disabled={pending} onClick={() => void refresh()}>
          {t("ui.item-analysis.refresh")}
        </Button>
      </Group>

      {errorCode ? (
        <Alert role="alert" color="red">
          {t(`ui.item-analysis.error.${errorCode}`)}
        </Alert>
      ) : null}

      {response === null && pending ? (
        <Text c="dimmed">{t("ui.item-analysis.loading")}</Text>
      ) : response !== null && items.length === 0 ? (
        <Text c="dimmed">{t("ui.item-analysis.empty")}</Text>
      ) : items.length > 0 ? (
        <Table striped highlightOnHover>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>{t("ui.item-analysis.title")}</Table.Th>
              <Table.Th>{t("ui.item-analysis.quantity")}</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {items.map((item) => (
              <Table.Tr key={item.itemId} data-testid="item-row">
                <Table.Td>{translateItemId(item.itemId)}</Table.Td>
                <Table.Td>
                  {item.quantity} / {response?.maximum ?? 999}
                </Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      ) : null}
    </Stack>
  );
};

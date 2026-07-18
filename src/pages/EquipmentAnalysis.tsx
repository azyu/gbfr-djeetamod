import { Badge, Box, Center, Select, Stack, Table, Text, Title } from "@mantine/core";
import { invoke } from "@tauri-apps/api";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useShallow } from "zustand/react/shallow";

import { characterTypeKey, useEquipmentAnalysisStore } from "@/stores/useEquipmentAnalysisStore";
import {
  CharacterEquipmentAnalysis,
  CharacterType,
  EquipmentAnalysisResponse,
  EquipmentSourceKind,
  EquippedTraitSource,
  TraitAnalysis,
  TraitAnalysisState,
} from "@/types";
import { translateTraitId } from "@/utils";

const STATE_ORDER: Record<TraitAnalysisState, number> = {
  overflow: 0,
  capped: 1,
  below: 2,
  unknown: 3,
};

export const EquipmentAnalysis = () => {
  const { t } = useTranslation();
  const { response, selectedCharacter, loadResponse, selectCharacter } = useEquipmentAnalysisStore(
    useShallow((state) => ({
      response: state.response,
      selectedCharacter: state.selectedCharacter,
      loadResponse: state.loadResponse,
      selectCharacter: state.selectCharacter,
    }))
  );

  useEffect(() => {
    invoke<EquipmentAnalysisResponse>("fetch_equipment_analysis").then(loadResponse);
    const listener = listen<EquipmentAnalysisResponse>("equipment-analysis-update", (event) =>
      loadResponse(event.payload)
    );
    return () => {
      listener.then((unlisten) => unlisten());
    };
  }, [loadResponse]);

  const selectedKey = selectedCharacter ? characterTypeKey(selectedCharacter) : null;
  const selected = response.characters.find((character) => characterTypeKey(character.characterType) === selectedKey);
  const options = response.characters.map((character) => ({
    value: characterTypeKey(character.characterType),
    label: characterLabel(character.characterType, t),
  }));

  return (
    <Stack gap="md">
      <Title order={2}>{t("ui.equipment-analysis.title")}</Title>
      <Text c="dimmed" size="sm">
        {t("ui.equipment-analysis.scope")}
      </Text>
      {!response.connected ? (
        <StatusMessage>{t("ui.equipment-analysis.waiting-game")}</StatusMessage>
      ) : response.characters.length === 0 ? (
        <StatusMessage>{t("ui.equipment-analysis.waiting-equipment")}</StatusMessage>
      ) : (
        <>
          <Select
            label={t("ui.equipment-analysis.select-character")}
            data={options}
            value={selectedKey}
            onChange={(value) => {
              const character = response.characters.find(
                (candidate) => characterTypeKey(candidate.characterType) === value
              );
              if (character) selectCharacter(character.characterType);
            }}
          />
          {selected?.status === "unsupported" ? (
            <StatusMessage>{t("ui.equipment-analysis.unsupported")}</StatusMessage>
          ) : selected ? (
            <TraitTable character={selected} />
          ) : null}
        </>
      )}
    </Stack>
  );
};

const TraitTable = ({ character }: { character: CharacterEquipmentAnalysis }) => {
  const { t } = useTranslation();
  const traits = useMemo(
    () =>
      [...character.traits].sort(
        (left, right) => STATE_ORDER[left.state] - STATE_ORDER[right.state] || left.traitId - right.traitId
      ),
    [character.traits]
  );

  return (
    <Table striped highlightOnHover>
      <Table.Thead>
        <Table.Tr>
          <Table.Th>{t("ui.equipment-analysis.trait")}</Table.Th>
          <Table.Th>{t("ui.equipment-analysis.level")}</Table.Th>
          <Table.Th>{t("ui.equipment-analysis.status")}</Table.Th>
          <Table.Th>{t("ui.equipment-analysis.sources")}</Table.Th>
        </Table.Tr>
      </Table.Thead>
      <Table.Tbody>
        {traits.map((trait) => (
          <TraitRow key={trait.traitId} trait={trait} />
        ))}
      </Table.Tbody>
    </Table>
  );
};

const TraitRow = ({ trait }: { trait: TraitAnalysis }) => {
  const { t } = useTranslation();
  const stateText =
    trait.state === "overflow"
      ? `${trait.overflowLevel} ${t("ui.equipment-analysis.overflow")}`
      : t(`ui.equipment-analysis.${trait.state}`);
  const color =
    trait.state === "overflow"
      ? "red"
      : trait.state === "capped"
        ? "yellow"
        : trait.state === "below"
          ? "green"
          : "gray";

  return (
    <Table.Tr data-testid="trait-row">
      <Table.Td>{translateTraitId(trait.traitId)}</Table.Td>
      <Table.Td>
        {trait.totalLevel} / {trait.maxLevel ?? "—"}
      </Table.Td>
      <Table.Td>
        <Badge color={color} variant="light">
          {stateText}
        </Badge>
      </Table.Td>
      <Table.Td>
        {trait.sources.length > 0 ? (
          <details>
            <summary>{t("ui.equipment-analysis.sources")}</summary>
            <Stack gap={2} mt="xs">
              {trait.sources.map((source, index) => (
                <Text size="xs" key={`${source.kind}-${source.slot}-${index}`}>
                  {sourceKindLabel(source.kind, t)} #{source.slot + 1} · 0x
                  {formatItemId(source)} · +{sourceTraitLevel(source) ?? "—"}
                </Text>
              ))}
            </Stack>
          </details>
        ) : (
          "—"
        )}
      </Table.Td>
    </Table.Tr>
  );
};

const StatusMessage = ({ children }: { children: React.ReactNode }) => (
  <Box py="xl">
    <Center>
      <Text c="dimmed">{children}</Text>
    </Center>
  </Box>
);

function characterLabel(characterType: CharacterType, t: (key: string) => string): string {
  if (typeof characterType === "string") return t(`characters:${characterType}`);
  return `0x${characterType.Unknown.toString(16).padStart(8, "0")}`;
}

function sourceKindLabel(kind: EquipmentSourceKind, t: (key: string) => string): string {
  return t(`ui.equipment-analysis.source.${kind}`);
}

type LegacyEquippedTraitSource = EquippedTraitSource & {
  item_id?: number;
  trait_level?: number;
};

function formatItemId(source: EquippedTraitSource): string {
  const legacySource = source as LegacyEquippedTraitSource;
  const itemId = source.itemId ?? legacySource.item_id;
  return itemId === undefined ? "????????" : itemId.toString(16).padStart(8, "0");
}

function sourceTraitLevel(source: EquippedTraitSource): number | undefined {
  const legacySource = source as LegacyEquippedTraitSource;
  return source.traitLevel ?? legacySource.trait_level;
}

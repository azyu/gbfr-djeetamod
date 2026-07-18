import { CharacterType, ComputedSkillState } from "@/types";
import { getSkillName } from "@/utils";
import { Alert, Badge, Box, Group, Modal, SimpleGrid, Stack, Table, Text } from "@mantine/core";
import { Info } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";

type DamageDetailsModalProps = {
  characterType: CharacterType;
  skill: ComputedSkillState;
  opened: boolean;
  onClose: () => void;
};

const formatMultiplier = (value: number) => `${value.toFixed(3).replace(/0+$/, "").replace(/\.$/, "")}×`;

const formatDamage = (value: number) => value.toLocaleString(undefined, { maximumFractionDigits: 0 });

const modifierColor: Record<string, string> = {
  Attack: "orange",
  Defense: "blue",
  DamageLimit: "grape",
  BonusAttack: "cyan",
  Amplify: "red",
};

const formatStatusValue = (kind: string, value: number) => {
  if (kind === "BonusAttack") return value.toLocaleString(undefined, { maximumFractionDigits: 3 });
  return `${value >= 0 ? "+" : ""}${(value * 100).toFixed(0)}%`;
};

const ModifierCard = ({ label, value }: { label: string; value: number }) => (
  <Box p="xs" style={{ border: "1px solid var(--mantine-color-dark-4)", borderRadius: 4 }}>
    <Text size="xs" c="dimmed">
      {label}
    </Text>
    <Text fw={700}>{formatMultiplier(value)}</Text>
  </Box>
);

export const DamageDetailsModal = ({ characterType, skill, opened, onClose }: DamageDetailsModalProps) => {
  const { t } = useTranslation();
  const details = skill.damageDetails;

  if (!details) return null;

  const averageDamage = details.totalDamage / details.hits;

  return (
    <Modal
      opened={opened}
      onClose={onClose}
      title={`${t("ui.damage-details.title")} · ${getSkillName(characterType, skill)}`}
      size="xl"
    >
      <Stack gap="md">
        <Group justify="space-between">
          <Text fw={700}>{t("ui.damage-details.average-title")}</Text>
          <Text size="xs" c="dimmed">
            {t("ui.damage-details.hits-and-damage", {
              hits: details.hits,
              damage: details.totalDamage.toLocaleString(),
            })}
          </Text>
        </Group>

        <SimpleGrid cols={{ base: 2, sm: 5 }} spacing="xs">
          <ModifierCard label={t("ui.damage-details.elemental")} value={details.elementalMultiplier} />
          <ModifierCard label={t("ui.damage-details.amplify")} value={details.amplifyMultiplier} />
          <ModifierCard label={t("ui.damage-details.defense")} value={details.defenseMultiplier} />
          <ModifierCard label={t("ui.damage-details.attack")} value={details.attackMultiplier} />
          <ModifierCard label={t("ui.damage-details.supplementary")} value={details.supplementaryMultiplier} />
        </SimpleGrid>

        <SimpleGrid cols={{ base: 1, sm: 2 }} spacing="xs">
          <Box p="sm" bg="dark.7" style={{ borderRadius: 4 }}>
            <Text size="xs" c="dimmed">
              {t("ui.damage-details.formula-average")}
            </Text>
            <Text fw={700} size="xl">
              {formatMultiplier(details.formulaMultiplier)}
            </Text>
            <Text size="xs" c="dimmed">
              {t("ui.damage-details.formula")}
            </Text>
          </Box>
          <Box p="sm" bg="dark.7" style={{ borderRadius: 4 }}>
            <Text size="xs" c="dimmed">
              {t("ui.damage-details.observed-average")}
            </Text>
            <Text fw={700} size="xl">
              {formatMultiplier(details.observedMultiplier)}
            </Text>
            <Text size="xs" c="dimmed">
              {t("ui.damage-details.observed-description")}
            </Text>
          </Box>
        </SimpleGrid>

        <Alert icon={<Info size={16} />} color="blue" variant="light">
          {t("ui.damage-details.mismatch-note")}
        </Alert>

        <Table striped withTableBorder withColumnBorders>
          <Table.Tbody>
            <Table.Tr>
              <Table.Td>{t("ui.damage-details.attack-rate")}</Table.Td>
              <Table.Td>{formatMultiplier(details.attackRate)}</Table.Td>
              <Table.Td>{t("ui.damage-details.damage-limit")}</Table.Td>
              <Table.Td>{formatMultiplier(details.damageLimitMultiplier)}</Table.Td>
            </Table.Tr>
            <Table.Tr>
              <Table.Td>{t("ui.damage-details.uncapped-average")}</Table.Td>
              <Table.Td>{formatDamage(details.uncappedDamage)}</Table.Td>
              <Table.Td>{t("ui.damage-details.cap-average")}</Table.Td>
              <Table.Td>{formatDamage(details.damageCap)}</Table.Td>
            </Table.Tr>
            <Table.Tr>
              <Table.Td>{t("ui.damage-details.actual-average")}</Table.Td>
              <Table.Td colSpan={3}>{formatDamage(averageDamage)}</Table.Td>
            </Table.Tr>
          </Table.Tbody>
        </Table>

        {details.statuses.length > 0 && (
          <Stack gap={4}>
            <Text size="xs" fw={700}>
              {t("ui.damage-details.statuses")}
            </Text>
            <Group gap={6}>
              {details.statuses.map((status, statusIndex) => (
                <Badge
                  key={`${status.statusName}-${status.category}-${statusIndex}`}
                  color={modifierColor[status.kind] || "gray"}
                  variant="light"
                >
                  {status.statusName}: {formatStatusValue(status.kind, status.averageValue)} ·{" "}
                  {t("ui.damage-details.uptime", {
                    percent: ((status.activeHits / details.hits) * 100).toFixed(0),
                  })}
                </Badge>
              ))}
            </Group>
          </Stack>
        )}
      </Stack>
    </Modal>
  );
};

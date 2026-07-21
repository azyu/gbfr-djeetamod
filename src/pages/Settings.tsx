import { DragDropContext, Draggable, Droppable } from "@hello-pangea/dnd";
import {
  ActionIcon,
  Box,
  Button,
  Checkbox,
  ColorInput,
  Divider,
  Fieldset,
  Flex,
  Menu,
  Select,
  Slider,
  Stack,
  Switch,
  Text,
  Tooltip,
} from "@mantine/core";
import { DotsSixVertical } from "@phosphor-icons/react";
import { invoke } from "@tauri-apps/api";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import useRepeatQuest from "./useRepeatQuest";
import useSettings from "./useSettings";

const SettingsPage = () => {
  const { t, i18n } = useTranslation();
  const [debugMode, setDebugMode] = useState(false);

  const {
    connectionState,
    color_1,
    color_2,
    color_3,
    color_4,
    transparency,
    show_display_names,
    streamer_mode,
    show_full_values,
    use_condensed_skills,
    setMeterSettings,
    languages,
    handleLanguageChange,
    overlay_columns,
    handleReorderOverlayColumns,
    availableOverlayColumns,
    addOverlayColumn,
    removeOverlayColumn,
    open_log_on_save,
  } = useSettings();
  const repeatQuest = useRepeatQuest(connectionState);

  const toggleDebugMode = () => {
    const enabled = !debugMode;
    setDebugMode(enabled);
    invoke("set_debug_mode", { enabled });
    console.info("Debug Mode:", enabled ? "Enabled" : "Disabled");
  };

  return (
    <Box>
      <Stack gap={4} mb="md">
        <Text fw={700}>Djeeta MOD</Text>
        <Text size="sm" c="dimmed">
          GBFR Logs와 Awa Edition을 기반으로 제작되었습니다.
        </Text>
        <Text size="sm" c="dimmed">
          {t(`ui.connection.${connectionState}`)}
        </Text>
      </Stack>
      <Fieldset legend={t("ui.meter-settings")}>
        <Stack>
          <Select
            label={t("ui.language")}
            data={languages}
            defaultValue={i18n.language}
            allowDeselect={false}
            onChange={handleLanguageChange}
          />
          <ColorInput
            defaultValue={color_1}
            onChangeEnd={(value) => setMeterSettings({ color_1: value })}
            withEyeDropper={false}
            label={t("ui.player-1-color")}
            placeholder={t("ui.color-placeholder")}
          />
          <ColorInput
            defaultValue={color_2}
            onChangeEnd={(value) => setMeterSettings({ color_2: value })}
            withEyeDropper={false}
            label={t("ui.player-2-color")}
            placeholder={t("ui.color-placeholder")}
          />
          <ColorInput
            defaultValue={color_3}
            onChangeEnd={(value) => setMeterSettings({ color_3: value })}
            withEyeDropper={false}
            label={t("ui.player-3-color")}
            placeholder={t("ui.color-placeholder")}
          />
          <ColorInput
            defaultValue={color_4}
            onChangeEnd={(value) => setMeterSettings({ color_4: value })}
            withEyeDropper={false}
            label={t("ui.player-4-color")}
            placeholder={t("ui.color-placeholder")}
          />
          <Text size="sm">{t("ui.meter-transparency")}</Text>
          <Slider
            min={0}
            max={1}
            step={0.005}
            defaultValue={transparency}
            onChangeEnd={(value) => setMeterSettings({ transparency: value })}
          />
          <Checkbox
            label={t("ui.show-player-names")}
            checked={show_display_names}
            onChange={(event) => setMeterSettings({ show_display_names: event.currentTarget.checked })}
          />
          <Tooltip label={t("ui.streamer-mode-description")}>
            <Checkbox
              label={t("ui.streamer-mode")}
              checked={streamer_mode}
              onChange={(event) => setMeterSettings({ streamer_mode: event.currentTarget.checked })}
            />
          </Tooltip>
          <Tooltip label={t("ui.show-full-values-description")}>
            <Checkbox
              label={t("ui.show-full-values")}
              checked={show_full_values}
              onChange={(event) => setMeterSettings({ show_full_values: event.currentTarget.checked })}
            />
          </Tooltip>
          <Tooltip label={t("ui.use-condensed-skills-description")}>
            <Checkbox
              label={t("ui.use-condensed-skills")}
              checked={use_condensed_skills}
              onChange={(event) => setMeterSettings({ use_condensed_skills: event.currentTarget.checked })}
            />
          </Tooltip>
          <Tooltip label={t("ui.open-log-on-save-description")}>
            <Checkbox
              label={t("ui.open-log-on-save")}
              checked={open_log_on_save}
              onChange={(event) => setMeterSettings({ open_log_on_save: event.currentTarget.checked })}
            />
          </Tooltip>
          <Tooltip label={t("ui.debug-mode-description")}>
            <Checkbox label={t("ui.debug-mode")} checked={debugMode} onChange={toggleDebugMode} />
          </Tooltip>
          <Divider />
          <Text size="sm">{t("ui.customize-overlay-columns")}</Text>
          <Menu shadow="md" trigger="hover" openDelay={100} closeDelay={400}>
            <Menu.Target>
              <Button>{t("ui.add-column")}</Button>
            </Menu.Target>
            <Menu.Dropdown>
              {availableOverlayColumns.map((item) => (
                <Menu.Item key={item} onClick={() => addOverlayColumn(item)}>
                  {t(`ui.meter-columns.${item}`)} - {t(`ui.meter-columns.${item}-description`)}
                </Menu.Item>
              ))}
            </Menu.Dropdown>
          </Menu>
          <DragDropContext onDragEnd={handleReorderOverlayColumns}>
            <Droppable droppableId="overlay-columns">
              {(droppableProvided) => (
                <Stack ref={droppableProvided.innerRef}>
                  {overlay_columns.map((item, index) => (
                    <Draggable key={item} draggableId={item} index={index}>
                      {(draggableProvided) => (
                        <Box
                          bg="var(--mantine-color-dark-8)"
                          display="flex"
                          p={10}
                          ref={draggableProvided.innerRef}
                          {...draggableProvided.draggableProps}
                          {...draggableProvided.dragHandleProps}
                        >
                          <Flex align="center" flex={1}>
                            <DotsSixVertical size={16} style={{ cursor: "grab", marginRight: "0.5em" }} />
                            {t(`ui.meter-columns.${item}`)} - {t(`ui.meter-columns.${item}-description`)}
                          </Flex>
                          <Flex align="center">
                            <ActionIcon
                              aria-label={t("ui.remove-column")}
                              variant="transparent"
                              color="gray"
                              onClick={() => removeOverlayColumn(item)}
                            >
                              x
                            </ActionIcon>
                          </Flex>
                        </Box>
                      )}
                    </Draggable>
                  ))}
                  {droppableProvided.placeholder}
                </Stack>
              )}
            </Droppable>
          </DragDropContext>
        </Stack>
      </Fieldset>
      <Fieldset legend={t("ui.game-features.title")} mt="md">
        <Stack gap="xs">
          <Switch
            label={t("ui.game-features.repeat-quest.label")}
            checked={repeatQuest.status?.state === "on"}
            disabled={repeatQuest.pending || repeatQuest.status === null || repeatQuest.status.state === "unavailable"}
            onChange={(event) => void repeatQuest.setEnabled(event.currentTarget.checked)}
          />
          <Text size="sm" c="dimmed">
            {t("ui.game-features.repeat-quest.description")}
          </Text>
          {repeatQuest.status?.reason && (
            <Text size="sm" c="red">
              {t(`ui.game-features.repeat-quest.reason.${repeatQuest.status.reason}`)}
            </Text>
          )}
        </Stack>
      </Fieldset>
    </Box>
  );
};

export default SettingsPage;

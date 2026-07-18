import { SUPPORTED_LANGUAGES } from "@/i18n";
import { useMeterSettingsStore } from "@/stores/useMeterSettingsStore";
import { ConnectionState, MeterColumns } from "@/types";
import { DropResult } from "@hello-pangea/dnd";
import { invoke } from "@tauri-apps/api";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const reorder = <TList extends unknown[]>(list: TList, startIndex: number, endIndex: number): TList => {
  const result = Array.from(list) as TList;
  const [removed] = result.splice(startIndex, 1);
  result.splice(endIndex, 0, removed);

  return result;
};

export default function useSettings() {
  const [connectionState, setConnectionState] = useState<ConnectionState>("searching");
  const {
    color_1,
    color_2,
    color_3,
    color_4,
    transparency,
    show_display_names,
    streamer_mode,
    show_full_values,
    use_condensed_skills,
    overlay_columns,
    open_log_on_save,
    setMeterSettings,
  } = useMeterSettingsStore((state) => ({
    color_1: state.color_1,
    color_2: state.color_2,
    color_3: state.color_3,
    color_4: state.color_4,
    transparency: state.transparency,
    show_display_names: state.show_display_names,
    streamer_mode: state.streamer_mode,
    show_full_values: state.show_full_values,
    use_condensed_skills: state.use_condensed_skills,
    open_log_on_save: state.open_log_on_save,
    setMeterSettings: state.set,
    overlay_columns: state.overlay_columns,
  }));

  const { i18n } = useTranslation();

  useEffect(() => {
    let disposed = false;
    let unsubscribe: (() => void) | undefined;
    void listen<ConnectionState>("connection-state", (event) => setConnectionState(event.payload)).then(
      async (unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        unsubscribe = unlisten;
        const currentState = await invoke<ConnectionState>("get_connection_state");
        if (!disposed) setConnectionState(currentState);
      }
    );

    return () => {
      disposed = true;
      unsubscribe?.();
    };
  }, []);

  const handleLanguageChange = (language: string | null) => {
    i18n.changeLanguage(language as string);
  };

  const handleReorderOverlayColumns = (result: DropResult) => {
    if (!result.destination) return;
    const items = reorder(overlay_columns, result.source.index, result.destination.index);
    setMeterSettings({ overlay_columns: items });
  };

  // Adds a column to the overlay_columns array if it doesn't exist.
  const addOverlayColumn = (column: MeterColumns) => {
    const items = [...overlay_columns];

    if (items.indexOf(column) === -1) {
      items.push(column);
      setMeterSettings({ overlay_columns: items });
    }
  };

  // Removes a column from the overlay_columns array.
  const removeOverlayColumn = (column: MeterColumns) => {
    const items = overlay_columns.filter((item) => item !== column);
    setMeterSettings({ overlay_columns: items });
  };

  const languages = Object.keys(SUPPORTED_LANGUAGES).map((key) => ({ value: key, label: SUPPORTED_LANGUAGES[key] }));

  const availableOverlayColumns = Object.values(MeterColumns).filter(
    (column) => overlay_columns.indexOf(column) === -1 && column !== MeterColumns.Name
  );

  return {
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
    overlay_columns,
    availableOverlayColumns,
    open_log_on_save,
    handleLanguageChange,
    handleReorderOverlayColumns,
    addOverlayColumn,
    removeOverlayColumn,
  };
}

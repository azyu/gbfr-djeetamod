import { CSSProperties } from "react";
import { useTranslation } from "react-i18next";

import { CompactMeterRow } from "./compactMeterModel";

import "./CompactDamageMeter.css";

const format = new Intl.NumberFormat("ko-KR", { maximumFractionDigits: 0 });

export type CompactDamageMeterProps = {
  rows: CompactMeterRow[];
  transparency: number;
};

export const CompactDamageMeter = ({ rows, transparency }: CompactDamageMeterProps) => {
  const { t } = useTranslation();
  const unknown = t("ui.compact-meter.unknown-character");
  const characterName = (row: CompactMeterRow) => {
    if (typeof row.characterType !== "string") return unknown;
    const translated = t(`characters:${row.characterType}`, { defaultValue: "" });
    return translated.trim() ? translated : unknown;
  };

  return (
    <section
      className="compact-meter"
      style={{ "--meter-opacity": transparency } as CSSProperties}
      aria-label={t("ui.compact-meter.title")}
    >
      <header className="compact-meter__header" data-tauri-drag-region>
        {t("ui.compact-meter.title")}
      </header>
      {rows.slice(0, 4).map((row) => (
        <div className="compact-meter__row" key={row.actorIndex}>
          <div className="compact-meter__bar" style={{ width: `${row.barPercent}%` }} aria-hidden="true" />
          <div className="compact-meter__content">
            <span className="compact-meter__name">{characterName(row)}</span>
            <span>{format.format(row.totalDamage)}</span>
            <span>{format.format(row.dps)} DPS</span>
          </div>
        </div>
      ))}
    </section>
  );
};

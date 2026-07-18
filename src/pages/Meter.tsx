import { CompactDamageMeter } from "@/components/compact-meter/CompactDamageMeter";
import "@/i18n";

import useCompactMeter from "./useCompactMeter";

export const Meter = () => {
  const { rows, transparency } = useCompactMeter();
  if (rows.length === 0) return null;
  return <CompactDamageMeter rows={rows} transparency={transparency} />;
};

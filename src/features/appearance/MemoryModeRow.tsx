import { SettingsChoice } from "../../components/settings/SettingsChoice";
import { SettingsRow } from "../../components/settings/SettingsRow";
import { useTranslation } from "../../i18n";
import type { IslandSettings, MemoryMode } from "./types";

const MODES = ["saver", "balanced", "smooth"] as const;

/**
 * Settings → General → Memory: how soon the collapsed island hands the browser's memory back
 * to Windows. The trade-off changes per mode, so the line under the title follows the choice.
 */
export function MemoryModeRow({
  settings,
  onSettingsChange,
}: {
  settings: IslandSettings;
  onSettingsChange: (settings: IslandSettings) => void;
}) {
  const text = useTranslation().settings.memory;
  const options = MODES.map((value) => ({ value, label: text.modes[value] }));
  return (
    <SettingsRow
      title={text.mode}
      description={<>{text.modeHint}<br />{text.modeSummary[settings.memoryMode]}</>}
      control={
        <SettingsChoice<MemoryMode>
          value={settings.memoryMode}
          options={options}
          ariaLabel={text.mode}
          onChange={(memoryMode) => onSettingsChange({ ...settings, memoryMode })}
        />
      }
    />
  );
}

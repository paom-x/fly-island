import "./SettingsChoice.css";

export type SettingsChipOption<T extends string> = { value: T; label: string };

/**
 * Several options side by side with any number of them pressed. Unlike a SettingsChoice, none of
 * them is exclusive: each chip is its own switch, styled to sit in the same row as a choice.
 */
export function SettingsChips<T extends string>({
  values,
  options,
  ariaLabel,
  onToggle,
}: {
  values: readonly T[];
  options: readonly SettingsChipOption<T>[];
  ariaLabel: string;
  onToggle: (value: T, on: boolean) => void;
}) {
  return (
    <div className="settings-choice" role="group" aria-label={ariaLabel}>
      {options.map((option) => {
        const on = values.includes(option.value);
        return (
          <button
            key={option.value}
            className={on ? "settings-choice__option settings-choice__option--active" : "settings-choice__option"}
            type="button"
            aria-pressed={on}
            onClick={() => onToggle(option.value, !on)}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}

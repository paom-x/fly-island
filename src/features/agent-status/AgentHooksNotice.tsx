import { useEffect, useState } from "react";
import { CircleAlert } from "lucide-react";
import { SettingsGroup } from "../../components/settings/SettingsGroup";
import { SettingsLinkButton } from "../../components/settings/SettingsLinkButton";
import { SettingsRow } from "../../components/settings/SettingsRow";
import { useTranslation } from "../../i18n";
import { getAgentHooksState, type AgentHooksState } from "./api";
import { getAgentHooksNotice, type AgentHooksShown } from "./status";

/**
 * On the Sessions page, a row saying the hooks are missing or old, with the way to the
 * Connect AI tools page where they are installed. Nothing while they are current, and nothing
 * while every hooked agent is switched off right above it: a source the user turned off reads
 * nothing from its hooks, so asking to install them would only be noise.
 */
export function AgentHooksNotice({ onOpenConnect, shown }: { onOpenConnect: () => void; shown: AgentHooksShown }) {
  const text = useTranslation().settings.agent;
  const [state, setState] = useState<AgentHooksState | null>(null);

  useEffect(() => {
    let cancelled = false;
    void getAgentHooksState()
      .then((next) => {
        // An older native side, or a page mocking it, may give nothing usable.
        if (!cancelled) setState(next && typeof next === "object" ? next : null);
      })
      .catch((error: unknown) => console.error("Failed to read agent hooks state", error));
    return () => {
      cancelled = true;
    };
  }, []);

  // Judged on the sources switched on right now, so turning one off takes the row away at once
  // without refetching the install state.
  const notice = state ? getAgentHooksNotice(state, shown) : null;
  if (!notice) return null;
  return (
    <SettingsGroup>
      <SettingsRow
        title={
          <span className="agent-hooks-message agent-hooks-message--attention">
            <CircleAlert size={13} strokeWidth={2.4} aria-hidden="true" />
            {notice === "update" ? text.hooksOutdated : text.hooksMissing}
          </span>
        }
        control={<SettingsLinkButton onClick={onOpenConnect}>{text.install}</SettingsLinkButton>}
      />
    </SettingsGroup>
  );
}

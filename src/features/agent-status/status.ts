import type { Translations } from "../../i18n";
import type { AgentHooksState, AgentProvider, HookAgentProvider, AgentStatusSnapshot, AgentTaskPhase, AgentTaskStatus } from "./api";
import type { SessionFilter } from "./sessionList";

export type AgentVisualState = "idle" | "running" | "attention";
export type AgentHooksInstallState = "idle" | "installing" | "installed" | "error";

export const DEFAULT_AGENT_TASK_STATUS: AgentTaskStatus = {
  phase: "idle",
  updatedAt: 0,
};
export const DEFAULT_AGENT_STATUS: AgentStatusSnapshot = {
  codex: DEFAULT_AGENT_TASK_STATUS,
  claudeCode: DEFAULT_AGENT_TASK_STATUS,
  zcode: DEFAULT_AGENT_TASK_STATUS,
  updatedAt: 0,
  statusPath: "",
  others: [],
};
export const AGENT_PROVIDERS: HookAgentProvider[] = ["codex", "claudeCode", "zcode"];
/**
 * The providers whose hooks gate the install notice. Z Code's hooks only feed the island light —
 * its sessions come from the zcode plugin and Z Code may not even be installed — so a machine
 * without it must not be nagged to install anything; its row on the connection page still shows
 * the state of its own hooks.
 */
const HOOK_NOTICE_PROVIDERS: HookAgentProvider[] = ["codex", "claudeCode"];
export const AGENT_PROVIDER_LABELS: Record<AgentProvider, string> = {
  codex: "Codex",
  claudeCode: "Claude Code",
  dsh: "DeepSeek Harness",
  workbuddy: "WorkBuddy",
  plugin: "Plugin",
  cursor: "Cursor",
  kimiCode: "Kimi Code",
  hermes: "Hermes",
  zcode: "Z Code",
};
/** One row of agent status: a built-in provider or a source reported by a plugin or the local API. */
export type AgentStatusEntry = {
  /** The provider for built-ins, the source id for the others; what clear_agent_status takes. */
  key: string;
  label: string;
  /** Null for reported sources, which have no brand icon. */
  provider: AgentProvider | null;
  status: AgentTaskStatus;
};

export function listAgentStatuses(snapshot: AgentStatusSnapshot): AgentStatusEntry[] {
  return [
    ...AGENT_PROVIDERS.map((provider) => ({
      key: provider,
      label: AGENT_PROVIDER_LABELS[provider],
      provider,
      // A snapshot written before a provider existed has no key for it; show idle rather than
      // throwing, so adding a provider never blanks the whole island on an old status file.
      status: snapshot[provider] ?? DEFAULT_AGENT_TASK_STATUS,
    })),
    ...(snapshot.others ?? []).map((other) => ({ key: other.id, label: other.name, provider: null, status: other })),
  ];
}

/** The snapshot with the agents switched off in Settings → AI Agent shown as idle. */
export function withoutHiddenProviders(snapshot: AgentStatusSnapshot, shown: SessionFilter): AgentStatusSnapshot {
  const hidden = AGENT_PROVIDERS.filter((provider) => !shown[provider]);
  if (hidden.length === 0) return snapshot;
  return { ...snapshot, ...Object.fromEntries(hidden.map((provider) => [provider, DEFAULT_AGENT_TASK_STATUS])) };
}

export function isAgentAttentionPhase(phase: AgentTaskPhase) {
  return phase === "failed" || phase === "stale";
}

/**
 * `activeSessions`: sessions the session list still counts as working. The light turns blue
 * for them too, so it never shows green beside the island's blue session count.
 */
export function getAgentVisualState(snapshot: AgentStatusSnapshot, activeSessions = 0): AgentVisualState {
  const statuses = listAgentStatuses(snapshot).map((entry) => entry.status);

  if (statuses.some((status) => isAgentAttentionPhase(status.phase))) {
    return "attention";
  }

  if (activeSessions > 0 || statuses.some((status) => status.phase === "running")) {
    return "running";
  }

  return "idle";
}

/** What the settings page should say about the installed hooks, or null when they are current. */
export type AgentHooksNotice = "update" | "install";

/** Which hooked agents Settings → AI Agent still shows; a source switched off reads nothing from its hooks. */
export type AgentHooksShown = Partial<Record<HookAgentProvider, boolean>>;

/**
 * The notice about the hooks, judged only on the agents the user left switched on. Turning a source
 * off in Settings → AI Agent already hides its sessions, so its missing or old hooks cannot change
 * anything the user sees, and asking to install them would be pure noise; with both switched off
 * there is no notice at all. `shown` omitted counts every agent, for callers that only know the
 * install state.
 */
export function getAgentHooksNotice(state: AgentHooksState | null, shown?: AgentHooksShown): AgentHooksNotice | null {
  if (!state) return null;
  const providers = HOOK_NOTICE_PROVIDERS.filter((provider) => shown?.[provider] ?? true);
  if (providers.length === 0) return null;
  if (state.scriptsOutdated || providers.some((provider) => state[provider] === "outdated")) {
    return "update";
  }
  return providers.every((provider) => state[provider] === "notInstalled") ? "install" : null;
}

/** What the status light says (its aria-label); `activeSessions` running in the session list also count as running. */
export function getAgentStatusLabel(snapshot: AgentStatusSnapshot, t: Translations, activeSessions = 0) {
  const entries = listAgentStatuses(snapshot);
  const attention = entries.find((entry) => isAgentAttentionPhase(entry.status.phase));

  if (attention) {
    return attention.status.phase === "stale"
      ? t.agent.stale(attention.label)
      : t.agent.failed(attention.label);
  }

  const running = entries.find((entry) => entry.status.phase === "running");

  if (running) {
    return t.agent.running(running.label);
  }

  return activeSessions > 0 ? t.agent.running("AI Agent") : t.agent.idleOrDone;
}

export function getAgentPhaseLabel(phase: AgentTaskPhase, t: Translations) {
  switch (phase) {
    case "running":
      return t.agent.phase.running;
    case "completed":
      return t.agent.phase.completed;
    case "failed":
      return t.agent.phase.failed;
    case "stale":
      return t.agent.phase.stale;
    case "waiting":
      return t.agent.phase.waiting;
    case "idle":
    default:
      return t.agent.phase.idle;
  }
}

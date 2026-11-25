import type { Translations } from "../../i18n";
import type {
  AgentHooksState,
  AgentProvider,
  AgentSessionSummary,
  HookAgentProvider,
  AgentStatusSnapshot,
  AgentTaskPhase,
  AgentTaskStatus,
} from "./api";
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
 * its sessions are read from Z Code's own store, and Z Code may not even be installed — so a
 * machine without it must not be nagged to install anything; its row on the connection page still
 * shows the state of its own hooks.
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

/**
 * The sources the native side reads on its own, without hooks: DSH's desktop session files and
 * the WorkBuddy browser. They still light the island, so the connection page lists them with the
 * hooked agents — there is nothing to install for them, and their state cannot be cleared.
 */
const NATIVE_STATUS_PROVIDERS: AgentProvider[] = ["dsh", "workbuddy"];

/**
 * The phases of the locally read agents, taken from the sessions the native side lists: a source
 * is working while any of its sessions is. Its hooks are not installed and it writes no
 * agent-status.json key, so this is the only place its state can come from — and being live, it
 * needs no way to clear it.
 */
export function liveAgentStatuses(sessions: readonly AgentSessionSummary[]): Partial<Record<AgentProvider, AgentTaskStatus>> {
  const live: Partial<Record<AgentProvider, AgentTaskStatus>> = {};
  for (const session of sessions) {
    if (!NATIVE_STATUS_PROVIDERS.includes(session.provider)) continue;
    const phase = session.phase === "running" ? "running" : session.phase === "waiting" ? "waiting" : null;
    if (!phase) continue;
    // A working session outranks one of its siblings waiting on the user.
    if (live[session.provider]?.phase === "running") continue;
    live[session.provider] = { phase, updatedAt: session.updatedAt };
  }
  return live;
}

/**
 * The status table on the connection page: hooked agents, then the locally read ones, then any
 * source a plugin or the local API reported. live carries the state of the agents read without
 * hooks (see liveAgentStatuses).
 */
export function listAgentLightStatuses(
  snapshot: AgentStatusSnapshot,
  live: Partial<Record<AgentProvider, AgentTaskStatus>> = {},
): AgentStatusEntry[] {
  // Only the hooked agents have a key in the snapshot; the rest fall back to live and then idle.
  const written = snapshot as Partial<Record<AgentProvider, AgentTaskStatus>>;
  const builtIn = [...AGENT_PROVIDERS, ...NATIVE_STATUS_PROVIDERS].map(
    (provider): AgentStatusEntry => ({
      key: provider,
      label: AGENT_PROVIDER_LABELS[provider],
      provider,
      // A snapshot from an older build can still carry an idle placeholder for these agents;
      // the session list is the truth for them.
      status: live[provider] ?? written[provider] ?? DEFAULT_AGENT_TASK_STATUS,
    }),
  );
  const reported = listAgentStatuses(snapshot).filter((entry) => entry.provider === null);
  return [...builtIn, ...reported];
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

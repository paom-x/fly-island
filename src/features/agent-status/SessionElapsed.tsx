import { useEffect, useState } from "react";
import { useTranslation } from "../../i18n";
import { useIslandAwake } from "../../lib/islandAwake";
import type { AgentSessionSummary } from "./api";
import { durationParts, isActivePhase, turnElapsedMs } from "./sessionList";

/**
 * "已运行 3 分 20 秒" while the session works, "用时 4 分 12 秒" once it is done. Only a working
 * session ticks, every second, here in the leaf so the island around it does not re-render. It
 * stops entirely while the island is collapsed: the card is hidden, and the first tick after the
 * panel opens catches the clock up because it always reads `Date.now()` instead of counting.
 */
export function SessionElapsed({ session }: { session: AgentSessionSummary }) {
  const t = useTranslation();
  const awake = useIslandAwake();
  const active = isActivePhase(session.phase);
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    if (!active || !awake) return;
    setNow(Date.now());
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [active, awake]);

  const elapsed = turnElapsedMs(session, now);
  if (elapsed === null) return null;
  const duration = t.agent.sessions.duration(durationParts(elapsed));
  return (
    <span className="agent-session__elapsed">
      {active ? t.agent.sessions.elapsedRunning(duration) : t.agent.sessions.elapsedDone(duration)}
    </span>
  );
}

import { useSyncExternalStore } from "react";

// Whether the island's panel is on screen. While the island is collapsed only the collapsed row is
// visible: the panel pages stay mounted, so their timers would keep ticking for a UI nobody can
// see. Set by IslandShell (windows/island), read by the hooks and components inside those pages.
let awake = true;
const listeners = new Set<() => void>();

/** Notes whether the island's panel is open (`true`) or only its collapsed row is showing. */
export function setIslandAwake(next: boolean) {
  if (awake === next) return;
  awake = next;
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/**
 * True while the island's panel is open. Windows that never show the island (settings, eye care,
 * message lists) stay `true`, so their hooks keep their normal cadence.
 */
export function useIslandAwake(): boolean {
  return useSyncExternalStore(subscribe, () => awake, () => true);
}

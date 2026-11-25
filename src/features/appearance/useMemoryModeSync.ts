import { useEffect } from "react";
import { setMemoryMode } from "./api";
import type { MemoryMode } from "./types";

/**
 * Keeps the Rust trimmer on the policy chosen in Settings → General → Memory. The island
 * window lives for the whole run, so its copy of the setting is the one that sticks: it is
 * sent once at start-up and again whenever the mode changes, including changes made in the
 * settings window (the settings hook here follows the same storage events).
 */
export function useMemoryModeSync(mode: MemoryMode) {
  useEffect(() => {
    setMemoryMode(mode).catch((error: unknown) => {
      console.error("Failed to set the memory mode", error);
    });
  }, [mode]);
}

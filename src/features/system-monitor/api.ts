import { invoke } from "@tauri-apps/api/core";
import type { SystemStats } from "./types";

/** One reading of the machine's counters. The command differences against its own last call. */
export function getSystemStats() {
  return invoke<SystemStats>("system_stats");
}

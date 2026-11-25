import { invoke } from "@tauri-apps/api/core";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import type { MemoryMode } from "./types";

/** Matches InstalledFont in src-tauri/src/system/fonts.rs. */
export type InstalledFont = { family: string; label: string };

/**
 * Tells the trimmer (src-tauri/src/memory_trim.rs) how hard to work. It keeps the mode in
 * memory and lives for the whole run, so this is sent at start-up and whenever it changes.
 */
export function setMemoryMode(mode: MemoryMode) {
  return invoke<void>("set_memory_mode", { mode });
}

export function listInstalledFonts(locale: string) {
  return invoke<InstalledFont[]>("list_installed_fonts", { locale });
}

/**
 * Saves a theme file to the Downloads folder (never overwriting); resolves to the file's path.
 */
export function saveThemeFile(fileName: string, content: string) {
  return invoke<string>("save_theme_file", { fileName, content });
}

/** Shows a saved theme file in Explorer. */
export function revealThemeFile(path: string) {
  return revealItemInDir(path);
}

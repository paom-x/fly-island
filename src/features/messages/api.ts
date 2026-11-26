import { invoke } from "@tauri-apps/api/core";

/**
 * Tells the native side how much of a notification it may read. The QQ and WeChat
 * listeners run on their own threads, so this is process state, not a saved setting:
 * it is pushed again on every start and whenever Settings → Messages changes.
 */
export function setMessageReading(content: boolean): Promise<void> {
  return invoke<void>("set_message_reading", { content });
}

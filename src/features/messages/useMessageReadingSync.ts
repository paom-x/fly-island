import { useEffect } from "react";
import type { IslandSettings } from "../appearance/types";
import { setMessageReading } from "./api";

/**
 * Keeps the near side's reading mode in step with Settings → Messages. In "only unread"
 * the QQ listener never fetches a notification's text elements and the WeChat reader
 * drops its summaries, so neither the sender nor the words reach this process.
 */
export function useMessageReadingSync(settings: IslandSettings) {
  const readsContent = settings.messageReading === "content";
  useEffect(() => {
    void setMessageReading(readsContent);
  }, [readsContent]);
}

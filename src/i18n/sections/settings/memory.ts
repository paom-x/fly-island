export type SettingsMemoryText = {
  title: string;
  /** How hard the island works to hand the browser's memory back to Windows. */
  mode: string;
  modeHint: string;
  modes: { saver: string; balanced: string; smooth: string };
  /** What each mode trades away, in one line under the choice. */
  modeSummary: { saver: string; balanced: string; smooth: string };
};

export const zhCN: SettingsMemoryText = {
  title: "内存",
  mode: "内存策略",
  modeHint: "岛收起以后，把浏览器占着的内存还给系统的力度。",
  modes: { saver: "省内存", balanced: "平衡", smooth: "顺滑" },
  modeSummary: {
    saver: "收起 10 秒后归还内存，回来的第一下悬停要等页面重新读入。",
    balanced: "收起 30 秒后归还内存，悬停基本感觉不到等待。",
    smooth: "不归还内存，悬停最快，但常驻占用也最高。",
  },
};

export const en: SettingsMemoryText = {
  title: "Memory",
  mode: "Memory policy",
  modeHint: "How hard the island hands the browser's memory back to Windows once it is collapsed.",
  modes: { saver: "Saver", balanced: "Balanced", smooth: "Smooth" },
  modeSummary: {
    saver: "Gives memory back 10 seconds after collapsing; the first hover waits for the page to be read in again.",
    balanced: "Gives memory back 30 seconds after collapsing; hovering still feels instant.",
    smooth: "Never gives memory back: the quickest hover, the largest idle footprint.",
  },
};

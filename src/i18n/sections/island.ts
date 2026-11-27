export type IslandText = {
  appName: string;
  focusing: (taskTitle: string) => string;
  pendingTodos: (count: number) => string;
  agentSession: (provider: string, title: string) => string;
  openMusic: string;
  collapse: string;
  collapseIsland: string;
  minimizeToTray: string;
  tuckToTop: string;
  resetPosition: string;
  resetPositionLabel: string;
  /** The settings button while its red dot leads to the community group in About. */
  settingsWithNews: string;
  editor: string;
  todoList: string;
  todoListWithUpdate: string;
  music: string;
  clipboardHistory: string;
  shelf: string;
  /** Short page names in the tips above the header dots. */
  pageNames: { todo: string; music: string; clipboard: string; shelf: string; translation: string; system: string };
  /** The machine page: one card per counter, each with the last two minutes behind it. */
  systemPage: {
    title: string;
    reading: string;
    cpu: string;
    memory: string;
    network: string;
    gpu: string;
    temperature: string;
    unavailable: string;
    memoryDetail: (used: string, total: string) => string;
    trend: (from: string, to: string) => string;
    oneReading: string;
    networkTrend: string;
    note: string;
    noteWithoutTemperature: string;
  };
  /** The tip on the status light, which opens the Agent page. */
  agentPage: string;
  calendar: string;
  shelfCount: (count: number) => string;
layoutEditor: string;
  /** Metric names on the collapsed island's system-monitor line. */
  systemMetrics: { cpu: string; memory: string; network: string; gpu: string; temperature: string };
};

export const zhCN: IslandText = {
  appName: "fly-island",
  focusing: (taskTitle) => `正在专注：${taskTitle}`,
  pendingTodos: (count) => `剩余${count}个待办`,
  agentSession: (provider, title) => `${provider} 正在处理：${title}`,
  openMusic: "打开音乐控制",
  collapse: "收起",
  collapseIsland: "收起岛屿",
  minimizeToTray: "最小化到托盘",
  tuckToTop: "收起到屏幕顶部",
  resetPosition: "复位",
  resetPositionLabel: "回到主屏顶部居中",
  settingsWithNews: "设置（关于里有新内容）",
  editor: "岛屿编辑",
  todoList: "任务清单",
  todoListWithUpdate: "任务清单（有新版本）",
  music: "Music",
  clipboardHistory: "剪贴板历史",
  shelf: "文件中转",
  pageNames: { todo: "待办", music: "音乐", clipboard: "剪贴板", shelf: "文件中转", translation: "翻译", system: "机器占用" },
  systemPage: {
    title: "机器占用",
    reading: "正在读…",
    cpu: "处理器",
    memory: "内存",
    network: "网络",
    gpu: "显卡",
    temperature: "温度",
    unavailable: "读不到",
    memoryDetail: (used, total) => `已用 ${used} / ${total} GB`,
    trend: (from, to) => `两分钟里从 ${from} 变到 ${to}`,
    oneReading: "刚开始记录这条曲线",
    networkTrend: "两分钟内的下载速度",
    note: "每 2 秒读一次，只画最近 2 分钟；离开这个页面就停止读取。",
    noteWithoutTemperature: "每 2 秒读一次，只画最近 2 分钟；离开这个页面就停止读取。这台电脑没有能读到的温度传感器。",
  },
  agentPage: "AI Agent",
  calendar: "日历",
  shelfCount: (count) => `文件中转里有 ${count} 个文件`,
  layoutEditor: "布局编辑",
  systemMetrics: { cpu: "CPU", memory: "内存", network: "网络", gpu: "GPU", temperature: "温度" },
};

export const en: IslandText = {
  appName: "fly-island",
  focusing: (taskTitle) => `Focusing: ${taskTitle}`,
  pendingTodos: (count) => (count === 1 ? "1 todo left" : `${count} todos left`),
  agentSession: (provider, title) => `${provider} is working on: ${title}`,
  openMusic: "Open music controls",
  collapse: "Collapse",
  collapseIsland: "Collapse island",
  minimizeToTray: "Minimize to tray",
  tuckToTop: "Tuck to top edge",
  resetPosition: "Reset",
  resetPositionLabel: "Reset island to the top center of the primary screen",
  settingsWithNews: "Settings (something new in About)",
  editor: "Island editor",
  todoList: "Tasks",
  todoListWithUpdate: "Tasks (update available)",
  music: "Music",
  clipboardHistory: "Clipboard history",
  shelf: "File shelf",
  pageNames: { todo: "Todo", music: "Music", clipboard: "Clipboard", shelf: "File shelf", translation: "Translate", system: "Machine load" },
  systemPage: {
    title: "Machine load",
    reading: "Reading…",
    cpu: "Processor",
    memory: "Memory",
    network: "Network",
    gpu: "Graphics",
    temperature: "Temperature",
    unavailable: "No reading",
    memoryDetail: (used, total) => `${used} of ${total} GB in use`,
    trend: (from, to) => `From ${from} to ${to} over two minutes`,
    oneReading: "Still drawing this one",
    networkTrend: "Download speed over two minutes",
    note: "A reading every two seconds, drawing the last two minutes; closing the page stops it.",
    noteWithoutTemperature: "A reading every two seconds, drawing the last two minutes; closing the page stops it. This computer has no readable temperature sensor.",
  },
  agentPage: "AI Agent",
  calendar: "Calendar",
  shelfCount: (count) => (count === 1 ? "1 file on the shelf" : `${count} files on the shelf`),
  layoutEditor: "Layout editor",
  systemMetrics: { cpu: "CPU", memory: "Memory", network: "Network", gpu: "GPU", temperature: "Temp" },
};

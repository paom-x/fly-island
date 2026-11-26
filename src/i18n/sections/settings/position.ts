export type SettingsPositionText = {
  title: string;
  moveHint: string;
  lock: string;
  lockHint: string;
  /** Row that puts the island back where it starts. */
  resetRow: string;
  reset: string;
  /** Switch that steps the collapsed island along the top edge (OLED burn-in). */
  oledShift: string;
  oledShiftHint: string;
  /** Row that chooses which screen edge the island is docked to. */
  dockEdge: string;
  dockEdgeHint: string;
  dockEdgeTop: string;
  dockEdgeLeft: string;
  dockEdgeRight: string;
};

export const zhCN: SettingsPositionText = {
  title: "位置",
  lock: "锁定小岛位置",
  lockHint: "开启后无法通过拖动移动小岛，仍可使用复位。",
  moveHint: "收起时直接拖动小岛；展开时按住 Ctrl 再拖。拖到屏幕顶部附近会自动贴齐。",
  resetRow: "回到默认位置",
  reset: "复位",
  oledShift: "OLED 防烧屏",
  oledShiftHint: "每 3 分钟把收起的小岛沿屏幕顶边挪 1~2 像素，避免同一列像素长时间常亮。幅度很小，平时看不出来。",
  dockEdge: "停靠边",
  dockEdgeHint: "岛贴在哪条边上。选左边或右边时窗口竖起来，岛转四分之一圈贴在那条边上，收起时朝那条边缩回去。",
  dockEdgeTop: "顶边",
  dockEdgeLeft: "左边",
  dockEdgeRight: "右边",
};

export const en: SettingsPositionText = {
  title: "Position",
  lock: "Lock island position",
  lockHint: "Prevent moving the island by dragging. Reset remains available.",
  moveHint: "Drag the collapsed island to move it; when it is open, hold Ctrl while dragging. Drop it near the top of the screen to snap flush.",
  resetRow: "Back to the default position",
  reset: "Reset",
  oledShift: "OLED burn-in guard",
  oledShiftHint: "Every three minutes the collapsed island steps one or two pixels along the top edge, so one column of pixels is not lit for hours. The step is small enough that it is not meant to show.",
  dockEdge: "Docked edge",
  dockEdgeHint: "Which edge the island sits on. On the left or right the window stands on its end, the island turns a quarter turn against that edge, and tucking slides it back into the same one.",
  dockEdgeTop: "Top",
  dockEdgeLeft: "Left",
  dockEdgeRight: "Right",
};

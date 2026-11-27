import { act } from "react";
import { TRANSLATIONS } from "../../src/i18n";

type Tour = {
  mount: () => void;
  click: (selector: string) => Promise<void>;
  settle: (ms: number) => Promise<void>;
  check: (condition: unknown, message: string) => void;
  results: HTMLElement;
  messages: string[];
};

function visibleNames(row: HTMLElement) {
  return [...row.querySelectorAll<HTMLElement>(".page-button-content__text")].filter((text) =>
    text.clientWidth > 0 && text.getClientRects().length > 0 && getComputedStyle(text).visibility === "visible");
}

function visibleNamesFit(row: HTMLElement, expected: readonly string[]) {
  const names = visibleNames(row);
  return names.length === expected.length && names.every((text, index) => {
    const button = text.closest("[data-page-button]")!.getBoundingClientRect();
    return text.textContent === expected[index] && text.scrollWidth <= text.clientWidth
      && getComputedStyle(text).whiteSpace === "nowrap"
      && text.getBoundingClientRect().right <= button.right + 0.25;
  });
}

function rowFits(row: HTMLElement, actions: Element) {
  const { left, right, top, bottom } = row.getBoundingClientRect();
  return right <= actions.getBoundingClientRect().left + 0.25
    && [...row.querySelectorAll("[data-page-button]")].every((button) => {
      const box = button.getBoundingClientRect();
      return box.left >= left - 0.25 && box.right <= right + 0.25 && box.top >= top - 0.25 && box.bottom <= bottom + 0.25;
    });
}

export async function checkIslandTourPageButtons({ mount, click, settle, check, results, messages }: Tour) {
  const locale = new URLSearchParams(location.search).get("lang") === "en" ? "en" : "zh-CN";
  const t = TRANSLATIONS[locale];
  const { todo, music, clipboard, shelf, translation, system } = t.island.pageNames;
  const expected = [t.island.agentPage, todo, music, clipboard, shelf, translation, system, t.messages.title];

  try {
    await act(async () => { mount(); });
    await settle(300);
    await click(".island__open-button");
    await click(".island__agent-button");
    const panel = document.querySelector<HTMLElement>(".island__expanded")!;
    const row = panel.querySelector<HTMLElement>(".editor-dots--icon-label")!;
    const actions = panel.querySelector(".window-actions")!;
    let selected = 0;
    const expectedVisible = (mode: string | undefined) => mode === "current" ? [expected[selected]]
      : mode === "icons" ? [] : expected;
    const verifyAdaptiveRow = (message: string) => {
      const rowBox = row.getBoundingClientRect();
      const actionBox = actions.getBoundingClientRect();
      const mode = row.dataset.pageLabelMode;
      const detail = [...row.querySelectorAll<HTMLElement>("[data-page-button]")].map((button) => {
        const text = button.querySelector<HTMLElement>(".page-button-content__text");
        return `${button.dataset.pageButton}:${Math.round(button.getBoundingClientRect().width)}/${text ? `${text.clientWidth}:${text.scrollWidth}` : "-"}`;
      }).join(",");
      check(visibleNamesFit(row, expectedVisible(mode)) && rowFits(row, actions),
        `${locale} ${message}（${mode}，${row.clientWidth}px；行 ${Math.round(rowBox.left)}..${Math.round(rowBox.right)}，操作 ${Math.round(actionBox.left)}；${detail}）`);
    };

    check(document.querySelector<HTMLElement>(".island")!.offsetWidth === 560 && panel.offsetWidth === 560
      && actions.querySelectorAll(".icon-button").length === 3, "560px 默认展开岛的 Agent 顶栏含三个操作按钮");
    verifyAdaptiveRow("默认宽度下按空间自适应显示页名，文字和整行均不溢出");

    await click(".dot-button--clipboard");
    selected = 3;
    verifyAdaptiveRow("切到剪贴板后页名仍按空间自适应且不溢出");
    await click(".island-settings-button");
    await click(".settings-sidebar__item[data-category='appearance']");
    const preview = panel.querySelector<HTMLElement>(".agent-page-preview__dots--icon-label")!;
    const previewMode = preview.dataset.pageLabelMode;
    // The preview always draws the Agent page as current, whichever island page the tour is on.
    const previewVisible = previewMode === "current" ? [expected[0]]
      : previewMode === "icons" ? [] : expected;
    check(visibleNamesFit(preview, previewVisible) && rowFits(preview, preview.nextElementSibling!),
      `${locale} 外观预览的页名按空间自适应且不溢出（${previewMode}）`);

    await click(".island__agent-button");
    selected = 0;
    panel.style.setProperty("--island-font-scale", "1.25");
    panel.style.width = "350px";
    await settle(100);
    check(row.dataset.pageLabelMode === "current" && visibleNames(row).map((text) => text.textContent).join() === t.island.agentPage
      && rowFits(row, actions), `${locale} 大字号和极窄顶栏仅保留当前 Agent 页名`);
    panel.style.width = "290px";
    await settle(100);
    check(row.dataset.pageLabelMode === "icons" && visibleNames(row).length === 0 && rowFits(row, actions)
      && row.querySelectorAll("[data-page-button] svg").length === 8,
    `${locale} 继续缩窄后八个图标都保留`);
    panel.style.removeProperty("--island-font-scale");
    panel.style.removeProperty("width");
    await settle(100);
    verifyAdaptiveRow("恢复默认宽度与字号后页名按空间自适应恢复");

    results.textContent = `${messages.join("\n")}\n\n全部 ${messages.length} 项检查通过。`;
    results.dataset.result = "passed";
  } catch (error) {
    results.textContent = `${messages.join("\n")}\nFAIL ${String(error)}`;
    results.dataset.result = "failed";
  }
}

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/** 选中工具后 Rust（`palette_open_tool`）发往主窗口的导航事件；
 * text 为斜杠命令携带的参数（如 /t 的待译文本），由主窗口转交目标工具页。 */
const OPEN_TOOL_EVENT = "open-tool";

/** 面板呼起事件（Rust `palette::RESET_EVENT`，无 payload）：每次召起都是新起点。 */
const RESET_EVENT = "palette-reset";

/** 剪贴板条目回填事件（Rust `palette::INSERT_TEXT_EVENT`），payload 为条目文本。 */
const INSERT_TEXT_EVENT = "palette-insert-text";

export interface OpenToolPayload {
  toolId: string;
  text?: string | null;
}

/** 打开工具，可选携带文本（斜杠命令直达）：Rust 侧显示主窗口、发导航事件并收起面板。 */
export const openToolWithText = (toolId: string, text?: string): Promise<void> =>
  invoke("palette_open_tool", { toolId, text: text ?? null });

/** 命令面板选中工具：Rust 侧显示主窗口、发导航事件并收起面板。 */
export const openTool = (toolId: string): Promise<void> => openToolWithText(toolId);

/** 主窗口订阅工具打开事件（切到对应工具页），返回取消订阅。 */
export const onOpenTool = (handler: (payload: OpenToolPayload) => void): Promise<() => void> =>
  listen<OpenToolPayload>(OPEN_TOOL_EVENT, (event) => handler(event.payload));

/** 面板订阅呼起事件（清空上次查询），返回取消订阅。 */
export const onPaletteReset = (handler: () => void): Promise<() => void> =>
  listen(RESET_EVENT, () => handler());

/** 面板订阅剪贴板条目回填（文本插入输入框），返回取消订阅。 */
export const onPaletteInsertText = (handler: (text: string) => void): Promise<() => void> =>
  listen<string>(INSERT_TEXT_EVENT, (event) => handler(event.payload));

/** 在 query 的 [start, end) 选区处插入 text，返回新查询与插入后的光标位置；
 * 下标缺失按追加到末尾、越界或为负按字符串长度截住。 */
export const insertAtSelection = (
  query: string,
  text: string,
  start: number | null,
  end: number | null,
): { query: string; caret: number } => {
  const clamp = (at: number | null): number =>
    Math.min(Math.max(at ?? query.length, 0), query.length);
  const from = clamp(start);
  const to = clamp(end);
  const [begin, finish] = from <= to ? [from, to] : [to, from];
  const next = `${query.slice(0, begin)}${text}${query.slice(finish)}`;
  return { query: next, caret: Math.min(begin + text.length, next.length) };
};

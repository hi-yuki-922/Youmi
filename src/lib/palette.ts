import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/** 选中工具后 Rust（`palette_open_tool`）发往主窗口的导航事件，payload 为工具 id。 */
const OPEN_TOOL_EVENT = "open-tool";

/** 命令面板选中工具：Rust 侧显示主窗口、发导航事件并收起面板。 */
export const openTool = (toolId: string): Promise<void> => invoke("palette_open_tool", { toolId });

/** 主窗口订阅工具打开事件（切到对应工具页），返回取消订阅。 */
export const onOpenTool = (handler: (toolId: string) => void): Promise<() => void> =>
  listen<string>(OPEN_TOOL_EVENT, (event) => handler(event.payload));

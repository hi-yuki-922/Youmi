/**
 * 浮层窗口能力的前端边界：固定浮层。
 */

import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

/** 固定/取消固定当前浮层：固定后不因失焦自动隐藏。 */
export const setOverlayPinned = (pinned: boolean): Promise<void> =>
  invoke("overlay_set_pinned", { label: getCurrentWindow().label, pinned });

import { invoke } from "@tauri-apps/api/core";

import { getSetting, setSetting } from "@/lib/settings";

/** 剪贴板条目类型（与 Rust `ClipboardKind` serde 小写序列化对应）。 */
export type ClipboardKind = "text" | "image";

/** 查询结果条目（Rust `ClipboardEntry` camelCase 序列化），不含图片字节。 */
export interface ClipboardEntry {
  id: number;
  kind: ClipboardKind;
  text: string | null;
  hasImage: boolean;
  createdAt: number;
  updatedAt: number;
}

export const CLIPBOARD_CAPACITY_DEFAULT = 1000;
const CLIPBOARD_CAPACITY_KEY = "clipboard.capacity";
const CLIPBOARD_CAPACITY_MAX = 1_000_000;

/** 剪贴板引擎是否已初始化；false 时历史与容量设置均不生效。 */
export const getClipboardEngineAvailable = (): Promise<boolean> =>
  invoke<boolean>("clipboard_engine_available");

/** 持久化设置中的历史容量；未设置过时返回默认值。 */
export const getClipboardCapacity = (): Promise<number> =>
  getSetting(CLIPBOARD_CAPACITY_KEY, CLIPBOARD_CAPACITY_DEFAULT);

/** 修改历史容量并立即生效（引擎即时淘汰），同时持久化。 */
export const setClipboardCapacity = async (capacity: number): Promise<void> => {
  const clamped = Math.min(Math.max(Math.floor(capacity), 1), CLIPBOARD_CAPACITY_MAX);
  await invoke("clipboard_set_capacity", { capacity: clamped });
  await setSetting(CLIPBOARD_CAPACITY_KEY, clamped);
};

/** 启动时把持久化容量应用到引擎（幂等，不回写设置）。 */
export const applyClipboardCapacity = async (): Promise<void> => {
  const capacity = await getClipboardCapacity();
  await invoke("clipboard_set_capacity", { capacity });
};

export const queryClipboardEntries = (params: {
  keyword?: string;
  kind?: ClipboardKind;
  limit?: number;
}): Promise<ClipboardEntry[]> => invoke<ClipboardEntry[]>("clipboard_query", params);

/** 条目图片的 PNG base64；文本条目或已删除返回 null。 */
export const getClipboardImage = (id: number): Promise<string | null> =>
  invoke<string | null>("clipboard_get_image", { id });

export const deleteClipboardEntry = (id: number): Promise<boolean> =>
  invoke<boolean>("clipboard_delete", { id });

/** 清空全部历史，返回删除的条数；不影响容量设置。 */
export const clearClipboardHistory = (): Promise<number> => invoke<number>("clipboard_clear");

/** 用户侧复制通道（工具页复制按钮等）：写文本并让监听器照常捕获入史。 */
export const writeClipboardText = (text: string): Promise<void> =>
  invoke<void>("clipboard_write_text", { text });

/** 自家写入通道（密码复制、划词模拟复制等）：写剪贴板但不进入历史。
 * 与 `writeClipboardText` 构成仅有的两条前端写路径。 */
export const writeOwnClipboardText = (text: string): Promise<void> =>
  invoke<void>("clipboard_write_own_text", { text });

/** 回帖结果（Rust `PastebackOutcome` kebab-case 序列化）。 */
export type PastebackOutcome = "pasted" | "copied-only";

/** 回帖：条目写剪贴板（不入史）→ 焦点交还呼出浮层前的窗口 → 模拟 Ctrl+V。
 * `copied-only` = 无有效目标或焦点/键击失败，条目留在剪贴板供手动粘贴。 */
export const pastebackClipboardEntry = (id: number): Promise<PastebackOutcome> =>
  invoke<PastebackOutcome>("clipboard_pasteback", { id });

/**
 * 全局快捷键的 tauri 边界：三个全局工具动作的当前绑定与改键。
 * 组合键为插件解析格式的规范串（如 "alt+KeyV"、"alt+super+Space"）：
 * 修饰键按 ctrl, alt, shift, super 排列，主键为 KeyboardEvent.code 命名，
 * 与 Rust 侧 combo_string 一致。
 */

import { invoke } from "@tauri-apps/api/core";

export type ShortcutAction = "clipboard" | "palette" | "translate";

/** 当前绑定（Rust `ShortcutBinding` camelCase 序列化）；registered = OS 注册是否生效。 */
export interface ShortcutBinding {
  action: ShortcutAction;
  combo: string;
  registered: boolean;
}

export const getShortcuts = (): Promise<ShortcutBinding[]> =>
  invoke<ShortcutBinding[]>("shortcut_list");

/**
 * 改绑动作的组合键并立即生效、持久化。校验失败（无 Ctrl/Alt/Win 修饰键）、
 * Youmi 内部冲突、系统级注册失败时 reject（错误消息可直接展示），原绑定不变。
 */
export const setShortcut = (action: ShortcutAction, combo: string): Promise<void> =>
  invoke("shortcut_set", { action, combo });

const MODIFIER_CODES = new Set([
  "ControlLeft",
  "ControlRight",
  "AltLeft",
  "AltRight",
  "MetaLeft",
  "MetaRight",
  "ShiftLeft",
  "ShiftRight",
]);

/**
 * keydown 事件 → 组合键规范串；纯修饰键按下（尚未出现主键）返回 null，
 * 调用方应继续等待。修饰键取自事件状态，主键用 KeyboardEvent.code 物理键位。
 */
export const eventToCombo = (event: KeyboardEvent): string | null => {
  if (MODIFIER_CODES.has(event.code)) return null;
  const parts: string[] = [];
  if (event.ctrlKey) parts.push("ctrl");
  if (event.altKey) parts.push("alt");
  if (event.shiftKey) parts.push("shift");
  if (event.metaKey) parts.push("super");
  parts.push(event.code);
  return parts.join("+");
};

/** 组合键是否满足修饰键约束：至少含 Ctrl / Alt / Win 之一（Shift 不算）。 */
export const hasRequiredModifier = (combo: string): boolean =>
  combo.split("+").some((part) => part === "ctrl" || part === "alt" || part === "super");

const KEY_LABELS: Record<string, string> = {
  Space: "Space",
  Enter: "Enter",
  Tab: "Tab",
  Backspace: "Backspace",
  Delete: "Del",
  Insert: "Ins",
  Home: "Home",
  End: "End",
  PageUp: "PgUp",
  PageDown: "PgDn",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  Semicolon: ";",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Backslash: "\\",
  Quote: "'",
  Backquote: "`",
  BracketLeft: "[",
  BracketRight: "]",
  Minus: "-",
  Equal: "=",
  PrintScreen: "PrtSc",
  ScrollLock: "ScrollLock",
  CapsLock: "CapsLock",
  NumLock: "NumLock",
  NumpadAdd: "Num+",
  NumpadSubtract: "Num-",
  NumpadMultiply: "Num*",
  NumpadDivide: "Num/",
  NumpadDecimal: "Num.",
  NumpadEnter: "NumEnter",
  NumpadEqual: "Num=",
};

const keyLabel = (code: string): string => {
  if (KEY_LABELS[code] !== undefined) return KEY_LABELS[code];
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  if (code.startsWith("Numpad")) return `Num${code.slice(6)}`;
  return code;
};

/** 组合键规范串 → 用户可读形式（如 "alt+KeyV" → "Alt+V"）。 */
export const formatCombo = (combo: string): string =>
  combo
    .split("+")
    .map((part) => {
      switch (part) {
        case "ctrl":
          return "Ctrl";
        case "alt":
          return "Alt";
        case "shift":
          return "Shift";
        case "super":
          return "Win";
        default:
          return keyLabel(part);
      }
    })
    .join("+");

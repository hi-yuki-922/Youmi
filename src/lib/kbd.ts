/**
 * 组合键的展示格式化：快捷键规范串（主键为 KeyboardEvent.code 命名，修饰键按
 * ctrl, alt, shift, super 排列）→ 人类可读的符号或文本 token。Kbd 组件与
 * aria/title 等纯文本场合共用。
 */

/** 修饰键词汇：Kbd 符号与文本形式各一份，Alt ⌘ 与 Win ⊞ 必须可区分。 */
const MODIFIERS: Record<string, { symbol: string; label: string }> = {
  ctrl: { symbol: "⌃", label: "Ctrl" },
  alt: { symbol: "⌘", label: "Alt" },
  shift: { symbol: "⇧", label: "Shift" },
  super: { symbol: "⊞", label: "Win" },
};

/** 该 token 是否为组合键规范串中的修饰键。 */
export const isModifier = (part: string): boolean => part in MODIFIERS;

/** 具名主键的符号；未列出的主键沿用 {@link keyLabel} 的文本形式。 */
const KEY_SYMBOLS: Record<string, string> = {
  Enter: "⏎",
  Escape: "⎋",
};

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

/**
 * 修饰键展示顺序：Win 在前（Win+X 类系统组合的书写惯例），其余保持规范序；
 * 展示形态与串内顺序无关，"alt+super+Space" 与 "super+alt+Space" 同形。
 */
const MODIFIER_DISPLAY_ORDER = ["super", "ctrl", "alt", "shift"] as const;

const splitCombo = (combo: string): { modifiers: string[]; key: string } => {
  const parts = combo.split("+");
  const modifiers = MODIFIER_DISPLAY_ORDER.filter((modifier) => parts.includes(modifier));
  const key = parts.find((part) => !isModifier(part)) ?? "";
  return { modifiers, key };
};

/** 组合键规范串 → Kbd 符号 token 序列（如 "alt+super+Space" → ["⊞", "⌘", "Space"]）。 */
export const comboSymbols = (combo: string): string[] => {
  const { modifiers, key } = splitCombo(combo);
  return [
    ...modifiers.map((modifier) => MODIFIERS[modifier].symbol),
    ...(key ? [KEY_SYMBOLS[key] ?? keyLabel(key)] : []),
  ];
};

/** 组合键规范串 → 文本形式（如 "alt+KeyV" → "Alt+V"），用于 aria-label/title 等场合。 */
export const formatCombo = (combo: string): string => {
  const { modifiers, key } = splitCombo(combo);
  return [...modifiers.map((modifier) => MODIFIERS[modifier].label), keyLabel(key)].join("+");
};

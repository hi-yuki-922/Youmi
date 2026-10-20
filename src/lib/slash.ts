import { openToolWithText } from "@/lib/palette";

/** 斜杠命令：命令面板的带参数直达动作，`/名称 参数` 形式（参数以首个空格分隔）。 */
export interface SlashCommand {
  /** 命令名（不含斜杠），小写；输入按名前缀过滤。 */
  name: string;
  /** 一句话描述：命令列表里展示。 */
  description: string;
  /** 用法提示：形如 "/t <文本>"。 */
  usage: string;
  /** 执行命令；args 为首个空格之后的原文。 */
  run: (args: string) => void;
}

export const SLASH_COMMANDS: SlashCommand[] = [
  {
    name: "t",
    description: "翻译：打开翻译页填入文本并立即翻译",
    usage: "/t <文本>",
    run: (args) => {
      const text = args.trim();
      if (text) openToolWithText("translate", text);
      else openToolWithText("translate");
    },
  },
];

/** 面板查询的斜杠解析结果：none 之外的形态由面板专路渲染，不混常规搜索。 */
export type SlashQuery =
  | { mode: "none" }
  | { mode: "list"; matches: SlashCommand[] }
  | { mode: "execute"; command: SlashCommand; args: string }
  | { mode: "unknown"; name: string };

/** 解析面板查询：以 / 开头进入斜杠模式。无空格即命令名前缀过滤（"/" 列出全部）；
 * 有空格则首个空格前的名字必须精确命中一个命令，其余全是参数。 */
export const parseSlashQuery = (query: string, commands: SlashCommand[]): SlashQuery => {
  if (!query.startsWith("/")) return { mode: "none" };
  const rest = query.slice(1);
  const spaceIndex = rest.indexOf(" ");
  if (spaceIndex < 0) {
    const matches = commands.filter((command) => command.name.startsWith(rest));
    return matches.length > 0 ? { mode: "list", matches } : { mode: "unknown", name: rest };
  }
  const name = rest.slice(0, spaceIndex);
  const command = commands.find((entry) => entry.name === name);
  return command
    ? { mode: "execute", command, args: rest.slice(spaceIndex + 1) }
    : { mode: "unknown", name };
};

/**
 * 斜杠命令携带的文本跨窗口送达：命令面板是独立 webview，文本经 Rust 事件
 * 到主窗口布局层后，再经此通道交给目标工具页。
 * 页面已挂载 → 监听直送；未挂载（刚切路由）→ 暂存，挂载时取走。
 */
const pendingTexts = new Map<string, string>();
const textListeners = new Map<string, (text: string) => void>();

export const deliverToolText = (toolId: string, text: string): void => {
  const listener = textListeners.get(toolId);
  if (listener) {
    listener(text);
    return;
  }
  pendingTexts.set(toolId, text);
};

/** 取走并清掉暂存文本（若监听直送已发生则此处为空）。 */
export const takePendingToolText = (toolId: string): string | undefined => {
  const text = pendingTexts.get(toolId);
  pendingTexts.delete(toolId);
  return text;
};

export const subscribeToolText = (
  toolId: string,
  listener: (text: string) => void,
): (() => void) => {
  textListeners.set(toolId, listener);
  return () => {
    textListeners.delete(toolId);
  };
};

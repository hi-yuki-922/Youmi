import {
  Binary,
  Braces,
  CircleDollarSign,
  GitCompare,
  KeyRound,
  Languages,
  Settings,
} from "lucide-react";

import { fuzzyScore } from "@/lib/fuzzy";

/** 工作台工具与设置页的唯一清单：侧边导航、命令面板搜索、跨窗口跳转共用。 */
export const TOOLS = [
  {
    id: "encoding",
    path: "/encoding",
    label: "编码转换",
    icon: Binary,
    keywords: ["base64", "svg", "url", "query", "编码"],
  },
  {
    id: "json",
    path: "/json",
    label: "JSON 工具",
    icon: Braces,
    keywords: ["json", "格式化", "校验", "format"],
  },
  {
    id: "diff",
    path: "/diff",
    label: "文本 diff",
    icon: GitCompare,
    keywords: ["diff", "对比", "比较"],
  },
  {
    id: "translate",
    path: "/translate",
    label: "翻译",
    icon: Languages,
    keywords: ["translate", "translation", "deepseek", "中英互译"],
  },
  {
    id: "exchange",
    path: "/exchange",
    label: "汇率",
    icon: CircleDollarSign,
    keywords: ["exchange", "rate", "currency", "汇率", "换算", "货币"],
  },
  {
    id: "password-book",
    path: "/password-book",
    label: "密码本",
    icon: KeyRound,
    keywords: ["password", "凭据", "密码"],
  },
  {
    id: "settings",
    path: "/settings",
    label: "设置",
    icon: Settings,
    keywords: ["开机启动", "自启动", "设置"],
  },
] as const;

export type ToolEntry = (typeof TOOLS)[number];

/** 模糊搜索工具：label 与 keywords 取最高分排序，无命中的剔除，空查询返回全部。 */
export const searchTools = (query: string): ToolEntry[] => {
  const scored = TOOLS.map((tool) => ({
    tool,
    score: Math.max(
      fuzzyScore(query, tool.label) ?? -1,
      ...tool.keywords.map((keyword) => fuzzyScore(query, keyword) ?? -1),
    ),
  })).filter((entry) => entry.score >= 0);
  // Array.prototype.sort 同分保序，空查询时维持注册顺序
  scored.sort((a, b) => b.score - a.score);
  return scored.map((entry) => entry.tool);
};

import { createFileRoute } from "@tanstack/react-router";
import { Calculator, Search } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { Input } from "@/components/ui/Input";
import { writeOwnClipboardText } from "@/lib/clipboard";
import { openTool } from "@/lib/palette";
import { searchTools, type ToolEntry } from "@/lib/tools";
import { cn } from "@/lib/utils";
import { evaluate, formatResult, isCalculatable } from "@/tools/calculator/core";

/** 面板条目：算式结果优先于工具命中。 */
type PaletteItem = { kind: "calc"; value: number } | { kind: "tool"; tool: ToolEntry };

const PaletteOverlay = () => {
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const searchRef = useRef<HTMLInputElement>(null);

  // 浮层 hide 而非销毁：重新呼起时清空上次查询并找回焦点，每次召起都是新起点。
  // WebView2 隐藏窗口不派发 visibilitychange（实测），window focus 是唯一可靠的唤起信号。
  useEffect(() => {
    searchRef.current?.focus();
    const onWindowFocus = () => {
      setQuery("");
      setSelected(0);
      searchRef.current?.focus();
    };
    window.addEventListener("focus", onWindowFocus);
    return () => window.removeEventListener("focus", onWindowFocus);
  }, []);

  const calc = isCalculatable(query) ? evaluate(query) : null;
  const trimmedQuery = query.trim();
  const tools = useMemo(() => searchTools(trimmedQuery), [trimmedQuery]);

  const items = useMemo<PaletteItem[]>(() => {
    const list: PaletteItem[] = [];
    if (calc?.ok) list.push({ kind: "calc", value: calc.value });
    for (const tool of tools) list.push({ kind: "tool", tool });
    return list;
  }, [calc, tools]);

  // 条目数随输入收缩，选中下标越界时按尾项处理
  const activeIndex = Math.min(selected, items.length - 1);

  const hide = () => {
    void getCurrentWindow().hide();
  };

  const activate = (item: PaletteItem) => {
    if (item.kind === "calc") {
      // 复制成功即收起，粘贴目标就是呼起前的窗口；失败保留浮层供重试
      writeOwnClipboardText(formatResult(item.value))
        .then(hide)
        .catch((error) => console.error("复制计算结果失败", error));
    } else {
      openTool(item.tool.id).catch((error) => console.error("打开工具失败", error));
    }
  };

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background text-sm">
      <div className="flex items-center gap-2 border-b px-3 py-2.5">
        <div className="relative min-w-0 flex-1">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            ref={searchRef}
            role="combobox"
            aria-controls="palette-list"
            aria-activedescendant={items[activeIndex] ? `palette-item-${activeIndex}` : undefined}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "ArrowDown") {
                e.preventDefault();
                setSelected(Math.min(activeIndex + 1, items.length - 1));
              } else if (e.key === "ArrowUp") {
                e.preventDefault();
                setSelected(Math.max(activeIndex - 1, 0));
              } else if (e.key === "Enter") {
                e.preventDefault();
                const item = items[activeIndex];
                if (item) activate(item);
              } else if (e.key === "Escape") {
                e.preventDefault();
                hide();
              }
            }}
            placeholder="搜索工具，或直接输入算式…"
            className="h-8 pl-8"
          />
        </div>
        {calc && !calc.ok && (
          <span className="shrink-0 text-xs text-muted-foreground">{calc.message}</span>
        )}
      </div>

      {items.length === 0 ? (
        <p className="p-4 text-muted-foreground">没有匹配「{trimmedQuery}」的工具</p>
      ) : (
        <ul
          id="palette-list"
          role="listbox"
          aria-label="命令面板结果"
          className="flex-1 overflow-y-auto py-1"
        >
          {items.map((item, index) => (
            <PaletteRow
              key={item.kind === "calc" ? "calc" : item.tool.id}
              item={item}
              active={index === activeIndex}
              onHover={() => setSelected(index)}
              onActivate={() => activate(item)}
            />
          ))}
        </ul>
      )}

      <p className="border-t px-4 py-1.5 text-xs text-muted-foreground">
        ↑↓ 选择 · 回车 打开工具或复制结果 · Esc 关闭
      </p>
    </div>
  );
};

const PaletteRow = ({
  item,
  active,
  onHover,
  onActivate,
}: {
  item: PaletteItem;
  active: boolean;
  onHover: () => void;
  onActivate: () => void;
}) => {
  return (
    <li
      role="option"
      aria-selected={active}
      onMouseEnter={onHover}
      onClick={onActivate}
      className={cn("flex cursor-default items-center gap-3 px-4 py-2.5", active && "bg-accent/70")}
    >
      {item.kind === "calc" ? (
        <>
          <Calculator className="size-4 shrink-0 text-muted-foreground" />
          <span className="min-w-0 flex-1 truncate font-mono">= {formatResult(item.value)}</span>
        </>
      ) : (
        <>
          <item.tool.icon className="size-4 shrink-0 text-muted-foreground" />
          <span className="min-w-0 flex-1 truncate">{item.tool.label}</span>
        </>
      )}
      {active && (
        <span className="shrink-0 text-xs text-muted-foreground">
          {item.kind === "calc" ? "回车复制" : "回车打开"}
        </span>
      )}
    </li>
  );
};

export const Route = createFileRoute("/overlay/palette")({
  component: PaletteOverlay,
});

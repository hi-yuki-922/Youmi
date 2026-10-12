import { createFileRoute } from "@tanstack/react-router";
import { AppWindow, Calculator, Search } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { Input } from "@/components/ui/Input";
import { launchApp, listApps, searchApps, type AppEntry } from "@/lib/apps";
import { writeOwnClipboardText } from "@/lib/clipboard";
import { openTool } from "@/lib/palette";
import { searchTools, type ToolEntry } from "@/lib/tools";
import { cn } from "@/lib/utils";
import { evaluate, formatResult, isCalculatable } from "@/tools/calculator/core";

/** 面板条目：算式结果优先于工具命中，应用命中殿后。 */
type PaletteItem =
  | { kind: "calc"; value: number }
  | { kind: "tool"; tool: ToolEntry }
  | { kind: "app"; app: AppEntry };

/** 列表项 React key：条目内唯一即可。 */
const itemKey = (item: PaletteItem): string =>
  item.kind === "calc" ? "calc" : item.kind === "tool" ? item.tool.id : item.app.path;

const PaletteOverlay = () => {
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const [apps, setApps] = useState<AppEntry[]>([]);
  const [appsLoading, setAppsLoading] = useState(false);
  const searchRef = useRef<HTMLInputElement>(null);

  const refreshApps = () => {
    // Rust 侧后台线程维护应用缓存；拉取失败按"索引中"处理，由轮询重试自愈
    listApps()
      .then((snapshot) => {
        setApps(snapshot.apps);
        setAppsLoading(snapshot.loading);
      })
      .catch((error) => {
        console.error("获取应用列表失败", error);
        setAppsLoading(true);
      });
  };

  // 浮层 hide 而非销毁：重新呼起时清空上次查询、刷新应用列表并找回焦点，每次召起都是新起点。
  // WebView2 隐藏窗口不派发 visibilitychange（实测），window focus 是唯一可靠的唤起信号。
  useEffect(() => {
    refreshApps();
    searchRef.current?.focus();
    const onWindowFocus = () => {
      setQuery("");
      setSelected(0);
      refreshApps();
      searchRef.current?.focus();
    };
    window.addEventListener("focus", onWindowFocus);
    return () => window.removeEventListener("focus", onWindowFocus);
  }, []);

  // 首扫进行中（或拉取失败）时每秒重试，完成后自动停止——面板停留期间空态自愈
  useEffect(() => {
    if (!appsLoading) return;
    const timer = setTimeout(refreshApps, 1000);
    return () => clearTimeout(timer);
  }, [appsLoading]);

  const calc = isCalculatable(query) ? evaluate(query) : null;
  const trimmedQuery = query.trim();
  const tools = useMemo(() => searchTools(trimmedQuery), [trimmedQuery]);
  const appHits = useMemo(() => searchApps(apps, trimmedQuery), [apps, trimmedQuery]);

  const items = useMemo<PaletteItem[]>(() => {
    const list: PaletteItem[] = [];
    if (calc?.ok) list.push({ kind: "calc", value: calc.value });
    for (const tool of tools) list.push({ kind: "tool", tool });
    for (const app of appHits) list.push({ kind: "app", app });
    return list;
  }, [calc, tools, appHits]);

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
    } else if (item.kind === "tool") {
      openTool(item.tool.id).catch((error) => console.error("打开工具失败", error));
    } else {
      // 启动成功即收起，焦点交还呼起前的窗口；失败保留浮层供重试
      launchApp(item.app.path)
        .then(hide)
        .catch((error) => console.error("启动应用失败", error));
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
            placeholder="搜索工具、应用，或直接输入算式…"
            className="h-8 pl-8"
          />
        </div>
        {calc && !calc.ok && (
          <span className="shrink-0 text-xs text-muted-foreground">{calc.message}</span>
        )}
      </div>

      {items.length === 0 ? (
        <p className="p-4 text-muted-foreground">
          {appsLoading
            ? "正在索引开始菜单应用，稍候再试…"
            : `没有匹配「${trimmedQuery}」的工具或应用`}
        </p>
      ) : (
        <ul
          id="palette-list"
          role="listbox"
          aria-label="命令面板结果"
          className="flex-1 overflow-y-auto py-1"
        >
          {items.map((item, index) => (
            <PaletteRow
              key={itemKey(item)}
              item={item}
              active={index === activeIndex}
              onHover={() => setSelected(index)}
              onActivate={() => activate(item)}
            />
          ))}
        </ul>
      )}

      <p className="border-t px-4 py-1.5 text-xs text-muted-foreground">
        ↑↓ 选择 · 回车 打开工具、启动应用或复制结果 · Esc 关闭
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
  const hint = item.kind === "calc" ? "回车复制" : item.kind === "tool" ? "回车打开" : "回车启动";
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
      ) : item.kind === "tool" ? (
        <>
          <item.tool.icon className="size-4 shrink-0 text-muted-foreground" />
          <span className="min-w-0 flex-1 truncate">{item.tool.label}</span>
        </>
      ) : (
        <>
          <AppWindow className="size-4 shrink-0 text-muted-foreground" />
          <span className="min-w-0 flex-1 truncate">{item.app.name}</span>
        </>
      )}
      {active && <span className="shrink-0 text-xs text-muted-foreground">{hint}</span>}
    </li>
  );
};

export const Route = createFileRoute("/overlay/palette")({
  component: PaletteOverlay,
});

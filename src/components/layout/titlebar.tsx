import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Minus, Square, Wrench, X } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";

import { cn } from "@/lib/utils";

/**
 * 自绘标题栏（ADR-0004）：拖拽区 + 窗口控制按钮。
 * 拖拽与双击最大化由 `data-tauri-drag-region` 驱动（tauri 内置脚本），
 * 按钮是可点击元素、天然不触发拖拽；关闭按钮发起 close 请求，
 * 由 Rust 端 CloseRequested 统一处理为隐藏到托盘，保持单一语义路径。
 */
export function TitleBar() {
  const maximized = useIsMaximized();
  const appWindow = getCurrentWindow();

  return (
    <header
      data-tauri-drag-region="deep"
      className="flex h-10 shrink-0 select-none items-center justify-between pl-4"
    >
      <div className="flex items-center gap-2">
        <Wrench className="size-4" />
        <span className="text-sm font-semibold">Youmi</span>
      </div>
      <div className="flex h-full items-stretch">
        <WindowButton label="最小化" onClick={() => void appWindow.minimize()}>
          <Minus className="size-3.5" />
        </WindowButton>
        <WindowButton
          label={maximized ? "还原" : "最大化"}
          onClick={() => void appWindow.toggleMaximize()}
        >
          {maximized ? <Copy className="size-3" /> : <Square className="size-3" />}
        </WindowButton>
        <WindowButton label="关闭" destructive onClick={() => void appWindow.close()}>
          <X className="size-4" />
        </WindowButton>
      </div>
    </header>
  );
}

/** 最大化状态驱动按钮图标；拖拽缩放会连续触发 resized，逐次同步即可。 */
function useIsMaximized() {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    const appWindow = getCurrentWindow();
    let active = true;
    const sync = () =>
      void appWindow.isMaximized().then((value) => {
        if (active) setMaximized(value);
      });
    sync();
    const unlisten = appWindow.onResized(sync);
    return () => {
      active = false;
      void unlisten.then((dispose) => dispose());
    };
  }, []);

  return maximized;
}

function WindowButton({
  label,
  destructive = false,
  onClick,
  children,
}: {
  label: string;
  destructive?: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      onClick={onClick}
      className={cn(
        "flex w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground",
        destructive && "hover:bg-destructive hover:text-destructive-foreground",
      )}
    >
      {children}
    </button>
  );
}

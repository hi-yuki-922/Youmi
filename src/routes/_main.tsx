import { createFileRoute, Outlet } from "@tanstack/react-router";
import { useEffect } from "react";

import { Sidebar } from "@/components/layout/Sidebar";
import { TitleBar } from "@/components/layout/TitleBar";
import { applyClipboardCapacity } from "@/lib/clipboard";
import { onOpenTool } from "@/lib/palette";
import { TOOLS } from "@/lib/tools";
import { applyVaultLockTimeout } from "@/lib/vault";
import { router } from "@/Router";

const MainLayout = () => {
  useEffect(() => {
    // 启动即把持久化的配置应用到 Rust 侧：生效不依赖用户访问设置页。
    // 引擎初始化失败时对应 command 会 reject，告警即可，不阻塞界面
    void applyClipboardCapacity().catch((cause) => {
      console.warn("启动应用剪贴板历史容量失败（剪贴板历史可能不可用）", cause);
    });
    void applyVaultLockTimeout().catch((cause) => {
      console.warn("启动应用密码本锁定超时失败（密码本可能不可用）", cause);
    });
  }, []);

  useEffect(() => {
    // 命令面板直达：主窗口隐藏时 webview 仍存活、监听有效，收到工具 id 即切路由；
    // 显示与聚焦由 Rust 侧 palette_open_tool 完成
    const unlisten = onOpenTool((toolId) => {
      const tool = TOOLS.find((entry) => entry.id === toolId);
      if (tool) void router.navigate({ to: tool.path });
    });
    return () => {
      void unlisten.then((off) => off());
    };
  }, []);

  return (
    // 双卡片嵌套（ADR-0004）：窗口背景层（surface-base）承载标题栏与侧边导航，
    // 主内容区是浅一档的卡片（surface-raised）叠于其上，左右下三方留出背景层的边。
    <div className="flex h-screen flex-col overflow-hidden bg-surface-base">
      <TitleBar />
      <div className="flex min-h-0 flex-1">
        <Sidebar />
        <main className="min-w-0 flex-1 px-2 pb-2">
          <div className="h-full overflow-y-auto rounded-lg border bg-card shadow-xs">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  );
};

export const Route = createFileRoute("/_main")({
  component: MainLayout,
});

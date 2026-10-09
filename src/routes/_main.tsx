import { createFileRoute, Outlet } from "@tanstack/react-router";
import { useEffect } from "react";

import { Sidebar } from "@/components/layout/sidebar";
import { applyClipboardCapacity } from "@/lib/clipboard";

export const Route = createFileRoute("/_main")({
  component: MainLayout,
});

function MainLayout() {
  useEffect(() => {
    // 启动即把持久化的历史容量应用到引擎：容量生效不依赖用户访问设置页
    void applyClipboardCapacity();
  }, []);

  return (
    <div className="flex h-screen overflow-hidden">
      <Sidebar />
      <main className="flex-1 overflow-y-auto">
        <Outlet />
      </main>
    </div>
  );
}

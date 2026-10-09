import { createRootRoute, Outlet } from "@tanstack/react-router";

// 根路由不带布局：主窗口页面在 pathless 布局 `_main`（带 Sidebar）下，
// 浮层窗口路由（/overlay/*）直接挂在根上，不带 Sidebar。
export const Route = createRootRoute({
  component: () => <Outlet />,
});

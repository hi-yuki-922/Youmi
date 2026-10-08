import { createRootRoute, Outlet } from "@tanstack/react-router";

import { Sidebar } from "@/components/layout/sidebar";

export const Route = createRootRoute({
  component: () => (
    <div className="flex h-screen overflow-hidden">
      <Sidebar />
      <main className="flex-1 overflow-y-auto">
        <Outlet />
      </main>
    </div>
  ),
});

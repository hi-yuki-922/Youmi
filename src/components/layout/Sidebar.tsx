import { Link } from "@tanstack/react-router";

import { TOOLS } from "@/lib/tools";

// 侧边导航置于窗口背景层（ADR-0004）：不铺卡片底色，与内容卡片靠层次区分。
// 工具清单来自 TOOLS 注册表：与命令面板搜索、跨窗口跳转共用同一来源。
export const Sidebar = () => {
  return (
    <aside className="flex h-full w-52 shrink-0 flex-col">
      <nav className="flex flex-1 flex-col gap-1 px-2 pt-3">
        {TOOLS.map(({ path, label, icon: Icon }) => (
          <Link
            key={path}
            to={path}
            activeOptions={{ exact: true }}
            className="flex items-center gap-2 rounded-md px-3 py-2 text-sm text-muted-foreground hover:bg-accent hover:text-accent-foreground"
            activeProps={{
              className: "bg-accent font-medium text-accent-foreground",
            }}
          >
            <Icon className="size-4" />
            {label}
          </Link>
        ))}
      </nav>
    </aside>
  );
};

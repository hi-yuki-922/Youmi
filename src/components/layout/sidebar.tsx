import { Link } from "@tanstack/react-router";
import { Binary, Braces, GitCompare, KeyRound, Settings, Wrench } from "lucide-react";

const NAV_ITEMS = [
  { to: "/encoding", label: "编码转换", icon: Binary },
  { to: "/json", label: "JSON 工具", icon: Braces },
  { to: "/diff", label: "文本 diff", icon: GitCompare },
  { to: "/password-book", label: "密码本", icon: KeyRound },
  { to: "/settings", label: "设置", icon: Settings },
] as const;

export function Sidebar() {
  return (
    <aside className="flex h-full w-52 shrink-0 flex-col border-r bg-card">
      <div className="flex items-center gap-2 px-4 py-4">
        <Wrench className="size-5" />
        <span className="text-base font-semibold">Youmi</span>
      </div>
      <nav className="flex flex-1 flex-col gap-1 px-2 pt-1">
        {NAV_ITEMS.map(({ to, label, icon: Icon }) => (
          <Link
            key={to}
            to={to}
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
}

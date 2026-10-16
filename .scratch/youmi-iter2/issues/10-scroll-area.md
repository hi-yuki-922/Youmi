# 10 — 主窗口 ScrollArea 滚动容器

**What to build:** 主窗口布局的共享内容卡片容器替换为 shadcn ScrollArea（radix）：设置页高度不足时以 ScrollArea 滚动，所有工具页滚动条样式统一，不使用原生滚动条样式，且无双重滚动。

**Blocked by:** 无——可立即开始

**Status:** ready-for-human（2026-10-16 实现完成；滚动样式与无双重滚动需实机目检）

- [ ] 设置页内容超高时在 ScrollArea 内滚动，无原生滚动条
- [ ] 其他工具页滚动样式一致
- [ ] 无双重滚动（内层不出现第二套滚动）
- [ ] 键盘/滚轮滚动行为正常

## Comments

### 2026-10-16 — 实现

- **ScrollArea**（`src/components/ui/ScrollArea.tsx`，radix `@radix-ui/react-scroll-area@1.4.0`）：Viewport 上 `disableImplicitContentElement`——radix 默认会再包一层 display:table 的隐式 content div（auto 高度，工具页的 `h-full` 无法解析、滚动测量也落在错误层）；显式 Content 用 `style={{ display: "block" }}` 覆盖 radix 内联的 table 并 `grow`——内容不足时撑满视口（h-full 页面照常布局），超高时随内容增长、滚动归 viewport，thumb 尺寸测量正确。
- **接入**：`/_main` 布局的共享内容卡片容器（原 `overflow-y-auto` div）替换为 `<ScrollArea className="h-full rounded-lg border bg-card shadow-xs">`，主窗口全部工具页滚动条样式统一；radix 默认 hover 浮现滚动条 + 隐藏原生滚动条。
- **范围控制**：密码本条目列表、query 键值表、diff 视图等页面内部滚动属于内容级滚动（spec Out of Scope 明确"设置页之外的逐页 ScrollArea 定制"不做），保持现状。
- 实机验收注意：设置页超高滚动、无原生滚动条、无双重滚动、滚轮/键盘滚动均需目检（ticket 14 收口）。

# 17 — 自定义标题栏 + 双卡片嵌套布局

**What to build:** 主窗口去原生边框（ADR-0004）：自绘标题栏承载可拖拽区（双击最大化/还原）与最小化/最大化/关闭，关闭=隐藏到托盘（进程不退出）；窗口缩放仍可用。内容区采用双卡片嵌套：窗口背景层为较深的灰，侧边导航置于背景层，主内容区为浅一档的卡片叠于其上；深浅两主题各自取对应值。浮层窗口不受影响。原生 Snap Layouts 在 v1 失效，属已接受取舍。

**Blocked by:** 16 — design token 体系（两层 surface 先进入 token）

**Status:** done（2026-10-10）

- [x] 主窗口无原生边框，标题栏可拖拽移动、双击最大化/还原
- [x] 最小化/最大化/关闭自绘可用；关闭=隐藏到托盘，托盘可恢复主窗口
- [x] 无边框后拖拽窗口边缘可正常缩放
- [x] 双卡片层次在浅/深两主题下成立：背景层、侧边导航、内容卡片层次分明
- [x] 浮层窗口呼出/失焦隐藏行为不受影响

## Comments

- 窗口配置：主窗口 `decorations: false`（tauri.conf.json），`resizable` 保持默认 true——tao 的无边框窗口机制在 Windows 上保留边缘缩放 hit-test，无需自绘缩放；双击最大化由 tauri 内置 drag 脚本经 `data-tauri-drag-region` 驱动（`internal_toggle_maximize` 在 `core:window:default` 内），拖拽的 `start_dragging` 不在 default，需显式授权。
- TitleBar（`src/components/layout/titlebar.tsx`）：拖拽区用 `data-tauri-drag-region="deep"`（tauri 2.12.1 drag.js 支持，子树内非可点击元素都触发拖拽；`<button>` 无该属性天然阻断，故三个控制按钮无需处理）；`<button>` 是可点击元素天然阻断拖拽，无需逐个豁免。最大化/还原图标由 `useIsMaximized` 驱动（初始 `isMaximized()` + `onResized` 逐次同步，仓库无 window-state 持久化，启动必为还原态）。关闭按钮 `close()` 发起请求，由 Rust 端 CloseRequested 统一 hide+prevent_close（ticket 04 既有逻辑，未改）——Alt+F4 与自绘关闭同路径，语义单一。
- 布局（`_main.tsx` + `sidebar.tsx`）：根层 `bg-surface-base`，标题栏（h-10）与侧边导航直接置于背景层，内容区 `bg-card` 圆角卡片 `px-2 pb-2` 悬于其上（顶边与标题栏相接）。侧边栏去 `bg-card`/`border-r`，原品牌块（Youmi + Wrench）上移进标题栏左侧——标题栏因此承载品牌位，属 ticket 未明说的结构决策，code-review (b) 记录在案。关闭按钮 hover 用 `bg-destructive`（Windows 标题栏惯例，消费状态色 token）。
- 权限（`capabilities/`）：窗口控制四项权限拆到独立 capability `main-window.json`（windows: 仅 main），default capability 维持原样——浮层不调用窗口控制命令，code-review 后按最小粒度收权。
- 验证基线：tsc、oxlint、vitest 49、cargo check 全绿。实机验证（tauri dev + Win32 程序化测量 + PrintWindow 截图，gui-test-screenshots/t17_*）：无边框渲染；标题栏拖拽位移精确（+100,+80）；右缘拖拽缩放精确（+120）；双击最大化到工作区 -8,-8 → 1936×1048（任务栏未被遮挡），再双击精确还原；最大化按钮点击同理且图标随 isMaximized 切换（截图含还原图标态）；最小化按钮 IsIconic=true；关闭按钮点击后 IsWindowVisible=false 且进程存活；单实例重启唤回隐藏的主窗口；托盘恢复与浮层呼出/失焦隐藏由用户实测通过。浅色主题双卡片层次截图确认。
- 已知边界：深色主题未在真实 WebView2 下验证（本会话无法切 Windows 主题；两层 surface 的深色取值已在 ticket 16 浏览器验证，本 ticket 仅消费语义 token）；原生 Snap Layouts（悬停最大化按钮的分屏面板）在无边框窗口失效，ADR-0004 已接受的 v1 取舍。

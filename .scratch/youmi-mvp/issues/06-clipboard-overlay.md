# 06 — 剪贴板浮层 v1

**What to build:** 第一个全局工具浮层：按 `Win+V` 呼出无边框、置顶的剪贴板管理浮层——列表浏览剪贴板历史、关键词搜索、文本/图片分类查看、删除单条；失焦自动隐藏、再次呼起瞬时；设置页新增历史容量项与"请先关闭系统剪贴板历史"引导。浮层窗口模式（decorations:false / alwaysOnTop / 失焦 hide 而非销毁 / 定位到鼠标所在显示器）在此确立，供 09、11 复用。

**Blocked by:** 04 — 托盘常驻 + 单实例 + 设置页雏形；05 — 剪贴板捕获与存储引擎

**Status:** done（2026-10-09）

- [x] Win+V 呼出/隐藏浮层，浮层出现在鼠标所在显示器
- [x] 列表展示文本与图片条目，可搜索、可按类型过滤、可删除单条
- [x] 失焦即隐，再次呼起瞬时（hide 而非销毁窗口）
- [x] 设置页可修改历史容量并生效
- [x] 设置页含"在 Windows 设置中关闭系统剪贴板历史"的引导提示

## Comments

- Rust 侧：`src-tauri/src/overlay.rs` 承载浮层窗口模式（`create`/`toggle`/`move_to_cursor_monitor`/`contains`），decorations:false + alwaysOnTop + skipTaskbar，失焦（`WindowEvent::Focused(false)`）即 `hide()` 不销毁；`toggle` 以"可见且聚焦"为隐藏条件，可见但未聚焦（blur-hide 与再呼起的竞态）按呼起处理（code-review 修复）。定位：`AppHandle::cursor_position`（物理坐标）+ `available_monitors` 半开区间判定所在屏，居中用物理尺寸计算；几何判定为纯函数，cargo test 覆盖（含副屏负坐标、屏幕边界归属）。
- 快捷键：`tauri-plugin-global-shortcut` 注册默认 `super+v`（spec 默认，改键属 ticket 12）；注册失败（系统剪贴板历史占用）仅 eprintln 降级，不阻塞启动——实测注册失败路径（本机 Win+V 被系统占用时）。handler 以 `Shortcut` 相等匹配 + `ShortcutState::Pressed` 触发 `overlay::toggle`。
- 前端：路由重构——`__root.tsx` 只留 Outlet，sidebar 布局移入 pathless layout `routes/_main.tsx`（URL 不变），浮层路由 `routes/overlay/clipboard.tsx` 挂在根上不带 Sidebar。浮层页：搜索框（`useDeferredValue` 防抖）+ SegmentedTabs 全部/文本/图片 + 列表（文本 line-clamp-3、图片缩略图经 `clipboard_get_image` base64 data URL、相对时间 `formatEntryTime`、行悬停删除）。查询 limit 显式传 1000（command 层默认 100，会限制"浏览历史"——code-review 修复）。`src/lib/clipboard.ts` 为 tauri command 桥接层；`src/tools/clipboard/core.ts` 纯函数（vitest 3 例）。
- 设置页：新增"剪贴板"区——历史容量输入（失焦/回车提交，非法输入回退，引擎 clamp 后值回显；`clipboard_set_capacity` 立即生效 + settings.json 持久化）+ 系统剪贴板历史引导行。`_main` 布局挂载时 `applyClipboardCapacity()` 把持久化容量应用到引擎（容量生效不依赖访问设置页）。
- 实机验证（Windows，WebView2 CDP 驱动真实 UI + 截图）：呼起后 `visibilityState=visible`、搜索框自动聚焦、列表 refetch 到最新（隐藏期捕获的条目出现）；搜索精确过滤、图片 tab 只剩缩略图条目、删除单条 UI 与 DB 同步消失；toggle 双显示器定位精确命中副屏居中公式；hide→show 后 DOM 标记与列表状态保持（hide 不销毁）；主窗口 bringToFront 失焦即隐；设置页容量改 50/60/90 均即时落盘 settings.json。
- 实测发现并修复的问题：① WebView2 要求同一 user data folder 的所有 webview browser args 完全一致——只给主窗口加 additionalBrowserArgs（如 CDP 调试参数）会让隐藏浮层创建报 0x8007139F INVALID_STATE；09/11 新增浮层保持默认参数即可（overlay.rs 注释）。② 隐藏窗口不派发 `visibilitychange`（状态变事件不发），tanstack-query 的 refetchOnWindowFocus 在浮层永不触发——浮层组件在 window focus 时手动 refetch。③ plugin-store 的 set 依赖 auto-save 在多 webview reload/资源关闭时序下不保证落盘——`settings.ts` 写入后显式 `save()`。
- 验证基线：cargo test 34 通过（4 个真实剪贴板 smoke 仍 ignore 手动）、vitest 49 通过、tsc/vite build、oxlint、cargo fmt/clippy 全干净；生产配置（无调试参数）冒烟：浮层隐藏创建成功、注册失败正确降级。
- 遗留与已知边界：① 物理 Win+V 键击到浮层的端到端路径留手动验收——本机会话内系统剪贴板历史即使注册表关闭也不释放 Win+V（需注销），已验证到注册失败降级为止；关闭系统剪贴板历史后注销重新登录即可用真键验证。② 若 Windows 前台锁拦截 `set_focus`（非热键触发的极端时序），呼起刷新依赖的 DOM focus 事件可能不到——热键触发场景 RegisterHotKey 授予前台激活权，实测正常；09/11 若遇到再上 Rust 侧事件通知。③ 浮层列表 limit 1000 与容量上限（最大 1_000_000）之间无分页，v1 够用。④ 验证期间临时改动（CDP 参数、dev toggle command）已全部还原，不随本 ticket 提交；机器状态（系统剪贴板历史开启）已还原。

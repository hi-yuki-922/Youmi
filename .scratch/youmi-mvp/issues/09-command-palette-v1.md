# 09 — 命令面板 v1

**What to build:** 第二个全局工具：`Win+Alt+Space` 呼出命令面板浮层（复用 06 确立的浮层窗口模式）：模糊搜索切换 Youmi 任意工具页；输入算式即时显示四则运算结果；失焦即隐、唤起瞬时。

**Blocked by:** 06 — 剪贴板浮层 v1

**Status:** done（2026-10-12）

- [x] Win+Alt+Space 呼出，Esc/失焦隐藏，再次呼起瞬时
- [x] 模糊搜索工具名，回车直达主窗口对应工具页
- [x] 输入四则运算式即时出结果，回车复制结果
- [x] 计算器核心纯函数化，vitest 覆盖（含非法输入）

## Comments

- Rust：`overlay.rs` 浮层尺寸参数化为 `OverlaySize { width, height }`（剪贴板浮层保持 720×560，命令面板 640×420，模式供 11 继续复用）；新增 `palette.rs` 承载 `palette_open_tool` command——主窗口 show+set_focus → `emit_to("main", "open-tool", tool_id)` → 收起面板，三步均尽力而为、失败不打断。lib.rs 注册默认 `super+alt+space`（spec 默认，改键属 ticket 12），注册失败仅 eprintln 降级、不阻塞启动。capabilities：default.json 增加 palette-overlay 窗口与 `core:window:allow-hide`（Esc/复制成功后 JS 侧自隐窗口；失焦即隐仍由 Rust 共享 handler 承担）。
- 前端：`src/lib/tools.ts` 工具注册表成为单一来源（id/path/label/icon/keywords，Sidebar 导航、面板搜索、跨窗口跳转共用，原 Sidebar NAV_ITEMS 消除）；`src/lib/fuzzy.ts` 子序列模糊匹配（连续/词首/前缀/位置四项计分，空查询得 0 分返回全部）；`src/tools/calculator/core.ts` 递归下降四则计算器（+ - * /、括号、一元正负号、小数，× ÷ 与全角括号归一化；不完整/不配对/非法字符/除零/溢出返回失败原因；一元符号至多连续一个，"1++2" 与 "1+-2" 同构地接受；formatResult 取 12 位有效数字消浮点噪声），vitest 覆盖含非法输入共 21 例；浮层路由 `routes/overlay/palette.tsx`：算式结果行优先于工具命中，输入框持有焦点、↑↓ 选择、回车激活、Esc 自隐、重新呼起清空查询（WebView2 隐藏窗口不发 visibilitychange，window focus 是唯一可靠唤起信号，同 06 结论）；算式回车经 `clipboard_write_own_text`（自家写入通道、不入历史）复制成功后收起面板，失败保留浮层供重试；工具回车经 `palette_open_tool` 直达；`_main.tsx` 挂 open-tool 事件监听切路由（主窗口隐藏时 webview 存活、监听有效）。
- 实机验证（Windows，WebView2 CDP 驱动真实 webview + Win32 枚举窗口探针 + 临时 dev toggle command，临时改动已全部还原）：浮层隐藏创建成功（656×429 物理 = 640×420 逻辑 @125% 缩放）；呼起后全流程通过——模糊搜索 "js" 唯一命中 JSON 工具并高亮；"1+2*3" 即时 "= 7"，回车后系统剪贴板实读为 "7" 且面板收起；"diff" 回车主窗口切到 /diff（文本 diff）；再次呼起输入清空、列表复位；"1/0" 提示"除数不能为零"，未知字符提示带字符引号；Esc 隐藏；"0.1+0.2" 显示 "= 0.3"（浮点噪声已消）。截图存档 gui-test-screenshots/palette-09-*.png。
- 快捷键实测：本机 Microsoft Command Palette（Microsoft.CmdPal.UI）默认占用 Win+Alt+Space，注册走降级路径（仅日志、浮层照常创建）——与 06 的 Win+V 同类冲突；结束其进程后注册成功（外部 RegisterHotKey 探针证实组合键被 Youmi 持有），验证完毕后该应用已恢复运行。是否仿照 15 把默认键改掉属决策票，本票保持 spec 默认。物理键击端到端留手动验收：本验证会话为提权远程受控环境，SendInput/keybd_event 注入对任何全局热键（含 ticket 15 已验证过的 Alt+V）均不触发，无法在本会话完成真键验证；注册→WM_HOTKEY→handler→toggle 链路与剪贴板浮层完全同构。
- 遗留与已知边界：① open-tool 事件在主窗口 webview 尚未完成加载的极端时序下会丢（v1 接受，出现时再上 Rust 侧 pending 补偿）；② Esc 挂在输入框 keydown——输入框每次呼起强制聚焦且列表行不可聚焦，焦点实际不会离开输入框；③ 在装有 Microsoft Command Palette 的机器上 Win+Alt+Space 大概率被占用，冲突引导文案与改键留 ticket 12；④ 验证期间临时改动（CDP browser args、dev toggle command）已全部还原，不随本 ticket 提交。
- 验证基线：cargo test 49 通过（5 个真实剪贴板 smoke 仍 ignore 手动）、vitest 70 通过（新增 21）、tsc + vite build、oxlint、oxfmt、cargo fmt/clippy 全干净。

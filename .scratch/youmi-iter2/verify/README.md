# youmi-iter2 verify

迭代二实机验证脚本与记录。涉键击注入 / UI 交互的修复按 MVP 惯例不写自动化
UI 测试，以「脚本 + 实机验收清单」收口（各 ticket 文件内有对应清单与已执行
记录）；正式验收票为 issues/14。

**收口（2026-10-17）**：tickets 01~13 实机验收全部通过，tauri build 安装包
冒烟通过，全部 ticket 状态收口 done。

## 脚本

| 脚本 | 覆盖 | 前置 |
| --- | --- | --- |
| `verify-pasteback-main-window.ps1` | ticket 01：回帖进主窗口 JSON 输入框 + 剪贴板恢复 | dev 应用运行且带 CDP 插桩 |
| `verify-overlay-modifier-cleanup.ps1` | ticket 03：呼出后残留 Alt/Win 键态清理 + ↓ 正常移动选择 | 同上；需交互式桌面会话（可建立前台窗口） |
| （无脚本） | ticket 02：diff 滚动——已按 verify 记录在案执行（截图 `gui-test-screenshots/iter2-diff-*.png`） | 复跑用 CDP 或人工粘贴长文本 |
| （无脚本） | tickets 04~06：Kbd 呈现（设置页/面板/剪贴板浮层）+ 图标按钮进入录制 + mousedown 取消——CDP 断言记录在各 ticket Comments（截图 `gui-test-screenshots/iter2-*-kbd.png`） | 复跑用 CDP |
| （无脚本） | tickets 07~09：工具页状态跨路由/重启保留与空态不复活、清空按钮联动、SVG 预览与模板复制——2026-10-16 实机验收通过（清单与记录见各 ticket） | 复跑用 CDP 或人工 |
| （无脚本） | ticket 10：ScrollArea 滚动样式统一、无双重滚动、滚轮/键盘滚动——2026-10-16 实机验收通过 | 人工目检 |
| （无脚本） | ticket 11：锁定超时 0=永不（长放置不锁定、手动锁定正常、1..1440 不回归）——2026-10-16 实机验收通过，cargo test 覆盖语义 | 人工（长等待场景） |
| （无脚本） | ticket 12：浮层内 Alt+1~5 快速回帖（含过滤后）、数字角标、搜索框焦点生效、全局无注册——2026-10-16 实机验收通过 | 复跑用 CDP `Input.dispatchKeyEvent` 或人工 |
| （无脚本） | ticket 13：主窗口翻译工作台（语言对 prompt 泛化、流式、共享设置、划词翻译不回归）——2026-10-16 实机验收通过，vitest 覆盖 prompt 泛化 | 复跑用 dev 实跑 |

脚本保持 **纯 ASCII**：PowerShell 5.1 按 ANSI 解码无 BOM 脚本，中文注释会破坏解析。

## CDP 插桩（临时，验收后还原）

WebView2 的 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` 被 wry 显式传入的 browser
args 覆盖、不生效（MVP ticket 13 结论）。插桩做法是给主窗口（`tauri.conf.json`
的 `windows[0].additionalBrowserArgs`）与全部浮层（`overlay.rs::create` 的
builder）以**完全一致**的 `--remote-debugging-port=9223`，`pnpm tauri:dev` 后用
`gui-test-screenshots/cdp.mjs` 驱动/断言。注意：

- 同一 user data folder 的所有 webview 必须用完全一致的 browser args（否则
  隐藏浮层创建报 0x8007139F），两处必须同时加、同时还原；
- 本机若已装有 Youmi 且在托盘运行，single-instance 会让 dev 实例立即退出，
  需先退出已装实例，测毕再启动回来。

## 已知环境注意

- 验证驱动所在会话若无交互式桌面（`GetForegroundWindow()` 恒为 0），呼出后的
  浮层拿不到 OS 前台、OS 层键击注入无法送达；ticket 03 的面板 ↓ 断言此时可退
  用 CDP `Input.dispatchKeyEvent` 注入（仅验 webview 层），OS 层行为留人工清单。

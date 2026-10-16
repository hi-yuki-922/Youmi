# youmi-iter2 verify

迭代二实机验证脚本与记录。涉键击注入 / UI 交互的修复按 MVP 惯例不写自动化
UI 测试，以「脚本 + 实机验收清单」收口（各 ticket 文件内有对应清单与已执行
记录）；正式验收票为 issues/14。

## 脚本

| 脚本 | 覆盖 | 前置 |
| --- | --- | --- |
| `verify-pasteback-main-window.ps1` | ticket 01：回帖进主窗口 JSON 输入框 + 剪贴板恢复 | dev 应用运行且带 CDP 插桩 |
| `verify-overlay-modifier-cleanup.ps1` | ticket 03：呼出后残留 Alt/Win 键态清理 + ↓ 正常移动选择 | 同上；需交互式桌面会话（可建立前台窗口） |
| （无脚本） | ticket 02：diff 滚动——已按 verify 记录在案执行（截图 `gui-test-screenshots/iter2-diff-*.png`） | 复跑用 CDP 或人工粘贴长文本 |

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

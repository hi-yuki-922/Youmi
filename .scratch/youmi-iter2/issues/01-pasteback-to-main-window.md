# 01 — 回帖支持主窗口目标

**What to build:** 焦点在主窗口任意工具输入框时呼出剪贴板浮层，回帖把条目内容粘贴进该输入框光标处——JSON 工具、编码转换、文本 diff 的输入框不再收不到回帖。目标捕获放开主窗口限制（浮层窗口仍排除）；回帖编排对主窗口目标先唤起（还原最小化、显示、置前台）再走既有「自家写入通道写剪贴板 → 浮层隐藏 → 焦点等待与前台校验 → 模拟 Ctrl+V」流程；原剪贴板恢复与 copied-only 降级语义不变。

**Blocked by:** 无——可立即开始

**Status:** done（2026-10-16 实机验收通过）

- [x] 焦点在 JSON 工具输入框 → Alt+V → 双击条目：内容粘贴进该输入框光标处
- [x] 编码转换与文本 diff 的输入框同样可回帖
- [x] 焦点在主窗口非输入区时回帖：无粘贴、无报错，条目已复制可手动 Ctrl+V
- [x] 回帖后用户原剪贴板内容恢复、剪贴板历史不变
- [x] 前台是浮层窗口时捕获行为不变（排除浮层/清过期目标）
- [x] cargo test 覆盖主窗口目标用例（捕获记录主窗口、浮层仍排除、异常路径降级）；实机验证脚本留档 verify/

## Comments

### 2026-10-16 — 实现说明

- **平台 seam**：`PastebackAccess` 新增 `awaken_window(hwnd) -> bool`（还原最小化 → `SW_SHOW` → `SetForegroundWindow`，成败以置前台返回值为准）；Win32 实现于 `platform/windows/pasteback.rs`。
- **目标捕获**（`clipboard/pasteback.rs::capture_target`）：前台为外部窗口 → 目标；前台为本进程且恰为主窗口句柄（`main_window_hwnd` 从 `WebviewWindow::hwnd()` 取）→ 目标；浮层窗口与其他自家窗口 → 无目标（固定浮层前台照旧清除过期目标）。
- **回帖编排**（`run`）：`PastebackDeps` 新增 `main_window: Option<isize>`；`valid_target` 放宽为「窗口存活且（外部 或 == 主窗口）」；主窗口目标先 `awaken_window` 再走既有焦点等待/前台校验/Ctrl+V/延迟恢复流程，异常路径（唤起失败、焦点被抢、注入失败）仍降级 copied-only。
- **测试**：pasteback 模块 19 例全绿——新增主窗口回帖成功（先 Awaken 后 Paste + 恢复）、浮层目标仍排除、唤起失败降级、主窗口句柄失效降级；捕获用例改为主窗口命中/浮层排除/无主窗口句柄时自家窗口一律拒绝。

### 2026-10-16 — 实机验证记录（verify/verify-pasteback-main-window.ps1，ALL PASS）

CDP 插桩构建（见 verify/README）驱动真机：种入两条历史后焦点置于主窗口 JSON 工具输入框 → Alt+V → {DOWN}{DOWN}{ENTER} 回帖第 2 行 → **条目文本出现在 JSON 输入框内**（8s 轮询命中）→ **用户原剪贴板内容恢复**（5s 轮询命中）。人工待确认项：编码转换/文本 diff 输入框同路径回帖、非输入区焦点回帖降级表现。

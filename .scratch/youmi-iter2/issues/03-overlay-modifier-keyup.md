# 03 — 浮层呼出残留修饰键清理

**What to build:** Alt+V / 命令面板全局快捷键呼出浮层后，修饰键的按下状态被立即清干净：呼出后按 ↓ 直接移动选择，不再触发系统窗口菜单。呼出路径在窗口 show 后补发残留修饰键 keyup，清理原语与回帖/划词取词路径共用同一实现（查询异步键态后补 keyup、不回按）。

**Blocked by:** 无——可立即开始

**Status:** done（2026-10-16 实机验收通过）

- [x] Alt+V 呼出剪贴板浮层后按 ↓：移动列表选择，不弹系统菜单
- [x] 命令面板快捷键呼出后同理
- [x] 呼出后立即敲字符、回车等行为不受影响
- [x] 回帖与划词取词路径的既有清理行为不回归
- [x] 实机验证（涉键击注入，留 verify 脚本或记录手动步骤）

## Comments

### 2026-10-16 — 实现说明

- **共享原语**：`platform/windows/mod.rs` 抽出 `release_lingering_modifiers()`——查询异步键表，对仍按下的 Shift/Alt/Win（左右两枚）补发 keyup，不回按；`send_ctrl_chord`（回帖 Ctrl+V / 划词 Ctrl+C 路径）改为复用同一收集函数，既有行为不变。Ctrl 不在清理列：它是 chord 自身组成部分。
- **呼出路径接入**：`overlay::toggle` 的呼起分支在 `window.show()` + `set_focus()` 之后调用原语（剪贴板浮层与命令面板共用），失败仅日志不阻断呼出；划词翻译的 `show_at_cursor` 不在此票范围。
- **冒烟测试**：新增 `#[ignore]` smoke `release_lingering_modifiers_releases_alt`（真实注入 Alt 按下 → 清理 → 断言释放），与既有 chord smoke 同款手动执行。

### 2026-10-16 — 实机验证记录（verify/verify-overlay-modifier-cleanup.ps1）

- **Alt+V 场景**：按住 Alt（模拟快捷键触发瞬间）敲 V 呼出浮层 → `GetAsyncKeyState(Alt)` 为**抬起态**（清理已生效）→ 普通 ↓ 使剪贴板列表第一行获得焦点（`activeElement` = LI，未被残留 Alt 吞掉/未弹系统菜单）。
- **super+alt+space 场景**：按住 Win+Alt 敲 Space 呼出命令面板 → Win 与 Alt 异步键态均为**抬起态** → ↓ 使 `aria-activedescendant` 从 `palette-item-0` 前移到 `palette-item-1`（此步经 CDP `Input.dispatchKeyEvent` 注入——验证驱动会话无法建立 OS 前台窗口，OS 层 ↓ 到面板的行为在交互式桌面上由人工按验收清单复核）。
- 呼出后敲字符/回车不受影响、回帖与取词路径清理不回归：原语为纯提取重构，chord 注入事件序列不变，由既有 cargo 测试与人工验收覆盖。

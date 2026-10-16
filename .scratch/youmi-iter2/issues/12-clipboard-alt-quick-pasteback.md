# 12 — 剪贴板 Alt+1~5 快速回帖

**What to build:** 剪贴板浮层内 Alt+1~5 回帖当前可见（含搜索过滤后）列表第 N 条，行为等同双击回帖；列表前五行显示数字角标；焦点在搜索框时同样生效；仅浮层窗口内生效（前端键盘监听，不占全局改键表、全局无注册）。设置页快捷键区"剪贴板"组补入 Alt+1~5 只读条目（Kbd 呈现）。

**Blocked by:** 05 — 设置页快捷键区：作用域分组 + 录制交互改造

**Status:** ready-for-human（2026-10-16 实现完成；Alt+1~5 键击行为需实机/CDP 复验）

- [ ] Alt+1~5 回帖可见列表第 N 条（无过滤与过滤后都成立）
- [ ] 前五行内数字角标
- [ ] 焦点在搜索框时同样生效
- [ ] 全局无注册（其他应用按 Alt+数字不受影响）
- [ ] 设置页剪贴板组只读展示 Alt+1~5

## Comments

### 2026-10-16 — 实现

- **监听**（`src/routes/overlay/clipboard.tsx`）：浮层窗口内 `window` keydown 前端监听（非全局注册、不占全局改键表）；`altKey` 且无其他修饰、`e.code` 匹配 `Digit1~5` 时回帖当前可见（含搜索过滤后）列表第 N 条，行为等同双击回帖（复用同一 `pasteback`，含 isPending 防抖）；Alt 组合不产生字符输入，焦点在搜索框时同样生效；无对应条目时按键放行。
- **角标**：前五行行首渲染 `<Kbd>` 数字角标（带 aria-label "Alt+N"）。
- **底部提示**追加 `⌘ 1~5 回帖前五条`（Kbd 呈现）。
- **设置页**（`src/routes/_main/settings.tsx`）：快捷键区新增"剪贴板"作用域组（ScopeGroupCard scope="clipboard"），5 条只读条目 `alt+Digit1~5`（ComboKbd 呈现 ⌘ 1~⌘ 5，formatCombo 为 Alt+1~Alt+5），无录制入口。

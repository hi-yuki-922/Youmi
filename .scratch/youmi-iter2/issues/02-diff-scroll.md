# 02 — diff 长内容滚动修复

**What to build:** 文本 diff 输入数百行长文本后，diff 视图可垂直滚动到末尾。滚动仍由 diff 库内部 scroller 承担：应用侧只保留高度约束、不接管滚动；若需覆写优先用库提供的 overflow 覆写变量，不 fork。

**Blocked by:** 无——可立即开始

**Status:** done（2026-10-16 实机验收通过）

- [x] 数百行长文本两侧对比：diff 视图可垂直滚动到底
- [x] 并排与统一两种视图均可滚动
- [x] wrap 模式行内换行正常，页面无双重滚动条
- [x] 短内容时布局无回退（无多余空白/滚动区）

## Comments

### 2026-10-16 — 实现说明

- **根因**：`@pierre/diffs` 的垂直滚动根就是 `diffs-container` 元素本身——库在其上监听 scroll、读 `root.scrollTop` 驱动虚拟化，并为内部容器写入总高度。应用侧此前给它 `overflow-hidden`，把库自己的滚动裁掉：外层页面高度恰好占满卡片不产生滚动，内层被 clip，长内容不可滚。
- **修复**（`src/routes/_main/diff.tsx`）：`overflow-hidden` → `overflow-x-hidden overflow-y-auto`，`min-h-0 flex-1` 高度约束保留。未 fork、未接管滚动——开启的正是库内部 scroller 所在元素；横向裁剪防御 wrap 模式下无意义的横向滚动条（内部 `[data-code]` 自带横向滚动职责）。

### 2026-10-16 — 实机验证记录（CDP 驱动真机 webview）

- 两侧各 400 行长文本（wrap 模式）：`scrollHeight 16036 / clientHeight 388`，`scrollTop` 置底后 `scrollTop == scrollHeight - clientHeight`（atBottom=true），截图 `gui-test-screenshots/iter2-diff-top.png`（第 1~10 行）与 `iter2-diff-bottom.png`（第 392~400 行 + "No newline at end of file"）。
- 统一视图同样滚到底（`iter2-diff-unified-bottom.png`）。
- 行内换行正常（长行折行显示）、diff 容器只有一条滚动条、页面无双重滚动；两输入框清空时仍走既有的空态占位分支，无多余滚动区。

# 02 — diff 长内容滚动修复

**What to build:** 文本 diff 输入数百行长文本后，diff 视图可垂直滚动到末尾。滚动仍由 diff 库内部 scroller 承担：应用侧只保留高度约束、不接管滚动；若需覆写优先用库提供的 overflow 覆写变量，不 fork。

**Blocked by:** 无——可立即开始

**Status:** ready-for-agent

- [ ] 数百行长文本两侧对比：diff 视图可垂直滚动到底
- [ ] 并排与统一两种视图均可滚动
- [ ] wrap 模式行内换行正常，页面无双重滚动条
- [ ] 短内容时布局无回退（无多余空白/滚动区）

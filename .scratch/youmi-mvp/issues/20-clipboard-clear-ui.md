# 20 — 剪贴板历史"清空"入口

**What to build:** spec story 25 要求"删除单条或清空历史"；单条删除已由浮层承担（ticket 06），清空的后端命令 `clipboard_clear` 已存在（commands.rs）但没有任何 UI 入口，属死代码。在剪贴板浮层加清空入口（含误触防护），接通 `clipboard_clear` 并刷新列表。

**Blocked by:** 06 — 剪贴板浮层

**Status:** needs-triage

**来源:** ticket 19 全量 review，Spec 轴发现（story 25 部分实现）。

- [ ] 浮层内可清空全部历史，有误触防护
- [ ] 清空后列表即时刷新，容量/recency 状态一致

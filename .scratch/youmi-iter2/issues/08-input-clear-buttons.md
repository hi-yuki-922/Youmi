# 08 — 工具输入清空按钮

**What to build:** JSON 工具输入框、编码转换三个子工具的输入框、文本 diff 两个输入框各带一键清空，放置于输入框 label 行动作位；转换类清空输入时联动清空输出与错误态。

**Blocked by:** 无——可立即开始

**Status:** ready-for-human（2026-10-16 实现完成；清空交互待实机复验）

- [ ] 上述每个输入框都有清空按钮且风格一致
- [ ] 清空即清输入，转换类同时清输出/错误态
- [ ] 清空后可继续输入，无残留状态

## Comments

### 2026-10-16 — 实现

- **ClearButton**（`src/components/ClearButton.tsx`）：X 图标 ghost icon 按钮，置于 LabeledTextarea 的 label 行动作位，直接清空无确认；内容为空时禁用（与 CopyButton 的空禁用一致）。
- **JSON 工具**：输入/输出两侧由裸 label+Textarea 迁到 LabeledTextarea（复制按钮保持在页头）；清空输入联动清空输出与状态条（旧输入的结果不再有意义）。
- **编码转换**：双向转换器清空任一侧即整体归零（两侧文本 + 错误态）；query 工具清空走既有 `handleQueryChange("")`（行表与错误态联动）。
- **文本 diff**：原/新文本各自独立清空（diff 即时重算，无输出态需联动）。

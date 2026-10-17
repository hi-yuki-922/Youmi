# 08 — 工具输入清空按钮

**What to build:** JSON 工具输入框、编码转换三个子工具的输入框、文本 diff 两个输入框各带一键清空，放置于输入框 label 行动作位；转换类清空输入时联动清空输出与错误态。

**Blocked by:** 无——可立即开始

**Status:** done（2026-10-17 实机验收通过，收口验收票 14）

- [x] 上述每个输入框都有清空按钮且风格一致
- [x] 清空即清输入，转换类同时清输出/错误态
- [x] 清空后可继续输入，无残留状态

## Comments

### 2026-10-16 — 实现

- **ClearButton**（`src/components/ClearButton.tsx`）：X 图标 ghost icon 按钮，置于 LabeledTextarea 的 label 行动作位，直接清空无确认；内容为空时禁用（与 CopyButton 的空禁用一致）。
- **JSON 工具**：输入/输出两侧由裸 label+Textarea 迁到 LabeledTextarea（复制按钮保持在页头）；清空输入联动清空输出与状态条（旧输入的结果不再有意义）。
- **编码转换**：双向转换器清空任一侧即整体归零（两侧文本 + 错误态）；query 工具清空走既有 `handleQueryChange("")`（行表与错误态联动）。
- **文本 diff**：原/新文本各自独立清空（diff 即时重算，无输出态需联动）。

### 2026-10-16 — 验收反馈与修复

行为验收通过，3 处呈现问题反馈如下，均已修复：

1. **编码转换：复制结果与清空按钮未靠右成组**（验收图 1）——`LabeledTextarea` 的 label 行 `justify-between` 直接渲染 action fragment，fragment 不产生 DOM 节点，label/复制/清除三个直接子元素被撑开成左中右分布，复制按钮被挤到中部。修复：`LabeledTextarea` 内部把 action 包进右侧 flex 组（`flex items-center gap-1`），label 行恒为两子元素；translate 页调用方原来的同款包裹 div 随之删除。
2. **JSON 工具：左右两栏高度不一致**（验收图 2）——输入侧 label 行被 32px 清除按钮撑高、输出侧仅文字行高（20px），两栏 textarea 顶边错位。修复：label 行加 `min-h-8`，无 action 的一侧与有按钮一侧同高。
3. **清除按钮图标改 BrushCleaning**——`ClearButton` 的 X 换为 lucide `BrushCleaning`（扫帚语义更贴合"清扫输入"）。

修复经 vite dev 实渲染截图复核（编码转换两栏按钮成组靠右、JSON 两栏顶边对齐、图标渲染与联动清空正常），tsc/oxlint/vitest 全绿；收口待人工复验。

### 2026-10-17 — 实机验收通过

三处修复实机复核无回归：清空按钮成组靠右、双栏同高、BrushCleaning 图标正常；联动清空行为符合本票三条验收项。随迭代验收票 14 收口。

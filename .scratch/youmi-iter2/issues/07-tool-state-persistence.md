# 07 — 工具页状态持久化

**What to build:** JSON 工具、编码转换（含子 tab 与三个子工具）、文本 diff 的全部状态跨路由切换与应用重启保留：输入、输出、错误态、视图/缩进选择经 localStorage 持久化，按工具划分存储键，挂载时恢复。

**Blocked by:** 无——可立即开始

**Status:** done（2026-10-16 实机验收通过）

- [x] JSON 工具：输入/输出/缩进选择跨路由与重启保留
- [x] 编码转换：子 tab 与三个子工具的输入输出各自保留
- [x] 文本 diff：两侧文本与视图模式保留
- [x] 清空后的空状态重启后仍是空（空值不复活旧内容）
- [x] vitest 覆盖持久化序列化/恢复 roundtrip

## Comments

### 2026-10-16 — 实现

- **持久化原语**（`src/lib/tool-state.ts`）：`saveToolState` / `loadToolState`（注入式 storage，键前缀 `tools.`）+ `usePersistedToolState`（挂载恢复、变更即写回）；restore 形状校验，损坏/形状不符回退默认值。**空状态同样落盘**——清空后重启不复活旧内容由"每次变更全量写回"保证，roundtrip 测试有专门用例。
- **接入范围**：json（输入/输出/缩进/状态条）、编码转换（子 tab、svg-url 与 base64 双侧文本与错误态、query 字符串与错误态）、文本 diff（视图/语言/两侧文本）。键按工具划分：`tools.json.input`、`tools.encoding.svg-url.left` 等。
- **query 键值表是派生视图**：只持久化 query 字符串与错误态，恢复时重新 `parseQuery` 生成行（行 id 每会话重新分配）。
- **错误态可空**：`restoreNullableString` 区分 undefined（形状不符→回退）与 null（合法"无错误"）——roundtrip 测试抓到并修掉了 `??` 合并会吞掉合法 null 的 bug。
- vitest：`tool-state.test.ts` 9 例（roundtrip、键前缀、空值不复活、null 恢复、JSON 损坏、形状不符、存储抛错不外溢）。

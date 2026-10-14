# 20 — 剪贴板历史"清空"入口

**What to build:** spec story 25 要求"删除单条或清空历史"；单条删除已由浮层承担（ticket 06），清空的后端命令 `clipboard_clear` 已存在（commands.rs）但没有任何 UI 入口，属死代码。在剪贴板浮层加清空入口（含误触防护），接通 `clipboard_clear` 并刷新列表。

**Blocked by:** 06 — 剪贴板浮层

**Status:** ready-for-human

**来源:** ticket 19 全量 review，Spec 轴发现（story 25 部分实现）。

- [ ] 浮层内可清空全部历史，有误触防护
- [ ] 清空后列表即时刷新，容量/recency 状态一致

## Comments

- 实现（2026-10-14，前端单点改动，Rust 零改动）：`src/lib/clipboard.ts` 新增 `clearClipboardHistory()` 桥接已注册的 `clipboard_clear`（返回删除条数）。浮层头部（类型过滤右侧）新增 `ClearHistoryButton`，仅当未过滤历史非空时渲染——用独立的 `limit:1` 查询（`["clipboard-entries","any"]`）而非当前过滤结果判定，避免"过滤无结果时入口消失"与"过滤态下清空范围误判"；该查询与列表共享 invalidate 前缀，清空后一并刷新。
- 误触防护为两段式：首击图标进入确认态（destructive"确认清空"），限时 3s 内再击才执行；Esc、窗口失焦、超时均退回。进入确认态后 400ms 内忽略点击（原地快速双击不等于确认）；窗口 blur 也退回——浮层 hide 而非销毁，确认态会跨隐藏存活，重新呼起不能停留在"一击即清"。清空成功后 invalidate `["clipboard-entries"]`（含过滤视图与 any 查询）并把焦点还给搜索框；容量设置不受影响（repo `clear` 只 `DELETE FROM clipboard_history`）。
- 验证：tsc + vite build、vitest 84 通过、oxlint 干净、pre-commit hook（oxlint --fix + oxfmt）干净；code-review 双轴（Standards/Spec）通过，review 发现的 4 项已当场修复（裸 button 换 ghost Button 补 focus-visible、unmount cleanup 直调 disarm、mutationFn 直引函数、入口显隐改按未过滤历史 + blur 退回）。
- 待人工实机冒烟后置 done：Alt+V 呼出浮层 → 右上清空图标变"确认清空" → 再击清空 → 列表/空态即时刷新、设置页容量不变；顺带过一遍 Esc / 3s 超时 / 点别处超时 / 失焦重呼退回四条退回路径。
- 实机反馈（2026-10-14）：清空主路径与四条退回路径通过。发现缺陷——清空后重新复制再呼出，列表有内容但清空按钮不显示，需重启应用。根因：呼起刷新走的是 ticket 06 确立的手动 refetch（WebView2 隐藏/显示不派发 visibilitychange），但只 refetch 了列表查询；新增的显隐 any 查询被清空后的空结果缓存住，隐藏期新捕获的条目不会让它刷新。修复（7632db5）：呼起刷新改按 `["clipboard-entries"]` 前缀 `refetchQueries`，列表各视图与显隐查询一并刷新。自动化全绿（tsc/build、vitest 84、oxlint）。待复验：清空 → 复制 → 重呼，按钮应随列表一同出现。

# 02 — 斜杠命令注册表与 /t 触发翻译

**What to build:** 命令面板支持 `/命令名 参数` 语法：输入 `/` 列出全部命令（名称+描述+用法提示），继续输入按命令名前缀过滤；`/` 前缀模式下结果列表只含斜杠命令，不混工具/应用/算式搜索。首个命令 `/t <文本>` 回车 → 收起面板、主窗口前置、切翻译页、填入文本并自动触发翻译。命令表为可扩展注册表，新增命令只改注册表。

**Blocked by:** None — can start immediately

**Status:** done（2026-10-20 实机验收通过）

- [x] 输入 `/` 列出全部命令（名称+描述+用法提示），按命令名前缀过滤
- [x] `/` 前缀模式下结果只含斜杠命令，不混入工具/应用/算式
- [x] 未知命令名给提示，不误执行
- [x] `/t 你好` → 面板收起、主窗口前置切翻译页、文本填入并自动翻译出结果（实机验收）
- [x] 裸 `/t` 仅打开翻译页，光标落输入框，不触发翻译（实机验收）
- [x] `/t你好`（无空格分隔）不算命令
- [x] 翻译流式中再触发：停止旧会话、填入新文本（实机验收）
- [x] 缺 API key 走翻译页既有引导态（复用 run 既有 onMissingKey 路径）
- [x] vitest 覆盖斜杠解析（前缀过滤、参数切分、未知命令）

## Comments

### 2026-10-19 — 实现

- **解析与注册表**（`src/lib/slash.ts`）：`SlashCommand { name, description, usage, run }` 注册表 + `parseSlashQuery` 纯函数——无空格即前缀过滤（`/` 列全部），有空格则首个空格前名字精确命中才算执行形态，紧凑输入（`/t你好`）落"未知命令"。同文件承载**文本直达通道**：面板是独立 webview，文本经 Rust 事件到主窗口后，`deliverToolText` 对已挂载页面监听直送、未挂载暂存（`takePendingToolText` 挂载时取走）。vitest 11 例。
- **首个命令 `/t`**：trim 后非空 → `openToolWithText("translate", text)`；空 → 仅打开翻译页。
- **事件载荷扩展**：Rust `palette_open_tool` 增 `text: Option<String>`，`open-tool` payload 变 `{ toolId, text }`（serde_json）；前端 `lib/palette.ts` 同步类型，`openTool` 收敛为 `openToolWithText` 的无文本特例。
- **主窗口分发**（`_main.tsx`）：先 `deliverToolText` 再导航，暂存兜住挂载竞态。
- **翻译页消费**（`_main/translate.tsx`）：挂载取积压 + 订阅直送，`translateDelivered` 与手动触发一致（先 stop 旧会话）；依赖仅语言对，方向恒为当前选择。缺 key 走既有引导态。
- **面板 UI**：斜杠模式独占结果列表（Terminal 图标行：`/名` + 描述；执行行：`/名` + 参数预览；未知命令空态提示）；占位文案补"或 / 命令"。

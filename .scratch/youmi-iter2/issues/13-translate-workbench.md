# 13 — 翻译工作台工具 v1

**What to build:** 主窗口新增"翻译"工作台工具（工具注册表收录，侧边栏与命令面板可达）：中/英双下拉 + 交换按钮，默认英→中；输入区 Ctrl+⏎ 或"翻译"按钮触发，译文流式呈现，翻译中再触发即中止上一次；译文一键复制；API key 未配置时页内引导态。DeepSeek 调用与划词翻译共享模型名与 API key 设置；prompt 层把方向规则泛化为"目标语言"参数，划词翻译既有行为（三方向 + 流式 + 失焦隐藏）不回归。

**Blocked by:** 04 — Kbd 组件与组合键符号格式化

**Status:** done（2026-10-16 实机验收通过）

- [x] 侧边栏与命令面板可进入翻译页
- [x] 中英双下拉 + 交换按钮，默认英→中
- [x] Ctrl+⏎ 与翻译按钮触发，流式呈现，再触发中止上一次
- [x] 译文一键复制
- [x] API key 未配置呈引导态（可达设置对应分区）
- [x] 与划词翻译共享模型/API key 设置（改一处两处生效）
- [x] 划词翻译三方向 + 流式 + 失焦隐藏不回归
- [x] vitest 覆盖 prompt 目标语言泛化

## Comments

### 2026-10-16 — 实现

- **prompt 泛化**（`src/tools/translate/core.ts`）：新增 `TranslateLanguage`（"zh" | "en"）与 `buildTranslatePairPrompt(text, source, target)`——方向规则泛化为语言对参数；源/目标语言名分表（中文作源用泛称 Chinese、作目标为 Simplified Chinese），两个固定划词方向改由同一 `languagePairRule` 生成，提示词字符串逐字节不变（既有用例原样通过）。vitest 新增 4 例：语言对与固定方向同构（双向）、原文透传、只输出译文约束。
- **传输层收敛**（`src/lib/translate.ts`）：`streamTranslate` 不再接收 text/direction，改收构造好的 `TranslatePrompt`——本模块只管传输，提示词知识全部收敛到 core。
- **流式会话共享**（`useTranslateStream`，同在 `src/lib/translate.ts`）：中止控制器生命周期、发起时现取 key/模型、delta 追加、「翻译中再触发即中止上一次（主动中止不算失败）」与卸载自动中止，收拢为浮层与工作台共用的 hook；缺 key 的处置经 `onMissingKey` 交给调用方（浮层报错文案 / 工作台引导态），未提供时回退浮层文案。划词浮层改为消费该 hook，行为逐分支等价。
- **工作台页**（`src/routes/_main/translate.tsx`，路由 `/translate`）：中/英双 Select + 交换按钮（默认英→中）；一侧选成与对侧相同即整体对调，恒保持源 ≠ 目标；输入区 Ctrl+⏎ 与"翻译"按钮触发（Kbd ⌃⏎ 提示）；翻译中再触发经 AbortController 中止上一次；译文区流式呈现（脉冲光标）+ CopyButton；原文/方向经 `usePersistedToolState` 持久化（键 `tools.translate.*`），译文会话态不持久化。
- **共享设置**：每次发起现取 `getTranslateApiKey` / `getTranslateModel`——设置页改一处，下一发即生效；未配置 key 时挂载探测呈页内引导态（Link 跳设置），探测失败不冒充未配置。
- **注册表**：`TOOLS` 收录 translate（Languages 图标，diff 与密码本之间），侧边栏、命令面板、跨窗口跳转自动共用；Rust `palette_open_tool` 仅转发 id，无需改动。

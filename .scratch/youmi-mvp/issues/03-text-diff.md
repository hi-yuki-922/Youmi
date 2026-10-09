# 03 — 文本 diff

**What to build:** 工作台工具：粘贴两段文本即时对比，集成 `@pierre/diffs` 呈现——并排视图为默认、可切统一视图，带行级增删高亮、行内字符级高亮与语法高亮。

**Blocked by:** 01 — App 骨架 + JSON 工具

**Status:** done（2026-10-09）

- [x] 两段文本并排对比：行级增删高亮 + 行内字符级高亮 + 语法高亮
- [x] 并排/统一视图可切换
- [x] 任一侧为空时界面不崩、呈现合理空态

**实现说明（2026-10-09）：**

- 集成 `@pierre/diffs@1.5.2`（Shiki 语法高亮，JS engine 无 WASM，样式自包含于 Shadow DOM，满足现有 CSP `style-src 'unsafe-inline'`）。页面 `src/routes/diff.tsx`：双栏 textarea（原文本/新文本）即时对比，`parseDiffFromFile` 在 useMemo 中随输入重算；`FileDiff` 组件渲染，options 为 `diffStyle: split|unified`（并排默认）、`lineDiffType: 'char'`（行内字符级）、`diffIndicators: 'bars'`、`overflow: 'wrap'`、`disableFileHeader`；`themeType` 不传即用库默认 `"system"`（CSS `light-dark()` 跟随系统，与外壳 `prefers-color-scheme` 同信号，无需自写 hook——review 后修正）。
- 语法高亮：任意粘贴文本无文件名可推断语言，提供「语言」下拉（纯文本默认 + JSON/TS/JS/TSX/HTML/CSS/Python/Rust/Go/Java/C++/C#/SQL/YAML/Markdown/Shell），选中值写入 `FileContents.lang`；纯文本不传 lang，交给库按 `.txt` 文件名推断。
- 空态：双侧为空显示虚线提示、不挂载 FileDiff；单侧为空渲染全增/全删 diff（有意义的结果，不算异常态）。
- review 修正：抽出与 encoding 页重复的两个共享件 `ui/segmented-tabs.tsx`（分段 tablist）与 `ui/labeled-textarea.tsx`（标签行 + 等宽 textarea，`action` 槽放复制按钮），encoding.tsx 同步改用（ConverterPane 内联删除，行为不变）。
- 验证：tsc / oxlint / vitest（全仓 46 全绿）/ vite build 通过；浏览器 GUI 冒烟截图验收（空态、JSON 并排、统一切换、单侧空、深色 themeType、重构后复查共 7 张）——深色为页内 matchMedia 模拟，仅驱动 diff 区配色，应用外壳深色留待 `pnpm run tauri dev` 系统主题验收。统一视图行号为库固有单列设计（库无双行号选项）。

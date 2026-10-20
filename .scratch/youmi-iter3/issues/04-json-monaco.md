# 04 — JSON 工具集成 monaco editor

**What to build:** JSON 工具输入区与输出区替换为 monaco 编辑器（输出只读）：语法高亮、行号、JSON 错误标红；本地打包（workers 一并打包，无任何 CDN 依赖，离线可用）；主题跟随系统明暗（`prefers-color-scheme`）。既有按钮组（格式化/压缩/校验/转义/去转义）、缩进选择、行列错误提示条、输入/输出持久化全部不回归。

**Blocked by:** None — can start immediately

**Status:** done（2026-10-20 实机验收通过）

- [x] 输入区为 monaco 编辑器：语法高亮、行号、JSON 错误标红（json worker 实时校验）
- [x] 输出区为只读 monaco，带语法高亮
- [x] 无 CDN 依赖，workers 正常加载（生产构建产物含独立 worker chunk，断网实机验收待做）
- [x] 主题跟随系统明暗切换（matchMedia 监听，vs-dark / vs）
- [x] 格式化/压缩/校验/转义/去转义按钮与缩进选择不回归
- [x] 行列错误提示条不回归（按钮触发的既有 JsonResult 反馈）
- [x] 输入/输出持久化不回归（usePersistedToolState 原样）

## Comments

### 2026-10-19 — 实现

- **组件**（`src/components/CodeEditor.tsx`）：自研薄封装（不引 @monaco-editor/react，其 loader 默认走 CDN）。编辑器实例挂载建一次；受控模式——外部值与编辑器内容一致时不同步（防光标跳动），内部输入经 ref 转发的 onChange 回流；占位文案空态叠加渲染；主题 `prefers-color-scheme` → vs-dark/vs 全局切换。选项：自动布局、wordWrap、无 minimap、fixedOverflowWidgets。
- **rolldown-vite 8.3.3 的三个坑**（都实测踩到，文档化在 vite.config.ts 注释）：① `?worker` 包子路径查询导入不做转换，原样外置进产物；② `new URL` worker 子构建里 monaco 包子路径解析失败；③ `editor.api` 子路径解析时而失败、exports 通配符 TS 也解不出类型。**解法**：所有 monaco 导入走 vite alias 直链包内真实文件（`monaco-api` / `monaco-json-contribution` / `monaco-json-worker` / `monaco-editor-worker`），worker 用本地 wrapper 文件（`src/monaco/*.worker.ts`）作 `new Worker(new URL(...))` 锚点独立成 chunk。alias 模块类型经 `src/types/monaco-modules.d.ts` 挂到包根。
- **产物**：主包 +2.7MB（monaco 核心 + JSON 语言），worker 独立 chunk；CSP 补 `font-src 'self' data:`（codicon 字体 data URI）与 `worker-src 'self'`。
- **按需引入**：只挂 JSON 语言贡献（不引 editor.main 全量语言），CSS/TS 等其他语言 worker 不打包。

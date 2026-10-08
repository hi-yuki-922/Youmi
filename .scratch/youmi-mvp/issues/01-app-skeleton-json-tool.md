# 01 — App 骨架 + JSON 工具

**What to build:** 第一条 tracer bullet，贯穿整个前端栈：`tauri dev` 启动后出现中文主窗口——左侧 sidebar（lucide 图标）导航各工作台工具与设置占位，tanstack-router 扁平路由；JSON 工具页完整可用；husky + oxlint/oxfmt、vitest 与 cargo test 工具链在此就位。技术栈选型见 spec（`.scratch/youmi-mvp/spec.md`）Implementation Decisions。

**Blocked by:** None — can start immediately

**Status:** done（2026-10-08）

- [x] `tauri dev` 打开主窗口：sidebar 导航、扁平路由、中文 UI、主题跟随系统（shadcn）
- [x] JSON 工具：格式化（2/4/tab 缩进可调）、压缩、校验并定位错误行、转义/去转义，结果一键复制
- [x] JSON 转换核心为独立纯函数模块，vitest 覆盖正常与畸形输入（`src/tools/json/core.ts`，12 测试）
- [x] husky pre-commit（oxlint + oxfmt via lint-staged）生效；vitest 与 cargo test 均可运行且通过
- [x] Vite 8 + React 19 + TypeScript + tanstack-router + tanstack-query + shadcn-ui + lucide 按 spec 选型落地

**实现说明（2026-10-08）：**

- 前端在浏览器（vite dev）逐项验证：sidebar 导航/路由重定向/格式化/压缩/校验行列定位/转义/复制/视觉布局均通过；原生窗口打开留待用户 `npm run tauri dev` 手动验收（Rust 侧已随 cargo test 编译通过）。
- cargo test 当前 0 个测试即通过：spec Testing Decisions 将 Rust 测试对象（剪贴板 Repository、密码本 crypto）划在 ticket 05/08，届时落地，不为此造假测试。
- 审查修正：移除模板残留的 opener 插件（不在 spec 插件清单）；CSP 由 `null` 收紧为白名单（spec 要求不放开外联）；删除未使用的 card 组件；README 重写。

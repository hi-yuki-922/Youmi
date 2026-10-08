# 01 — App 骨架 + JSON 工具

**What to build:** 第一条 tracer bullet，贯穿整个前端栈：`tauri dev` 启动后出现中文主窗口——左侧 sidebar（lucide 图标）导航各工作台工具与设置占位，tanstack-router 扁平路由；JSON 工具页完整可用；husky + oxlint/oxfmt、vitest 与 cargo test 工具链在此就位。技术栈选型见 spec（`.scratch/youmi-mvp/spec.md`）Implementation Decisions。

**Blocked by:** None — can start immediately

**Status:** ready-for-agent

- [ ] `tauri dev` 打开主窗口：sidebar 导航、扁平路由、中文 UI、主题跟随系统（shadcn）
- [ ] JSON 工具：格式化（2/4/tab 缩进可调）、压缩、校验并定位错误行、转义/去转义，结果一键复制
- [ ] JSON 转换核心为独立纯函数模块，vitest 覆盖正常与畸形输入
- [ ] husky pre-commit（oxlint + oxfmt via lint-staged）生效；vitest 与 cargo test 均可运行且通过
- [ ] Vite 8 + React 19 + TypeScript + tanstack-router + tanstack-query + shadcn-ui + lucide 按 spec 选型落地

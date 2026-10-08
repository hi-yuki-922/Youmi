# Youmi

Windows 优先（保留未来迁移 macOS 的可能）的个人桌面工具集合。**主窗口**承载工作台工具（编码转换、JSON 工具、文本 diff、密码本），**全局工具**（剪贴板管理、命令面板、划词翻译）常驻系统托盘、以全局快捷键呼出。

## 技术栈

- Tauri 2 + Rust（平台能力隔离在 seam 模块，见 `docs/adr/`）
- Vite + React 19 + TypeScript
- tanstack-router（扁平路由）+ tanstack-query
- Tailwind CSS v4 + shadcn 风格组件 + lucide icons
- 工具链：husky + lint-staged（oxlint / oxfmt）、vitest、cargo test

## 开发

```bash
npm install        # 安装前端依赖（husky 在 prepare 阶段自动接入）
npm run tauri dev  # 启动开发窗口
npm test           # vitest
npm run lint       # oxlint
npm run format     # oxfmt（写入模式）
cd src-tauri && cargo test
```

## 结构

- `src/routes/` — 扁平路由页（每个工作台工具一页）
- `src/tools/<tool>/` — 工具核心纯函数（与 React 组件分离，vitest 直测）
- `src/components/` — 布局与 UI 组件
- `src-tauri/` — Rust 侧（tauri command 之下藏 Repository 等深模块）
- `.scratch/<feature>/` — 本地 issue tracker（spec + tickets）
- `CONTEXT.md` — 领域词汇表；`docs/adr/` — 架构决策记录

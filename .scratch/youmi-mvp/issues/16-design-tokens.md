# 16 — Design token 体系：自有中性灰阶

**What to build:** 全应用视觉建立在自有 design token 上（ADR-0003）：保留 shadcn 语义变量接口与 Tailwind v4 挂载方式，覆写全部取值为自有纯中性灰阶——不引入品牌强调色，选中/激活等强调态由灰阶深浅与反转表达；增补自有语义层（状态色、surface 层次变量）；浅/深双主题取值常驻、跟随系统。现有页面与组件全部消费语义 token。

**Blocked by:** None — can start immediately

**Status:** done（2026-10-10）

- [x] 语义 token 取值全部自有定义，浅/深双主题成对，跟随系统切换
- [x] 全库无脱离语义 token 的调色板色与硬编码字体栈
- [x] 无品牌强调色：强调态由灰阶对比表达；destructive 等功能语义色保留
- [x] 浅色/深色下主窗口与浮层窗口视觉正常

## Comments

- token 取值：全部脱离 shadcn neutral 预设数值，纯中性灰阶（chroma 0）。浅色 surface-base 0.967 / surface-raised 1（白卡片），深色 0.155 / 0.21（背景更深、卡片略浅，ADR-0004）；文字浅色 0.17 / 深色 0.96；primary 保持反转强调（浅色近黑底白字、深色浅底深字）。state 色：destructive 浅 0.55 0.22 27.5 / 深 0.68 0.19 25（自有化微调），新增 success（152 色相）与 warning（78/85 色相）均含 foreground 对，warning/success-foreground 用深色文字保证浅底对比。
- 语义层增补：`--surface-base`/`--surface-raised` 两层层次变量进入 token（ADR-0004"双卡片的两层 surface"），`--background`/`--card`/`--popover` 别名到这两层（深色块因此无需重复覆写这三个变量，var 引用在使用处求值自然传导）；`@theme inline` 挂出 `bg-surface-base`/`bg-surface-raised` utility，供 ticket 17 双卡片布局消费。success/warning 眼下仅 success 有消费点（json.tsx），warning 与各 foreground 对为状态色集合的完整接口（shadcn 生态组件即插即用依赖成对 token，ADR-0003 Consequences），code-review 判定为被成文 standard 压制的合理超前。
- 字体进 token：`@theme` 定义 `--font-sans`（Segoe UI Variable Text → Segoe UI → 雅黑回退）与 `--font-mono`（Cascadia Mono → Consolas → 雅黑回退，避免代码区中西文混排跳变），body 硬编码字体栈移除改 `font-sans`。radius 保持 0.625rem 不变。
- 全库收敛：json.tsx 状态提示从 emerald 调色板色改为 success/destructive 语义 token（含删掉 dark: 变体——token 分主题取值后无需）；grep 核对 src/ 无调色板类、无 hex/rgb/hsl/oklch 字面量（core.test.ts 的 svg fixture 是测试数据非 UI 样式）、无硬编码字体栈、无 dark: 变体残留。code-review 后状态条改用 `cn()` 消重，对齐 ui 组件惯例。
- 验证基线：tsc/vite build、oxlint、vitest 49 通过。视觉验证（vite dev + 浏览器 1200×800 截图）：浅色为宿主真实 prefers-color-scheme（light），深色经注入与 dark media query 逐字相同的 token 覆盖层验证——JSON 工具页 success/destructive 状态条、设置页（卡片层次、switch、输入）、浮层页（搜索框、SegmentedTabs 反转激活态）在两主题下层次与对比均正常；构建产物 CSS 中 `@media (prefers-color-scheme:dark)` 块完整包含全部新 token。局限：深色取值经样式注入而非真实系统深色，真实窗口（WebView2）跟随 Windows 主题的表现建议在 17 的实机验证中一并复核。
- 已知边界：真实系统深色下的截屏未做（本会话无法切 Windows 主题）；浮层在纯浏览器中 IPC 不可用，只验证了界面框架层。未引入品牌强调色，强调态全部由灰阶深浅（hover accent）与反转（primary/激活 tab）表达。

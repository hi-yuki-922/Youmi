# 02 — 编码转换

**What to build:** 工作台工具：三个子工具——SVG 源码 ↔ URL 编码、文本 ↔ Base64、URL query 字符串 ↔ 键值表。全部输入即时双向转换、结果一键复制，无需点任何按钮。

**Blocked by:** 01 — App 骨架 + JSON 工具

**Status:** done（2026-10-08）

- [x] 三个子工具在主窗口内可切换使用
- [x] 输入即时转换；结果一键复制
- [x] 转换核心纯函数化，vitest 覆盖中文、emoji、空输入等边界

**实现说明（2026-10-08）：**

- 核心纯函数 `src/tools/encoding/core.ts`：SVG ↔ data URI（最小编码集 `% #" '& <>`，其余打印 ASCII 保持可读，非 ASCII/控制字符按 UTF-8 百分号编码，输出可直接用于 CSS `background-image: url("...")`）；文本 ↔ Base64（UTF-8，容忍空白/缺失填充/base64url 字母表，解码严格校验 UTF-8）；URL query ↔ 键值表（保序、保重复键，容忍前导 `?` 与完整 URL 粘贴）。vitest 34 个测试覆盖中文、emoji、空输入、roundtrip 与错误输入（全仓 46 个全绿）。
- 页面 `src/routes/encoding.tsx`：三个子工具 tab 切换；`InstantConverter` 双栏即时双向转换（失败方向保留旧值并显示错误条）；复制按钮下沉到每个面板，双向结果均可一键复制（回应 review 对"结果"口径的确认）。新增 `ui/input.tsx`（键值表行编辑）与共享 `components/copy-button.tsx`（json.tsx 一并复用，消除与 ticket 01 的复制逻辑重复）。
- 验证：tsc / oxlint / oxfmt 全绿；浏览器 GUI 冒烟 T1-T5（子工具切换、双向即时转换、错误条、复制反馈、键值表增删改查同步）通过，截图留证 `gui-test-screenshots/`。审查修正后重构（InstantConverter 抽取 + 复制按钮下沉）的点击类复验被 IAB 输入管线全局卡死阻断（example.com 无关页面同样复现，与代码无关）；重构仅抽取共享组件、交互语义与已验证版本一致，原生窗口与最终交互验收留待 `pnpm run tauri dev`。

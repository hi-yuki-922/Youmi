# 04 — Kbd 组件与组合键符号格式化

**What to build:** shadcn 风格 Kbd 展示组件与组合键格式化入库：Ctrl→⌃、Shift→⇧、Alt→⌘、Win→⊞、Enter→⏎、Esc→⎋、方向键→↑↓←→；Alt 与 Win 必须可区分（不得都渲染为 ⌘）。纯函数格式化 + 组件，是后续展示票（05/06/13）共用的地基。

**Blocked by:** 无——可立即开始

**Status:** done（2026-10-16 实机验收通过）

- [x] Kbd 组件入库（风格与既有 design token 一致）
- [x] 组合键格式化符号映射完整（含多修饰组合 `super+alt+space` → ⊞ ⌘ Space）
- [x] 主键区字母/数字原样展示（如 Alt+V → ⌘ V）
- [x] vitest 覆盖格式化映射

## Comments

### 2026-10-16 — 实现说明

- **纯函数层**（`src/lib/kbd.ts`，无 tauri 依赖）：`comboSymbols(combo)` 输出 Kbd 符号 token 序列，`formatCombo(combo)` 输出文本形式（aria-label/title 用）。修饰键词汇单表 `MODIFIERS`（⌃⇧⌘⊞ 各带 symbol/label），Alt ⌘ 与 Win ⊞ 可区分；`isModifier` 导出给 `shortcuts.ts` 的 `hasRequiredModifier` 复用，组合键语法知识只此一处。
- **展示顺序**：修饰键按 super→ctrl→alt→shift 排（Win+X 类系统组合的书写惯例），与串内顺序无关——Rust 规范串 `alt+super+Space` 与票面写法 `super+alt+space` 同形为 ⊞ ⌘ Space。
- **原 shortcuts.ts 的文本 `formatCombo` 与 KEY_LABELS 迁入**（该模块收窄为纯 tauri 边界）；文本形式同步改为 Win 在前的展示序（原 `Alt+Win+Space` → 现 `Win+Alt+Space`），两格式化器保持同构。
- **组件**（`src/components/ui/Kbd.tsx`）：`Kbd` 单键位（h-5、bg-muted、border、rounded-sm，design token 语义色）、`KbdGroup` 组容器、`ComboKbd` 接组合键规范串渲染整组并以文本形式作 aria-label。
- 应用范围：本批接入设置页全局组与两浮层提示；翻译页提示由 13、剪贴板组条目由 12 接入。

### 2026-10-16 — 实机验证记录（CDP 驱动 dev 应用，截图 gui-test-screenshots/iter2-settings-kbd.png）

- 设置页全局组 Kbd 渲染断言：`["⌘","V"]`、`["⊞","⌘","Space"]`、`["⌘","T"]`——⊞ 与 ⌘ 同屏可区分，字母原样；截图目检无豆腐块。
- vitest：`kbd.test.ts` 6 例（多修饰两序同形、⏎⎋↑↓ 映射、字母/数字/标点原样、文本形式同序）+ `shortcuts.test.ts` 2 例，全绿。

# 04 — Kbd 组件与组合键符号格式化

**What to build:** shadcn 风格 Kbd 展示组件与组合键格式化入库：Ctrl→⌃、Shift→⇧、Alt→⌘、Win→⊞、Enter→⏎、Esc→⎋、方向键→↑↓←→；Alt 与 Win 必须可区分（不得都渲染为 ⌘）。纯函数格式化 + 组件，是后续展示票（05/06/13）共用的地基。

**Blocked by:** 无——可立即开始

**Status:** ready-for-agent

- [ ] Kbd 组件入库（风格与既有 design token 一致）
- [ ] 组合键格式化符号映射完整（含多修饰组合 `super+alt+space` → ⊞ ⌘ Space）
- [ ] 主键区字母/数字原样展示（如 Alt+V → ⌘ V）
- [ ] vitest 覆盖格式化映射

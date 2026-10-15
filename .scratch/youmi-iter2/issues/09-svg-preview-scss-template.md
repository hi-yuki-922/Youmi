# 09 — SVG 预览卡片与 scss 模板

**What to build:** SVG ↔ URL 编码子工具新增预览卡片与 iconify/scss 模板输出：预览实时内联渲染输入的 SVG（非法输入呈错误态不白屏）；iconName 独立输入框（默认占位 `icon`），变更实时重生成模板 `.{iconName} { @include icon; --svg: url(<data URI>); }` 并一键复制；data URI 复用现有编码输出，无新增编码逻辑。

**Blocked by:** 无——可立即开始

**Status:** ready-for-agent

- [ ] 输入合法 SVG 时预览卡片实时渲染
- [ ] 输入非法时预览区错误态，不白屏不崩溃
- [ ] iconName 输入变更实时重生成模板
- [ ] 模板格式与约定一致（`@include icon;` 固定）
- [ ] 模板一键复制
- [ ] vitest 覆盖模板生成（iconName 注入与特殊字符）

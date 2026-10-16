# 09 — SVG 预览卡片与 scss 模板

**What to build:** SVG ↔ URL 编码子工具新增预览卡片与 iconify/scss 模板输出：预览实时内联渲染输入的 SVG（非法输入呈错误态不白屏）；iconName 独立输入框（默认占位 `icon`），变更实时重生成模板 `.{iconName} { @include icon; --svg: url(<data URI>); }` 并一键复制；data URI 复用现有编码输出，无新增编码逻辑。

**Blocked by:** 无——可立即开始

**Status:** ready-for-human（2026-10-16 实现完成，vitest 全绿；预览渲染与复制待实机复验）

- [ ] 输入合法 SVG 时预览卡片实时渲染
- [ ] 输入非法时预览区错误态，不白屏不崩溃
- [ ] iconName 输入变更实时重生成模板
- [ ] 模板格式与约定一致（`@include icon;` 固定）
- [ ] 模板一键复制
- [x] vitest 覆盖模板生成（iconName 注入与特殊字符）

## Comments

### 2026-10-16 — 实现

- **纯函数**（`src/tools/encoding/core.ts`）：`normalizeIconName`（空白折叠连字符、类名非法字符替换连字符、数字开头补 `icon-` 前缀、空值回退 `icon`）与 `svgIconScss(dataUri, iconName)`（`.{name} { @include icon; --svg: url("<dataUri>"); }`）。vitest 9 例（默认名、空白折叠、特殊字符、数字开头、模板注入与 data URI 原样嵌入）。
- **预览走 `<img src={dataUri}>`** 而非内联 SVG DOM：与最终 CSS background-image 同为图像语境，所见即所得；CSP `img-src data:` 已允许；非法 SVG 由 img error 事件呈错误态文案，不白屏不注入任意 HTML。data URI 复用现有 `svgToDataUri`，无新增编码逻辑。
- **面板**（`SvgIconPanel`）：挂在 svg-url 子工具双栏下方（InstantConverter 新增 `extra` render-prop，入参为左侧 SVG 源码）；iconName 独立输入框（占位 `icon`，状态持久化 `tools.encoding.svg-icon-name`），变更实时重生成模板；模板一键复制（SVG 源码为空时禁用）。

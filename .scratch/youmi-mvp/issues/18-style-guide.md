# 18 — Style guide 撰写

**What to build:** 成文的编码风格基准 docs/style-guide.md，并从 AGENTS.md 链接，作为后续 ticket 与 code review 的 Standards 依据。内容：TS 规则——除特定场景必须用 this 外严禁 this；函数式风格，函数以箭头函数定义（`const fn = () => {}`），非必要不用 function 声明、非必要不封装 class；文件命名——tsx 大驼峰（路由文件、第三方库与自动生成文件除外），css/js/ts 与目录小写烤串；注释——按各语言规范，实事求是描述代码现状，不含 ticket/spec 上下文，附正反例。Rust 侧：rustfmt/clippy 默认 + 同一套注释规范。

**Blocked by:** None — can start immediately

**Status:** done（2026-10-10）

- [x] style guide 覆盖 TS 语言约定、Rust 约定、文件命名、注释规范
- [x] 注释规范含正反例（"客观描述现状" vs "引用 ticket/spec"）
- [x] AGENTS.md 链接 style guide，后续 agent 可发现并遵循

## Comments

- 落点：`docs/style-guide.md`（TS 语言约定 / 文件命名 / 注释通则 / Rust 约定四节），AGENTS.md 顶部新增 Style guide 小节链接之，并声明其为 ticket 实现与 code review 的 Standards 依据。工具链已强制的（oxfmt/oxlint/rustfmt/clippy）明确不重复约定。
- 规则写法上的两个边界澄清（写票时未明说，成文时补齐）：(1) tsx 大驼峰的例外清单除票面三项（路由文件、第三方库与自动生成文件）外，把构建工具约定的入口 `src/main.tsx` 一并列入——Vite 入口属框架约定；(2) 注释禁令只针对 ticket/spec 进度上下文，ADR 引用明确放行——ADR 是仓库内长期架构文档，不是进度上下文（否则 `ADR-0004` 这类既有注释全部变成违规）。
- `this` 的例外场景写成"扩展以 class 为中心的内置/第三方类型、调用方绑定 this 的回调 API、纯类型位置的 polymorphic this"，并要求就地说明理由——当前代码库零 `this`，此条是给未来留的逃生门。
- 注释正反例取自本仓库真实案例（`spec story 24` 容量注释、"ticket 08 决定共用一库"等），ticket 19 落地时已按正例改写。
- 验证：文档票无代码面；AGENTS.md 链接由 ticket 19 的全量 review（Standards 轴以本文件为依据）反向确认可用。

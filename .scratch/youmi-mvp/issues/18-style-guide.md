# 18 — Style guide 撰写

**What to build:** 成文的编码风格基准 docs/style-guide.md，并从 AGENTS.md 链接，作为后续 ticket 与 code review 的 Standards 依据。内容：TS 规则——除特定场景必须用 this 外严禁 this；函数式风格，函数以箭头函数定义（`const fn = () => {}`），非必要不用 function 声明、非必要不封装 class；文件命名——tsx 大驼峰（路由文件、第三方库与自动生成文件除外），css/js/ts 与目录小写烤串；注释——按各语言规范，实事求是描述代码现状，不含 ticket/spec 上下文，附正反例。Rust 侧：rustfmt/clippy 默认 + 同一套注释规范。

**Blocked by:** None — can start immediately

**Status:** ready-for-agent

- [ ] style guide 覆盖 TS 语言约定、Rust 约定、文件命名、注释规范
- [ ] 注释规范含正反例（"客观描述现状" vs "引用 ticket/spec"）
- [ ] AGENTS.md 链接 style guide，后续 agent 可发现并遵循

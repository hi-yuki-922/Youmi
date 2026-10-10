# Style guide

本仓库的编码风格基准，作为后续 ticket 实现与 code review 的 Standards 依据。工具链已强制的内容（oxfmt / oxlint / rustfmt / clippy）不在此重复约定；code review 的 Standards 轴以本文件为准。

## TypeScript

### 语言约定

函数式风格：

- 函数以箭头函数定义：`const fn = () => {}`。非必要不用 `function` 声明。仅当语言/工具行为强制时豁免（如需要 declaration merging、依赖声明提升），并就地说明理由。
- 非必要不封装 class。用纯函数 + 数据变换表达逻辑；React 一律函数组件。
- `this` 严禁使用，除非特定场景必须用：扩展以 class 为中心的内置/第三方类型（如继承 `Error`）、回调 API 约定由调用方绑定 `this` 时，或仅在类型位置的 polymorphic `this`。当前代码库没有这类场景；出现时必须就地说明为什么不可避。

```tsx
// 正例：箭头函数 + 显式返回类型
const formatJson = (input: string, indent: JsonIndent): JsonResult<string> => { ... };

// 反例：无必要理由的 function 声明
function formatJson(input: string, indent: JsonIndent): JsonResult<string> { ... }

// 反例：不必要的 class 封装
class JsonFormatter {
  format = (input: string): string => { ... };
}
```

### 文件命名

- `.tsx` 文件大驼峰（PascalCase）：`CopyButton.tsx`、`LabeledTextarea.tsx`。例外：
  - 路由文件（`src/routes/**`）：tanstack-router 的文件名即路由语义，遵循其自身约定；
  - 构建工具约定的入口文件（`src/main.tsx`）；
  - 第三方库与自动生成文件（如 `routeTree.gen.ts`）。
- `.css` / `.js` / `.ts` 文件与目录一律小写烤串（kebab-case）：`src/tools/clipboard/core.ts`、`src/styles/globals.css`。
- Rust 源文件与模块不受上述约束，遵循 Rust 官方 snake_case。

## 注释

两种语言同一套通则，按各语言注释规范落笔：

- TS：行内用 `//`，公开 API 的意图性说明用 JSDoc；Rust：模块说明用 `//!`，公开项的 doc comment 用 `///`，行内用 `//`。
- 实事求是描述代码现状：写这段代码是什么、受什么约束、为什么这样写（取舍、坑、前提）。不写它从哪里来、被谁要求、经历过什么改动——那些属于 issue tracker，不随代码走。
- **不含 ticket/spec 上下文**：不引用 ticket 编号、spec 条目编号、"按 XX 的要求"。

```rust
// 反例：引用 spec 条目，读代码的人得不到容量语义本身
/// 默认容量（spec story 24）。

// 正例：客观陈述该常量在代码中的真实作用
/// 默认容量：超出后按 recency 淘汰最旧条目。
```

```rust
// 反例：引用 ticket 作为理由
// ticket 08 决定密文与历史共用一库

// 正例：陈述现状与约束
// 剪贴板历史与密码本密文共用一库；新增 schema 一律在 MIGRATIONS 末尾追加，禁止改动已有迁移。
```

```tsx
// 反例：叙述改动历史
// 原默认 Win+V，注册失败后改成 Alt+V

// 正例：说明当下成立的约束（不提改动过程）
// 默认 Alt+V：Win+V 与系统剪贴板历史固定绑定，无法可靠覆盖。
```

## Rust

- rustfmt 默认配置（`cargo fmt`），不引入自定义 `rustfmt.toml`；clippy 用默认 lint 集，不开 extra / pedantic。
- 语言风格沿用 Rust 社区惯例（impl 块、trait、所有权语义），不受上方 TS 函数式约定的约束。
- 注释遵循上方通则。

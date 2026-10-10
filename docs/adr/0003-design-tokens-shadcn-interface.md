# Design token 保留 shadcn 语义变量接口，取值自有：纯中性灰阶、无品牌强调色

Youmi 要一套自有视觉体系，不直接沿用 shadcn 预设；但前端已按 shadcn 语义变量（`--background`、`--primary`、`--radius` 等，oklch）组织样式，手写 ui 组件与全部页面均消费这套变量。决策：保留 shadcn 的语义变量命名与 Tailwind v4 `@theme inline` 挂载方式作为接口，覆写全部取值，并增补自有语义层（状态色、层次 surface 变量）；色彩方向为纯黑白灰极简——不引入品牌强调色，层次与强调态由灰阶深浅与反转表达；功能性语义色（如 destructive）保留。暗色主题继续跟随系统（`prefers-color-scheme`），双主题取值常驻定义，不提供手动切换。

## Considered Options

- 完全自有命名体系、脱离 shadcn 约定 —— 被拒：全部组件与用法要迁移，且失去 shadcn 生态组件的即插即用，成本买不回收益
- 直接沿用 shadcn neutral 预设取值 —— 被拒：即"使用 shadcn 自带预设"，无自有体系可言
- 中性灰 + 单一品牌强调色 —— 被拒（2026-10-09）：工具集合优先中性视觉，强调态由灰阶反转表达；日后若需品牌色，覆写少数变量即可

## Consequences

- shadcn 生态组件可直接引入，落在 token 上即风格一致
- 选中/激活等强调态只能靠灰阶对比表达，视觉自由度收窄是有意取舍
- 暗色手动切换日后作为设置项加入时，token 机制不变

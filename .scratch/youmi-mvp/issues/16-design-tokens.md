# 16 — Design token 体系：自有中性灰阶

**What to build:** 全应用视觉建立在自有 design token 上（ADR-0003）：保留 shadcn 语义变量接口与 Tailwind v4 挂载方式，覆写全部取值为自有纯中性灰阶——不引入品牌强调色，选中/激活等强调态由灰阶深浅与反转表达；增补自有语义层（状态色、surface 层次变量）；浅/深双主题取值常驻、跟随系统。现有页面与组件全部消费语义 token。

**Blocked by:** None — can start immediately

**Status:** ready-for-agent

- [ ] 语义 token 取值全部自有定义，浅/深双主题成对，跟随系统切换
- [ ] 全库无脱离语义 token 的调色板色与硬编码字体栈
- [ ] 无品牌强调色：强调态由灰阶对比表达；destructive 等功能语义色保留
- [ ] 浅色/深色下主窗口与浮层窗口视觉正常

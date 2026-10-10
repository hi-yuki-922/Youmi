# 19 — 全量 code review + 风格修正

**What to build:** 以 style guide（18）为 Standards 轴、以各 ticket 为 Spec 轴，对 MVP 规划完成点以来的全部代码提交做整体 review；风格类发现当场修正，包括 components 目录下 tsx 文件重命名为大驼峰（Windows 大小写不敏感文件系统上用 git 两段式改名保历史）。超出风格范围的结构性问题记录为新票，不顺手改。

**Blocked by:** 15 — Alt+V 热修；16 — design token 体系；17 — 自定义标题栏与双卡片布局；18 — style guide

**Status:** done（2026-10-10）

- [x] review 报告产出：Standards + Spec 两轴发现并列
- [x] 风格类发现全部修正，lint / fmt / 前后端测试全绿
- [x] components 目录 tsx 均为大驼峰命名，导入更新完整、构建通过
- [x] 超出风格范围的结构性发现已立票或记录

## Comments

### Review 报告（fixed point `8f36858` = MVP 规划完成点，22 个提交，103 文件）

双轴并行子代理 review，发现并列如下。

**Standards 轴**（依据 = ticket 18 产出的 docs/style-guide.md + Fowler smell baseline）：

- 硬违规 ×3：① `components/**` 全部 10 个 tsx + `router.tsx` 为 kebab 命名；② 全 src 约 45 处 `function` 声明非箭头函数（组件、lib、tools 全中招）；③ Rust 注释引用 ticket/spec 共 5 处（overlay.rs 还引用了 ticket 编号 06/09/11，双轴里我方 grep 补齐）。
- 合规面：零 `this`、零 class；oxlint/oxfmt 管辖项无发现。
- smell（judgement call，不当场改）：InstantConverter 四个 label/placeholder prop 恒同行（Data Clumps）；ErrorStrip 与 json 状态条同形状两次实现（Duplicated Code）；ConvertResult message 直连 UI 文案（Primitive Obsession，该规模可接受）；shadcn 的 `Select = SelectPrimitive.Root` 转发属仓库自选惯例，baseline 被 repo standard 压制。

**Spec 轴**（依据 = spec.md + 各 done ticket；07–14 为 open，不计缺失）：

- 忠实度高：clipboard-win Monitor、单实例首个注册、关窗隐藏到托盘、浮层 hide 不销毁、store 设置、PNG blob、recency 去重刷新、Alt+V 默认、中文 UI、主题跟随系统——逐项与 spec 决策核对无冲突；无提前实现 open ticket 的越界。
- 结构性发现 ×3：story 25 的"清空历史"无 UI 入口（`clipboard_clear` 命令已成死代码）→ 票 20；`_main.tsx` 启动 `void applyClipboardCapacity()` 引擎降级时 rejection 被静默吞掉 → 票 21；`CopyButton` 走 `navigator.clipboard` 绕过"前端只经 tauri command 访问 OS 能力"的 seam 决策，且与 story 26 的写入语义相关，需决策 → 票 22。
- scope creep 核对：diff 语言选择器、托盘左键恢复、`clipboard_write_own_image_png`、`--warning` token 未用、npm→pnpm——均有 ticket 记录背书，不算越界。

### 修正落地（commit 1cd70bf + review 修复）

- 12 个 tsx 两段式改名（`git mv` 经临时名，Windows 大小写不敏感盘上保历史，`git log --follow` 可达首个提交）：`components/**` 全部 + `router.tsx`（后者超出票面"components 目录下"的字面范围，但为 ticket 18 命名规则的直接推论——非路由/非入口/非生成物）；导入全部更新，构建通过。
- 约 45 处 `function` 声明转箭头函数。两个连带调整：路由文件 `export const Route = createFileRoute(...)` 移到组件定义之后（const 无提升，原地引用成 TDZ）；`SegmentedTabs` 泛型箭头用 `<T extends string,>` 消歧 JSX。
- Rust 注释 5 处改写为客观现状描述（如 `/// 默认容量（spec story 24）。` → `/// 默认容量：超出后按 recency 淘汰最旧条目。`）。
- 本 session 变更再过一轮双轴 review：无硬违规；3 个小修正当场落地——overlay.rs 模块注释把未实现的命令面板/划词翻译说成"共用"（改"设计为复用"）、style guide 箭头豁免清单补"模块顶层必须先引用后声明"场景、正例注释去掉暗示返回类型强制的话术。
- smell 类发现立票 23（低优先级集中记录）；新票 20/21/22/23 均 `needs-triage`，由人排优先级。

### 验证基线

tsc、oxlint（0 警告 0 错误）、oxfmt、vitest 49、cargo fmt --check、cargo test 34（4 个 `#[ignore]` smoke 属预期）、`npm run build`（tsc + vite build）全绿。chunk >500kB 警告为既有状态，非本次引入。

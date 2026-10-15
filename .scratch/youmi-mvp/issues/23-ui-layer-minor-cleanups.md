# 23 — UI 层轻微结构清理（review smell 记录，低优先级）

**What to build:** ticket 19 全量 review Standards 轴记录的三个 judgement-call 级 smell，均不阻塞、不顺手改，集中记录在此，供后续触碰相关文件时一并清理：(1) `InstantConverter` 的 `leftLabel/leftPlaceholder/rightLabel/rightPlaceholder` 四个 prop 恒同行——收拢为两侧各一个 `{ label, placeholder }` 对象；(2) `encoding.tsx` 的 `ErrorStrip` 与 `json.tsx` 的状态条是同一"结果反馈条"形状的两次实现——抽一个共享组件（注意 json 侧有 success/error 两态与行列定位，不是纯错误条）；(3) `ConvertResult<T>` 的 `message: string` 直接面向 UI 文案，核心层与展示层耦合——若日后做多语言或结构化错误再升级。

**Blocked by:** None — can start immediately

**Status:** ready-for-human（2026-10-15）

**来源:** ticket 19 全量 review，Standards 轴 smell baseline（Data Clumps / Duplicated Code / Primitive Obsession，均为 judgement call）。

- [x] (1)(2) 清理完成或明确 wontfix
- [x] (3) 维持现状或升级为结构化错误类型

## Comments

### 2026-10-15 — 实现说明

- **(1) Data Clumps 清理**：`InstantConverter` 改收 `left` / `right` 两个 `{ label, placeholder }` 对象（`ConverterSide` 类型），两个调用点同步更新；state 变量相应改名 `leftText`/`rightText` 避免与 prop 撞名。
- **(2) Duplicated Code 清理**：新增 `src/components/ui/FeedbackStrip.tsx`——success/error 两态的"结果反馈条"（样式与改前逐类名一致）。encoding 的 `ErrorStrip` 删除，两处调用改 `{error && <FeedbackStrip kind="error">…}`；json 的内联状态条换用 `FeedbackStrip`，行列定位文案拼接留在页面侧（strip 只管形态，不管消息内容）。
- **(3) 维持现状**：`ConvertResult.message: string` 保留。理由：单语言自用工具、核心层仅 encoding/json 两个消费方、错误文案只此一处直出，无多语言/结构化需求的现实信号；ticket 本身即把"维持现状"列为可选项。触发升级的条件留痕：出现第二语言、或需要按错误类型分支处理（如重试/定位跳转）时，再引入结构化错误类型。

### 2026-10-15 — code review 记录（双轴）

- Standards：纯重构与改前渲染一致；`FeedbackStrip` 命名与 ui/ 目录惯例一致。encoding 页两处一行式反馈条调用属可接受残留。
- Spec：(2) 的注意项（json 侧两态 + 行列定位）被保留；(3) 维持现状是票面允许选项。无缺失。

### 实机验收清单（待人工）

1. 编码转换三个子工具：右栏输入非法输入（如坏 Base64）出现红色错误条，修正后消失；布局与改前无肉眼差异。
2. JSON 工具：格式化/压缩/转义成功出现绿色成功条；校验非法 JSON 出现红色条且带"第 X 行第 Y 列"定位。
3. Query 键值表：非法 query 出现错误条。

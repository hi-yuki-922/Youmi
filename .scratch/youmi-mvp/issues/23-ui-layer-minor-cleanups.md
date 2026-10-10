# 23 — UI 层轻微结构清理（review smell 记录，低优先级）

**What to build:** ticket 19 全量 review Standards 轴记录的三个 judgement-call 级 smell，均不阻塞、不顺手改，集中记录在此，供后续触碰相关文件时一并清理：(1) `InstantConverter` 的 `leftLabel/leftPlaceholder/rightLabel/rightPlaceholder` 四个 prop 恒同行——收拢为两侧各一个 `{ label, placeholder }` 对象；(2) `encoding.tsx` 的 `ErrorStrip` 与 `json.tsx` 的状态条是同一"结果反馈条"形状的两次实现——抽一个共享组件（注意 json 侧有 success/error 两态与行列定位，不是纯错误条）；(3) `ConvertResult<T>` 的 `message: string` 直接面向 UI 文案，核心层与展示层耦合——若日后做多语言或结构化错误再升级。

**Blocked by:** None — can start immediately

**Status:** needs-triage

**来源:** ticket 19 全量 review，Standards 轴 smell baseline（Data Clumps / Duplicated Code / Primitive Obsession，均为 judgement call）。

- [ ] (1)(2) 清理完成或明确 wontfix
- [ ] (3) 维持现状或升级为结构化错误类型

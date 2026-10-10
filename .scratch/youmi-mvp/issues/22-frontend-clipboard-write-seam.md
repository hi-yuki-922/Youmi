# 22 — 前端写剪贴板绕过平台 seam，需要决策

**What to build:** spec Implementation Decisions 说"前端只经 tauri command 访问 OS 能力与存储"，但 `CopyButton` 用 `navigator.clipboard.writeText` 写剪贴板，未经 Rust 端 `ClipboardAccess`。当前后果：工具页的复制会被自家监听器当作用户复制收入历史（对工作台工具可接受甚至符合直觉）；但密码本的"复制密码不入史 + 30 秒清除"（story 26）必须走 Rust 写路径，届时两条写路径并存。需要决策并落地其一：(a) 所有前端写剪贴板统一走 tauri command（带"是否入史"语义）；(b) 保留 navigator 写路径，把取舍写成 ADR。

**Blocked by:** None — can start immediately（建议与 ticket 08 密码本一起决策）

**Status:** needs-triage

**来源:** ticket 19 全量 review，Spec 轴发现（与 spec 架构决策冲突）。

- [ ] 决策落地：写路径统一或 ADR 记录取舍
- [ ] story 26 的写入语义不被该分歧破坏

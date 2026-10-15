# 22 — 前端写剪贴板绕过平台 seam，需要决策

**What to build:** spec Implementation Decisions 说"前端只经 tauri command 访问 OS 能力与存储"，但 `CopyButton` 用 `navigator.clipboard.writeText` 写剪贴板，未经 Rust 端 `ClipboardAccess`。当前后果：工具页的复制会被自家监听器当作用户复制收入历史（对工作台工具可接受甚至符合直觉）；但密码本的"复制密码不入史 + 30 秒清除"（story 26）必须走 Rust 写路径，届时两条写路径并存。需要决策并落地其一：(a) 所有前端写剪贴板统一走 tauri command（带"是否入史"语义）；(b) 保留 navigator 写路径，把取舍写成 ADR。

**Blocked by:** None — can start immediately（建议与 ticket 08 密码本一起决策）

**Status:** ready-for-human（2026-10-15）

**来源:** ticket 19 全量 review，Spec 轴发现（与 spec 架构决策冲突）。

- [x] 决策落地：写路径统一或 ADR 记录取舍
- [x] story 26 的写入语义不被该分歧破坏

## Comments

### 2026-10-15 — 决策与实现（选 (a)：统一走 tauri command）

- **决策理由**：spec 明文"前端只经 tauri command 访问 OS 能力与存储"，选 (a) 让该决策字面成立、不留例外备忘；"是否入史"两条通道在 Rust 侧本就存在（自家通道 `clipboard_write_own_text` 已上线），补一条用户侧入史通道即闭环。写命令做成**无状态**（直接用平台 `WinClipboardAccess`，不取 `ClipboardState`），引擎降级时工具页复制仍可用、只是不入史——与 navigator 时代行为一致，不因统一而引入降级回归。
- **实现**：新增 `clipboard_write_text` command（写文本，监听器照常捕获入史）+ 前端 `writeClipboardText()`；`CopyButton` 弃用 `navigator.clipboard.writeText` 改调它，并补失败 `console.error`（失败时不置"已复制"态）。至此前端仅两条写路径：`clipboard_write_text`（入史，工具页复制）与 `clipboard_write_own_text`（不入史，面板计算结果复制/划词/密码），语义由 command 划分。
- **story 26 不受影响**：密码本 `copyVaultField` → `vault_copy` → 自家通道（不入史 + 30 秒清除）原样未动；`grep` 确认前端无 `navigator.clipboard` 残留。
- **不需要 ADR**：(a) 是向 spec 收敛而非引入新取舍；两条通道的语义已由 command 边界与注释承载，CONTEXT.md 补录「自家写入通道」词条稳定命名。

### 2026-10-15 — code review 记录（双轴）

- Spec：统一路径经平台 seam、入史/不入史由两条 command 区分，符合选项 (a)；已知边界——CopyButton 写失败无用户可见反馈（仅 console），本票未要求，留待后续触碰时再定（如按钮文案瞬时"复制失败"）。
- Standards：无硬违规。

### 实机验收清单（待人工）

1. 编码转换 / JSON 工具页点复制：目标应用可粘贴；剪贴板浮层（Alt+V）里出现该条目（入史语义与改前一致）。
2. 命令面板算式结果回车复制：剪贴板有结果，但浮层历史里**不**出现（自家通道语义不变）。
3. 密码本复制密码：不入历史、30 秒后剪贴板被清（story 26 回归）。

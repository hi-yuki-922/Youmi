# 21 — 启动时 applyClipboardCapacity 失败被静默吞掉

**What to build:** `_main.tsx` 挂载时 `void applyClipboardCapacity()`——若剪贴板引擎初始化失败（lib.rs 降级路径不持有 `ClipboardState`），`clipboard_set_capacity` command 会 reject，该 rejection 无人处理：每次启动静默失败，设置页的历史容量行却看起来一切正常。为启动应用容量的路径补失败处理（console 告警起步即可），并明确引擎降级时设置页容量行的呈现语义（禁用/隐藏或保留报错）。

**Blocked by:** None — can start immediately

**Status:** done（2026-10-15 实机验收通过）

**来源:** ticket 19 全量 review，Spec 轴发现。

- [x] 启动应用容量失败不再是无告警的未处理 rejection
- [x] 引擎降级时设置页容量行有明确的失效呈现

## Comments

### 2026-10-15 — 实现说明

- **启动路径告警**：`_main.tsx` 挂载 effect 里 `applyClipboardCapacity()` 与 `applyVaultLockTimeout()` 各补 `.catch` + `console.warn`（附"可能不可用"提示）。后者严格说超出本票范围，但与容量路径是完全同构的未处理 rejection（vault 初始化失败时 `vault_set_lock_timeout` 同样 reject），属同一 review 发现的良性推广，故一并修复。
- **降级探测**：新增 `clipboard_engine_available` command（`try_state::<ClipboardState>().is_some()`，与 lib.rs 降级路径的判定同源），前端 `getClipboardEngineAvailable()` 桥接。
- **设置页呈现语义（选"禁用 + 保留报错"）**：历史容量行挂载时并行读持久化值与探测引擎；引擎降级时输入禁用、行内显示"剪贴板引擎未启用，历史容量设置不可用"，持久化值仍回显（重启恢复后仍有效）。状态建模为 `engine: "probing" | "up" | "down"` 单一状态（review 后由布尔对收拢而来），容量读取失败补 `console.error`。

### 2026-10-15 — code review 记录（双轴）

- Standards：无硬违规。judgement call 中已修复容量行状态建模；启动路径两段同构 catch（第三处出现再抽 helper）与 CopyButton 失败仅 console（如实不置"已复制"态）维持现状。
- Spec：vault 超时路径的越界修复如上已说明；实现与票面要求逐条对应，无缺失。

### 实机验收清单（待人工）

1. 正常启动：设置页历史容量行可编辑，改值失焦后生效（回归）。
2. 人为制造引擎降级（如临时占用/破坏 youmi.db 后启动）：控制台出现"启动应用剪贴板历史容量失败"告警（不再是未处理 rejection）；设置页容量行输入禁用并显示失效文案；密码本超时路径同理有告警。
3. 恢复正常后重启：容量行恢复可编辑，持久化值不丢。

### 实机验收通过置 done（2026-10-15）

用户确认验收通过，上述清单确认无问题，置 done。

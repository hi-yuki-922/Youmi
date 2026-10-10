# 21 — 启动时 applyClipboardCapacity 失败被静默吞掉

**What to build:** `_main.tsx` 挂载时 `void applyClipboardCapacity()`——若剪贴板引擎初始化失败（lib.rs 降级路径不持有 `ClipboardState`），`clipboard_set_capacity` command 会 reject，该 rejection 无人处理：每次启动静默失败，设置页的历史容量行却看起来一切正常。为启动应用容量的路径补失败处理（console 告警起步即可），并明确引擎降级时设置页容量行的呈现语义（禁用/隐藏或保留报错）。

**Blocked by:** None — can start immediately

**Status:** needs-triage

**来源:** ticket 19 全量 review，Spec 轴发现。

- [ ] 启动应用容量失败不再是无告警的未处理 rejection
- [ ] 引擎降级时设置页容量行有明确的失效呈现

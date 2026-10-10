# 15 — 剪贴板浮层默认快捷键改为 Alt+V

**What to build:** 用户按 Alt+V 呼出/收起剪贴板浮层，重启后保持；设置页引导文案与代码注释同步为现状——不再出现 Win+V 与"关闭系统剪贴板历史"的引导，代码注释只客观描述现状、不含 ticket/spec 上下文。快捷键配置化不在本票（属 ticket 12）。

**Blocked by:** None — can start immediately

**Status:** done（2026-10-10）

- [x] Alt+V 呼出/收起剪贴板浮层，重启后仍为 Alt+V；Win+V 不再被注册
- [x] 设置页引导文案与实际快捷键一致，不再引导用户关闭系统剪贴板历史
- [x] 相关代码注释合规：客观描述现状，无 ticket/spec 引用

## Comments

- Rust：`CLIPBOARD_OVERLAY_HOTKEY` 改为 `alt+v`（编译期常量，"重启后保持"由构建保证，无需持久化层）；注册失败的降级日志与注释同步去掉 Win+V 语境——失败原因客观化为"可能被其他程序占用"，仍只降级不阻塞启动。注册点全仓唯一（`setup_clipboard_overlay`），Win+V 无任何残留注册。
- 设置页：引导行改为「呼不出剪贴板浮层？/ 按 Alt+V 呼出/收起剪贴板浮层；无效时通常是 Alt+V 被其他程序占用」，不再出现 Win+V 与"关闭系统剪贴板历史"引导。为静态解释性文案，不含冲突检测/改键（属 ticket 12）。
- 注释合规：lib.rs 两处注释（常量、注册降级）均客观描述现状，无 ticket/spec 引用。code-review 双轴（Standards/Spec）通过：无硬违规；scope creep 审查判定设置文案属 ticket:10 要求的"文案与实际一致"，非提前实现 ticket 12。
- 验证：cargo test 34、vitest 49、oxlint、tsc + vite build、cargo fmt/clippy 全干净。实机冒烟（dev 构建，PowerShell SendInput 合成键击驱动真实全局快捷键路径 + Win32 枚举窗口可见性判定）：启动日志无注册失败（Alt+V 注册成功）；Alt+V 连按 hidden→visible→hidden→visible，呼出/收起正确；完全停止进程后重启，Alt+V 依旧注册并正常呼出（重启保持）；设置页经 dev server 渲染快照确认新文案且无旧引导残留；grep 确认 src/ 与 src-tauri/src/ 无 Win+V/super+v 引用。
- 已知边界：① Alt+V 字面量在 lib.rs 常量与设置页文案两处出现（沿自旧 Win+V 文案的既有形状），单一事实来源留待 ticket 12 快捷键配置化时统一。② 合成键击验证覆盖到"注册成功 + toggle 生效"为止；真人物理键击属手动验收范畴。

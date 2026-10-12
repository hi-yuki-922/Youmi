# 07 — 剪贴板回帖

**What to build:** 剪贴板浮层的核心闭环：在浮层中选中条目回车/双击后——浮层隐藏、焦点回到此前聚焦的窗口、条目内容粘贴到该窗口光标处。图片条目同样可回帖。回帖过程不得污染用户原剪贴板内容与历史。

**Blocked by:** 06 — 剪贴板浮层 v1

**Status:** done（2026-10-12）

- [x] 在任意应用复制 → Win+V → 选中条目回车：内容粘贴进原应用光标处（默认快捷键已改 Alt+V，见 spec Further Notes）
- [x] 图片条目可回帖（粘贴为图片）
- [x] 回帖后用户原剪贴板内容与剪贴板历史状态不被破坏
- [x] 无前一聚焦窗口可回帖时（如直接从托盘呼出）有合理降级行为

## Comments

- 平台 seam：`platform::PastebackAccess`（ADR-0001 回帖模拟输入 seam，trait 已入 ADR-0001 seam 清单）——前台窗口句柄/窗口进程归属/置前台/模拟 Ctrl+V（SendInput vkey 序列）。Windows 实现 `platform/windows/pasteback.rs`，前台窗口有 `#[ignore]` 只读 smoke；send_paste 会向用户前台注入键击，不设自动 smoke，靠手动验收。
- 回帖编排：`clipboard/pasteback.rs`——流程为「条目经自家写入通道写剪贴板（不新增/不刷新历史）→ 浮层隐藏 → `SetForegroundWindow` 交还焦点 → 100ms 等待后校验前台确为目标 → SendInput Ctrl+V → 500ms 后恢复用户原剪贴板」。恢复带剪贴板序号守卫：延迟窗口内用户复制了新内容则放弃恢复；原内容不可读（文件等未支持格式）或平台无序号时不盲目恢复。
- 目标窗口捕获：全局快捷键 handler 在 `overlay::toggle` 前调 `pasteback::capture_current_target` 记录当时前台窗口；自家窗口（主窗口/浮层）不作目标，因此"浮层在前台时按 Alt+V 收起"这一分支恰好把过期目标清为无目标。回帖时再校验窗口存活与进程归属（呼起到回帖之间目标可能关闭）。
- 降级行为（`copied-only`）：无目标/目标已关闭/目标是自家窗口/置前台失败/前台校验失败/键击注入失败时，条目仍写入剪贴板（自家通道）供手动 Ctrl+V，浮层照常隐藏，不模拟键击。降级场景没有发生粘贴，故不做原剪贴板恢复（条目需留在剪贴板）。前端桥接 `PastebackOutcome = "pasted" | "copied-only"`，降级不打扰用户。
- 前端：浮层列表行可聚焦（tabIndex + focus 高亮），↑↓ 在行间移动（搜索框 ↓ 进入列表）、回车回帖、双击回帖；行内删除按钮的回车/双击不受行级回帖影响；底部新增一行快捷键提示。回帖 mutation 不 invalidate 历史（回帖设计上不改变历史）。
- 测试：pasteback 编排 14 例（fake 平台 seam 注入：粘贴+恢复、图片条目、无目标/自家窗口/已关闭窗口/置前台失败/前台被抢/键击失败六种降级、恢复窗口期新复制不覆盖、条目不存在报错、目标捕获 4 例）；repository `entry_content` 1 例。cargo test 49 通过（5 个真实剪贴板/前台 smoke `#[ignore]` 手动），cargo fmt/clippy 干净；前端 tsc/vite build/oxlint 干净、vitest 49 通过。
- 实机验证（Windows，dev 实跑）：PS WinForms 可读文本框作为粘贴目标 + SendKeys 驱动——复制 old → 复制 new → 目标窗体置前台 → Alt+V 呼出 → ↓↓ 选中 old → 回车：**目标文本框收到 old 条目内容**（证明 热键捕获目标→浮层呼出→键盘选中→回帖写剪贴板→浮层隐藏→焦点交还→Ctrl+V 全链路），回帖后剪贴板为回帖前内容（恢复语义正确；该场景回帖前剪贴板恰为另一测试串，"恢复不同内容"由单测确定性覆盖）。验证脚本存 `.scratch/youmi-mvp/verify/`（paste-target.ps1 + verify-pasteback.ps1，ASCII，可复跑）。后两轮重跑时目标窗体抢不到前台（机器正被人工使用，焦点竞争），为不打扰停止了进一步的按键驱动；图片条目端到端与"托盘直启降级"留手动验收。
- 图片回帖证据：真实剪贴板 smoke 全过（`cargo test -- --ignored --test-threads=1`，5 例）——`image_write_read_roundtrip` 证明 PNG→CF_BITMAP 真实写入/读回可用（"粘贴为图片"的写入半边）；`end_to_end_capture_and_own_write_channel` 证明回帖所依赖的自家写入通道端到端不入史；新回帖 seam 的前台窗口读取 smoke 同过。接收半边（目标应用收到图片粘贴）与文本路径共用已验证的 Ctrl+V 机制。
- code-review（Standards+Spec 双轴）修复：lib.rs 注释去掉 ticket 编号引用；`valid_target`/`capture_target` 抽出共用谓词 `is_foreign_window`；`entry_content` 用 `ClipboardKind` 解析代替 kind 字面量匹配。Spec 轴保留意见已记录于"已知边界"：500ms 恢复延迟的概率性时序（边界①）与 copied-only 不恢复语义（降级场景无粘贴，无需还原）。
- 已知边界：① 恢复延迟 500ms 是固定值——目标应用超时后才读剪贴板会拿到用户原内容（序号守卫使其不破坏用户数据，但该极端时序可能粘贴失败）；② 管理员权限窗口无法接收键击/被置前台（UIPI），走 copied-only 降级；③ 原剪贴板内容为文件列表等未支持格式时放弃恢复（条目留在剪贴板）；④ 回帖后浮层由 Rust 侧 hide，与失焦 hide 共存无冲突。

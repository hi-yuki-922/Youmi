# 13 — MVP 验收 + 打包冒烟

**What to build:** 收尾 ticket：对照 spec（`.scratch/youmi-mvp/spec.md`）的 user stories 逐条手动验收；`tauri build` 产物做安装/运行冒烟（无签名、无 updater）；遗留问题记录回 tracker。

**Blocked by:** 02 — 编码转换；03 — 文本 diff；07 — 剪贴板回帖；08 — 密码本；10 — 启动应用；12 — 快捷键改键 + 冲突检测

**Status:** ready-for-agent

- [ ] spec 全部 user stories 手动验收通过；未达项逐条记录到 tracker 并注明原因
- [ ] `tauri build` 产物在本机可安装运行，核心链路（四类工作台工具 + 三个全局工具）冒烟通过
- [ ] oxfmt 若因 beta 问题阻碍日常格式化，执行"回退 prettier、保留 oxlint"并记录决策

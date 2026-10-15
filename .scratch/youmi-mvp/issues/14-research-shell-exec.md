# 14 — 调研：命令面板 shell 执行（可选）

**What to build:** 调研 ticket（spec 待定项）：评估命令面板执行 shell 命令的安全模型与实现路径（白名单？确认弹层？工作目录语义？），产出"做/不做/怎么做"结论写入本 ticket 的 Answer 区。不 block MVP。

**Blocked by:** 09 — 命令面板 v1

**Status:** wontfix

- [ ] 结论与安全边界建议写入本 ticket `## Answer` 下，`Status` 改为 `resolved`

## Comments

- 2026-10-15 — 决定 MVP 阶段不做命令面板执行 shell 命令，本调研票跳过（`wontfix`）。MVP 收尾时全部其他 ticket 均已 done（见 13 验收票）。将来若重启此方向（白名单 / 确认弹层 / 工作目录语义的安全模型调研），直接把 `Status` 改回 `ready-for-agent` 并基于本票原文继续即可。

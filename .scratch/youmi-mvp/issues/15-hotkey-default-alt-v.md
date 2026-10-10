# 15 — 剪贴板浮层默认快捷键改为 Alt+V

**What to build:** 用户按 Alt+V 呼出/收起剪贴板浮层，重启后保持；设置页引导文案与代码注释同步为现状——不再出现 Win+V 与"关闭系统剪贴板历史"的引导，代码注释只客观描述现状、不含 ticket/spec 上下文。快捷键配置化不在本票（属 ticket 12）。

**Blocked by:** None — can start immediately

**Status:** ready-for-agent

- [ ] Alt+V 呼出/收起剪贴板浮层，重启后仍为 Alt+V；Win+V 不再被注册
- [ ] 设置页引导文案与实际快捷键一致，不再引导用户关闭系统剪贴板历史
- [ ] 相关代码注释合规：客观描述现状，无 ticket/spec 引用

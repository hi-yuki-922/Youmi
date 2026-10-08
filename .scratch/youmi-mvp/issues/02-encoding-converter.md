# 02 — 编码转换

**What to build:** 工作台工具：三个子工具——SVG 源码 ↔ URL 编码、文本 ↔ Base64、URL query 字符串 ↔ 键值表。全部输入即时双向转换、结果一键复制，无需点任何按钮。

**Blocked by:** 01 — App 骨架 + JSON 工具

**Status:** ready-for-agent

- [ ] 三个子工具在主窗口内可切换使用
- [ ] 输入即时转换；结果一键复制
- [ ] 转换核心纯函数化，vitest 覆盖中文、emoji、空输入等边界

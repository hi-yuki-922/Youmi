# 05 — 剪贴板捕获与存储引擎

**What to build:** Rust 侧引擎（本 ticket 无 UI，以 cargo test 验收）：用 clipboard-win 监听剪贴板变化，文本与图片（存为 PNG）写入 SQLite 剪贴板历史——重复内容刷新原条目到顶、容量满淘汰最旧、Youmi 自身写入的内容（后续的密码复制、取词模拟复制）经"自家写入通道"标记排除；提供查询/删除的 tauri command。监听能力藏在 ADR-0001 的平台 seam 后，仅实现 Windows。

**Blocked by:** 01 — App 骨架 + JSON 工具

**Status:** ready-for-agent

- [ ] 复制文本/图片自动入库（cargo test 验证 repository 行为）
- [ ] 去重刷新：相同内容重复复制只保留一条且置顶
- [ ] 容量控制：默认 1000 条（可配置），超出淘汰最旧
- [ ] 自家写入通道：经该通道写剪贴板的内容不进入历史
- [ ] 查询接口支持关键词搜索、按类型（文本/图片）过滤、删除单条/清空
- [ ] SQLite migration 机制就位（rusqlite），密码本表结构留待 08 扩展

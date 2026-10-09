# 05 — 剪贴板捕获与存储引擎

**What to build:** Rust 侧引擎（本 ticket 无 UI，以 cargo test 验收）：用 clipboard-win 监听剪贴板变化，文本与图片（存为 PNG）写入 SQLite 剪贴板历史——重复内容刷新原条目到顶、容量满淘汰最旧、Youmi 自身写入的内容（后续的密码复制、取词模拟复制）经"自家写入通道"标记排除；提供查询/删除的 tauri command。监听能力藏在 ADR-0001 的平台 seam 后，仅实现 Windows。

**Blocked by:** 01 — App 骨架 + JSON 工具

**Status:** done（2026-10-09）

- [x] 复制文本/图片自动入库（cargo test 验证 repository 行为）
- [x] 去重刷新：相同内容重复复制只保留一条且置顶
- [x] 容量控制：默认 1000 条（可配置），超出淘汰最旧
- [x] 自家写入通道：经该通道写剪贴板的内容不进入历史
- [x] 查询接口支持关键词搜索、按类型（文本/图片）过滤、删除单条/清空
- [x] SQLite migration 机制就位（rusqlite），密码本表结构留待 08 扩展

## Comments

- 模块结构：`src-tauri/src/db.rs`（rusqlite bundled + 顺序迁移，`MIGRATIONS[n]` 把 schema 从 n 升到 n+1，08 的密码本表追加 V2 即可）；`clipboard/model.rs`（`ClipboardContent::{Text,ImagePng}` + SHA-256 指纹）；`clipboard/repository.rs`（深模块：capture 去重刷新/容量淘汰在一个事务里，query 关键词 LIKE 转义 + 类型过滤、`image_data` 按需取 PNG blob、delete/clear/len）；`clipboard/engine.rs`（捕获管线 + 自家写入通道）；`clipboard/commands.rs` + `lib.rs` 接线（7 个 tauri command：query/get_image/delete/clear/set_capacity/write_own_text/write_own_image_png；初始化失败仅日志降级不阻塞启动）。
- 平台 seam（ADR-0001 clipboard-listener）：`platform/mod.rs` 声明 `ClipboardAccess`（read/write_text/write_image_png/sequence）+ `ClipboardWatcher`（阻塞式 watch，须在专用线程调用），Windows 实现在 `platform/windows/`。未来移植 = 逐 seam 补第二实现。
- 去重与排序：`(kind, content_hash)` 唯一索引 + `INSERT .. ON CONFLICT DO UPDATE`，`recency` 单调递增（MAX+1）承担排序，不受系统时钟回拨影响；刷新保留原行（id、created_at 不变），仅 updated_at/recency 更新。容量默认 1000，`clipboard_set_capacity` 运行时可调；淘汰 = 删除 recency 不在 top-N 的行，被刷新过的条目因此免于淘汰。
- 图片链路：clipboard-win 的 `Bitmap` 格式字节 = 带 BITMAPFILEHEADER 的 BMP 文件；`platform/windows/bmp.rs` 用 image crate（关默认 features，只开 bmp+png）做 BMP↔PNG。32bpp BI_RGB 的第 4 字节按 Windows 语义为保留位，roundtrip 后 alpha 一律不透明（RGB 无损）——对截图场景无损，含透明通道的图片回贴到剪贴板会失去透明，ticket 07 若有诉求再上 BITMAPV4 + alpha mask。
- 自家写入通道的关键发现：一次写入内部是 `EmptyClipboard` + `SetClipboardData`，会触发**两个** WM_CLIPBOARDUPDATE 事件，consume-once 的内容指纹标记挡不住第二个。最终方案用剪贴板序号（GetClipboardSequenceNumber，每次变化递增且永不复用）：`write_own` 完成后登记当前序号，变化事件按当前序号过滤——一次写入的全部事件序号相同全部命中，标记无需消费；用户稍后复制相同内容时序号已前进，正常入库。写入调用到序号登记之间的竞态窗口由 pending 标志兜底（TTL 5s 防写入方异常终止后永久误伤）。
- 另一个坑：clipboard-win 的 `set_bitmap` 为向后兼容**不清空剪贴板**（6.x 才改），直接用会与旧内容（如 CF_UNICODETEXT）合并；写入图片须自行 `with_clipboard` → `empty()` → `raw::set_bitmap`（内部 CreateDIBitmap → SetClipboardData(CF_BITMAP)）。
- 验证：cargo test 29 通过（repository 去重/容量/搜索/删除/持久化、engine 捕获/自家写入/失败路径、migration 幂等、BMP↔PNG roundtrip）；`cargo test -- --ignored --test-threads=1` 4 个真实剪贴板 smoke 通过（文本/图片读写 roundtrip、监听器收到同进程写入通知、端到端 捕获+自家写入+重复刷新）；clippy/fmt 干净，构建通过；前端 vitest 46 不受影响。
- 实机验收（Windows）：启动 youmi.exe → PowerShell `Set-Clipboard` 写文本 → SQLite（`%APPDATA%\com.youmi.app\youmi.db`）出现对应行；再次复制同文本 → 仍一行（同 id），recency 置顶、created_at 不变、updated_at 刷新；复制新文本排序正确。
- 边界决策：剪贴板同时含文本和图片时文本优先（读取顺序 CF_UNICODETEXT → CF_DIB）；空/纯空白文本不入库；关键词搜索仅匹配文本条目（SQLite LIKE 的 ASCII 大小写不敏感）；容量接入设置页（story 41 的剪贴板容量项）留给设置页后续 ticket 调 `clipboard_set_capacity` + plugin-store 持久化。

# 03 — 命令面板应用图标：按需提取与磁盘缓存

**What to build:** 命令面板应用条目显示应用自身图标（`.lnk` 解析到目标）：图标按需提取、落磁盘缓存，加载期间先显示回退图标异步替换，提取失败回退现有通用图标。第二次展示同一应用不再重新提取。

**Blocked by:** None — can start immediately

**Status:** done（2026-10-20 实机验收通过）

- [x] 搜索命中的开始菜单应用显示其真实图标（smoke：真实 .lnk 提取产出合法 PNG，样本全通过）
- [x] 图标异步加载：先回退图标后替换，列表布局不跳动（占位同尺寸）
- [x] 磁盘缓存生效：同一条目第二次展示不再重新提取（键 = 路径 + mtime，Rust 单测覆盖键稳定性与失效）
- [x] 缓存失效：lnk 变更（mtime）后重新提取（键含 mtime，天然失效）
- [x] 提取失败回退现有 lucide 通用图标，不阻塞条目交互（失败也会话内缓存，不反复提取）

## Comments

### 2026-10-19 — 实现

- **提取**（`platform/windows/icons.rs`，smoke 覆盖真实 .lnk）：SHGetFileInfoW（SHGFI_ICON | LARGEICON）解析 .lnk 取目标图标句柄 → GetIconInfo → 颜色位图 GetDIBits（32bpp top-down 负高度）→ BGRA 换 RGBA → image 编码 PNG。两个关键坑：① .lnk 解析依赖 COM（launch_lnk 同款 STA 初始化配平）；② windows crate 下 GetDIBits **空缓冲查询恒返回 0**，改 GetObjectW 取尺寸 + 单次 GetDIBits 读像素。句柄全路径释放（DestroyIcon + DeleteObject×2 + ReleaseDC）。
- **缓存与命令**（`apps.rs`）：`apps_icon(path) -> Option<Vec<u8>>`，放阻塞线程池执行；磁盘缓存 `app_data_dir/icons/<sha256(路径\nmtime秒)>.png`，命中直接回、未命中提取后落盘（读写失败只丢加速）；提取失败返回 None（前端回退，非错误）。`icon_cache_key` 单测 4 断言（稳定/mtime 失效/路径敏感/64 位 hex）。
- **前端**：`AppIcon` 组件——会话内 Map 缓存（上限 256，FIFO 挤出），blob URL 展示（卸载 revoke），加载中/失败统一回退 lucide AppWindow，两形态同 size-4 占位不跳列表。命令面板应用行替换为该组件。
- **依赖**：windows crate 增 `Win32_Graphics_Gdi`、`Win32_Storage_FileSystem` 两个 feature。

# 10 — 启动应用

**What to build:** 命令面板的启动器能力：扫描开始菜单快捷方式（.lnk），模糊搜索已安装应用，回车启动。应用列表在后台刷新，不拖慢面板首开。

**Blocked by:** 09 — 命令面板 v1

**Status:** done（2026-10-12）

- [x] 输入应用名片段即出候选（含中文名应用），回车启动
- [x] 应用列表后台构建/刷新，首次呼出无明显延迟
- [x] 无匹配结果时呈现合理空态

## Comments

- Rust：`src-tauri/src/apps.rs`——后台线程扫描用户级（`APPDATA`）与系统级（`ProgramData`）开始菜单 Programs 目录的 .lnk，名称取文件名去扩展名（即面向用户的显示名，含中文）；同名去重用户级覆盖系统级、同 rank 取路径较小者（结果与文件系统枚举顺序无关）。`AppsState` 常驻缓存：setup 时后台首扫不阻塞启动，`apps_query` 零 IO 读快照（实测 4.6ms / 128 条），缓存超龄（5 分钟 TTL）在呼起路径触发后台重扫——新装应用下次呼起可见；重扫以 `compare_exchange` 原子认领 + Drop guard 释放标志（扫描线程 panic 也不会让刷新永久卡死）。`apps_launch`：存在性 + .lnk 扩展名校验后经 `platform::windows::apps::launch_lnk` 启动（ShellExecuteW 以 shell 语义解析快捷方式，先 STA 初始化 COM 并配平，返回值 >32 判成功），放阻塞线程池不占主线程；失败以字符串返回前端。
- 前端：`src/lib/apps.ts`——`searchApps` 纯函数（`fuzzyScore` 按名称匹配，同分 `localeCompare` 稳定排序，截前 8 条防止应用淹没工具命中；空查询返回空数组，面板空查询只列 Youmi 工具），vitest 覆盖 8 例；`routes/overlay/palette.tsx`——条目顺序 算式 → 工具 → 应用，mount 与 window focus（重新呼起）时拉快照，应用行回车启动成功即收起、失败保留浮层（console.error 供重试）；空态区分「正在索引开始菜单应用」（首扫未完成时每秒轮询自愈，拉取失败同样进入轮询重试）与「没有匹配的工具或应用」。
- 实机验证（Windows，WebView2 CDP 驱动真实 webview，临时 CDP browser args 已全部还原）：真实开始菜单扫出 128 条（全 .lnk）；"note" 命中 Notepad++ 居首（前缀命中强于中间命中）；"微信" 命中 微信/微信开发者工具/卸载微信/卸载微信开发者工具，排序合理；"zzzzqq" 空态文案正确；"clean" → 回车启动 Disk Cleanup：cleanmgr 进程出现且面板自动收起；错误路径实测——不存在的路径与存在的非 .lnk（cmd.exe）分别报「快捷方式不存在」「不是开始菜单快捷方式」；TTL 刷新端到端：植入新 .lnk、超 5 分钟后下次查询 128→129 可见（验证条目已清理）；启动冒烟（review 修复后的最终代码）正常驻留。截图存档 gui-test-screenshots/palette-10-*.png。
- 验证环境限制：呼起靠 Win32 ShowWindow（提权远程会话 SendInput 无法触达全局热键，同 09 结论）；本会话 SetForegroundWindow 受前台锁限制拿不到 webview 键盘焦点，focus 复位路径（清空查询 + 列表复位）以合成 focus 事件实测通过，「重呼起时 focus 事件必达」由 09 实测背书且本票未改该机制，仅往同一 handler 增加了刷新调用。
- 已知边界：① 仅索引传统 .lnk——UWP/Store 应用在开始菜单无 .lnk，不可搜索启动（spec 数据源即 .lnk 扫描，接受）；② 非 UTF-8 可表示的文件名经 `to_string_lossy` 容错；③ 启动失败仅 console.error 无 UI 反馈（与算式复制失败的处理一致）；④ 结果截断 8 条与空查询不列应用为票外产品决策，已在代码注释说明理由。
- 验证基线：cargo test 55 通过（5 个真实剪贴板 smoke 仍 ignore 手动）、vitest 78 通过（新增 8）、tsc + oxlint + oxfmt、cargo fmt/clippy 全干净。code review（Standards/Spec 双轴）findings 已全部修复：COM 配平注释纠正、`is_lnk` 判定去重复、扫描线程 panic 安全 + 原子认领、去重同 rank 确定化（补测试）、面板 loading 轮询自愈、首拉失败的 loading 语义、`itemKey` 抽取、测试噪声清理。

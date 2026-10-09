# 04 — 托盘常驻 + 单实例 + 设置页雏形

**What to build:** Youmi 成为常驻应用：主窗口关闭按钮 = 隐藏到托盘继续运行；托盘菜单（打开主窗口 / 退出）；单实例——二次启动唤起已有实例；设置工作台页落地（tauri-plugin-store 持久化），首个设置项 = 开机自启动开关（默认关，tauri-plugin-autostart）。

**Blocked by:** 01 — App 骨架 + JSON 工具

**Status:** done（2026-10-09）

- [x] 点主窗口关闭按钮：进程存活、托盘图标在；托盘"打开"恢复窗口、"退出"结束进程
- [x] 二次启动 exe 唤起已有实例的主窗口，不产生第二个常驻进程（single-instance 注册为第一个插件）
- [x] 设置页可切换开机自启动并持久化，重启应用后状态保留
- [x] 设置页结构可扩展（后续 ticket 逐项加入改键、DeepSeek、容量、锁定超时）

## Comments

- Rust 侧：`single-instance` 注册为第一个插件（回调唤起主窗口），随后注册 `store`、`autostart`；`on_window_event` 拦截 `CloseRequested` 改为隐藏；`setup` 构建托盘（菜单：打开主窗口 / 退出；左键单击也恢复窗口）。托盘需要 `tauri` 的 `tray-icon` feature。
- 前端：`src/lib/settings.ts` 封装 plugin-store（`settings.json`，按命名键扩展）；`src/components/ui/switch.tsx`（radix）；设置页以 `SettingsSection` / `SettingRow` 组合出可扩展结构，首项 = 开机自启动开关。开关显示态以 autostart 插件（OS 注册表）为准，store 记录用户最近一次选择意图并在不一致时校正。
- capabilities 增加 `store:default`、`autostart:default`；`tauri.conf.json` 的 before 命令 npm→pnpm（跟随仓库迁移）。
- 验证（Windows 实机）：`CloseMainWindow()` 后进程存活、主窗口 visible=False；二次启动 exe → 第二进程退出且已有实例窗口恢复 visible=True；经 WebView2 CDP 驱动真实 UI：切换开关 → 注册表 Run 键出现/移除 Youmi 条目（auto-launch 0.6.0 动态模式：管理员写 HKLM、普通用户写 HKCU）+ `settings.json` 同步 true/false；杀进程重启后开关保持 checked；全程无控制台报错。cargo test / vitest 46 通过 / oxlint / cargo fmt 干净。
- 说明：auto-launch 的启用位置由 upstream 决定（管理员环境写 HKLM，发布版普通用户写 HKCU，均为每次登录自启）；托盘菜单点击与左键恢复属 shell 交互，程序化验证不覆盖，留手动确认。
- 遗留：曾尝试 tauri mock-runtime 的注册表 roundtrip 测试，本机 `STATUS_ENTRYPOINT_NOT_FOUND`（环境层 DLL 问题）未解，已删除该测试文件，以上 CDP 实测为更确凿证据。

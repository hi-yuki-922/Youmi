# 13 — MVP 验收 + 打包冒烟

**What to build:** 收尾 ticket：对照 spec（`.scratch/youmi-mvp/spec.md`）的 user stories 逐条手动验收；`tauri build` 产物做安装/运行冒烟（无签名、无 updater）；遗留问题记录回 tracker。

**Blocked by:** 02 — 编码转换；03 — 文本 diff；07 — 剪贴板回帖；08 — 密码本；10 — 启动应用；12 — 快捷键改键 + 冲突检测

**Status:** done（2026-10-15 用户实机验收通过）

- [x] spec 全部 user stories 验收：41/41 有实现证据、0 项未达——本会话完成代码级映射 + 安装版自动化冒烟；依赖真实硬件/凭据/实体按键的子项按 ticket 04/07/11/12 的既有实机验收记录转引，未重复验证（逐项注明原因，见 Comments 待人工清单）；验收中发现的唯一 spec 偏差——划词翻译默认模型名仍为已退役的 `deepseek-chat`——已当场修复为 `deepseek-flash`
- [x] `tauri build` 产物在本机可安装运行，核心链路（四类工作台工具 + 三个全局工具）冒烟通过（NSIS 静默安装到 `%LOCALAPPDATA%\Youmi` 实测；主窗口 30/30 断言全过 + 三浮层链路实测；MSI 产物同轮产出，未单独走安装流程）
- [x] oxfmt 若因 beta 问题阻碍日常格式化，执行"回退 prettier、保留 oxlint"并记录决策（实测不阻碍：保留 oxfmt + oxlint，无需回退；决策记录见 Comments）

## Comments

### 2026-10-15 — 验收方法与自动化结论

- **自动化全绿**：tsc + vite build 通过；vitest 88/88；cargo test 145 通过（8 个真实剪贴板/前台 smoke 为 `#[ignore]` 手动项）；clippy、oxlint 干净。
- **打包**：`tauri build` 产出 MSI `Youmi_0.1.0_x64_en-US.msi`（6.82 MiB）与 NSIS `Youmi_0.1.0_x64-setup.exe`（5.48 MiB）。NSIS `/S` 静默安装、覆盖升级安装均实测通过，安装位 `%LOCALAPPDATA%\Youmi`。
- **冒烟方法**：验收需驱动 GUI，而 WebView2 的 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` 环境变量被 wry 显式传入的 browser args 覆盖、不生效；临时给主窗口（tauri.conf.json）与全部浮层（overlay.rs builder）以**完全一致**的 additionalBrowserArgs 追加 CDP 调试端口（ticket 06 记录的一致性约束），以插桩构建完成安装版实测后**全部还原**，最终干净产物已重新构建并装回本机。插桩仅追加调试端口参数，不影响应用行为。
- **主窗口冒烟（CDP 驱动安装版，30/30 断言通过，截图 `gui-test-screenshots/t13-01..06-*.png`）**：JSON 格式化（2 空格/Tab 缩进）、压缩、校验错误定位（"第 1 行第 6 列"）、转义/去转义 roundtrip；SVG↔data URI、文本↔Base64（UTF-8）、query↔键值表全部双向即时转换；diff 即时渲染（截图证实并排视图、行级红删绿增、行内字符级高亮、"No newline at end of file" 提示）与统一视图切换；密码本锁定界面 + 错误主密码拒绝（"主密码错误"）——密码本完整链路（主密码设置/CRUD/30 秒清除/自动锁定）已由 ticket 08 于 2026-10-14 实机验收通过，本次未触碰用户已初始化的密码本数据；设置页五 section 齐备、模型现值/占位 deepseek-flash、容量回显 1000、自启动开关与快捷键三行呈现。
- **剪贴板浮层（Alt+V 注入呼出实测）**：呼出后 visible+聚焦；44 条真实历史渲染；真实用户路径自动捕获（浮层隐藏时外部写剪贴板 → Alt+V 呼出 → 新条目在顶，refetch-on-focus 链路成立）；关键词搜索命中/无结果空态；文本/图片分类 tab；删除单条（仅删除验收自己写入的 `youmi-t13-smoke-*` 条目，用户历史未动，未执行清空）。
- **命令面板**：`alt+Space`（用户改键后的现绑定）注入在本会话始终无法触发全局快捷键——与 ticket 09/12 记录的提权远程会话注入限制一致（同会话 Alt+V/Alt+T 却可触发，注入通道不可信）；改以 CDP 驱动面板页面完成功能冒烟：算式 `1+2*3` 即时 "= 7" 排首位、回车后系统剪贴板实读为 "7" 且面板收起；模糊搜索 "js" 唯一命中 JSON 工具且回车后主窗口切到 /json；应用搜索命中 Notepad++（"记事本"无命中系本机开始菜单无对应 .lnk，非功能缺陷）；Esc 隐藏。`shortcut_list` 实测三绑定全部 `registered:true`。OS 级实键呼出待用户日常使用确认（见待人工清单）。
- **划词翻译（Alt+T 注入呼出实测）**：外部窗口（记事本）前台时呼出，约 725ms 浮层可见（取词 600ms 超时后），显示"未检测到选中文本"提示与三个方向 tab；自家窗口前台时静默跳过属设计行为（story 32 目标是外部应用选中文本）。真实流式翻译已由 ticket 11 于 2026-10-13 实机验收通过。
- **固定浮层（story 34）机制证实**：验收中一度疑似"固定后失焦仍隐藏"，插桩日志排查证明守卫正确——`pinned=true` 期间的失焦事件被拦截、窗口保持可见（stderr 日志 `blur-hide check label=translate-overlay pinned=true` 后无 hide 调用）；后续的隐藏由一次合法的 `overlay_set_pinned(false)`（图钉取消固定，实为用户同时在机器上操作所致）之后的失焦产生，属未固定态正常行为。排查中还厘清：取词快捷键在自家窗口前台时静默跳过（不呼出浮层），自动化测试须以外部窗口为前台。
- **系统行为**：浮层 show/hide toggle、失焦即隐（面板/剪贴板浮层 blur 后隐藏）实测；关窗隐藏到托盘、单实例唤起、托盘菜单由代码与 ticket 04 实机验收覆盖，本次未重复破坏用户会话去验证。

### 2026-10-15 — 验收中发现并修复的 spec 偏差

- **story 35 默认模型名**：`src/lib/translate.ts` 的 `DEEPSEEK_DEFAULT_MODEL` 仍为已退役的 `"deepseek-chat"`，违背 spec Further Notes"默认值在实现时取当时官方文档"。查 DeepSeek 官方文档确认现行模型为 `deepseek-flash`（deepseek-v4-pro 自 2026-09-14 起请求被路由至 V4.1-Flash 并逐步淘汰），已改默认值为 `"deepseek-flash"`（设置页描述与占位符引用同一常量，自动跟随）。用户本机 settings.json 中实际配置值已是 `deepseek-flash`，与修复一致。tsc/vitest/oxlint 复跑全绿。

### 2026-10-15 — oxfmt 决策记录：不回退 prettier

- oxfmt 0.72.0 全仓格式化实测正常（69 文件 448ms、exit 0、幂等），oxlint 0 warning，均不阻碍日常格式化 → **保留 oxfmt + oxlint，不回退 prettier**。
- 已知小怪癖（记录在案，非阻塞）：手动全仓 `pnpm format` 会把 `src-tauri/Cargo.toml` 的 TOML 缩进重排为 2 空格（oxfmt 对 .toml 也生效）；lint-staged 提交路径的 glob（`ts/tsx/json/css/html/md`）不包含 .toml，日常提交不受影响，仅手动全仓格式化后 Cargo.toml 会出现在 diff 中，按需还原即可。

### 遗留与待人工复验清单

自动化无法覆盖或依赖真实硬件/凭据的项，逐条注明原因与既有验证来源：

1. **OS 级实键呼出三浮层**：本会话按键注入对全局快捷键不可信（Alt+V/Alt+T 触发成功、alt+Space 始终不触发，同机同会话自相矛盾，ticket 09/12 有同类记录）；三绑定注册状态实测 `registered:true`，且 ticket 12（2026-10-15）用户已实体验证改键与录制。→ 日常使用中确认 alt+Space 呼出面板即可。
2. **真实 DeepSeek 流式翻译**（需有效 API key 发起真实请求）：ticket 11（2026-10-13）实机复验通过（多应用划词 + 流式译文 + 剪贴板恢复 + 不入史）。
3. **回帖完整链路**（向外部窗口模拟 Ctrl+V）：ticket 07 实机验证通过（PS WinForms 目标 + 全链路断言）。
4. **托盘菜单/托盘单击**：ticket 04 实机验收通过；本会话未重复验证以免打断用户正在使用的实例。
5. **MSI 安装流程**：产物已产出但未走安装（NSIS 已完整验证安装/运行/覆盖升级；两者打包同一份产物，安装器差异不影响应用本身）。
6. **观察项（非 story 未达）**：`palette.tsx` 的 `aria-activedescendant` 引用 `palette-item-N` id，但列表 `li` 未携带对应 id（ARIA 组合框模式的部分不一致，键盘导航实际走的是自管理 selected 状态，功能无碍）；后续如做 a11y 打磨可顺手补上。

### 实机验收通过置 done（2026-10-15）

用户确认验收通过，ticket 置 done，MVP 收尾完成。待人工清单各项按上述来源转引或随日常使用覆盖（含 alt+Space 实键呼出面板）；观察项 6（palette ARIA id 不一致）不阻塞，留作后续 a11y 打磨素材。

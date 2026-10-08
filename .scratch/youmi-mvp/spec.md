# Youmi MVP

Status: ready-for-agent

## Problem Statement

日常工作与开发中，常用的小工具散落各处：编码转换、JSON 处理、文本对比依赖在线工具（有隐私顾虑、断网不可用、打开慢）；Windows 自带的 Win+V 剪贴板历史功能弱（不可搜索、不可分类、容量小）；PowerToys 命令面板偏重；DeepL 翻译偶尔响应太慢。用户希望有一个本地、快速、常驻、统一的自用工具集合。

## Solution

Youmi——常驻系统托盘的 Windows 桌面应用（保留未来迁移 macOS 的可能）。**主窗口**以侧边栏导航承载四个**工作台工具**：编码转换、JSON 工具、文本 diff、密码本。三个**全局工具**通过全局快捷键以**浮层窗口**随时呼出：剪贴板管理（`Win+V`）、命令面板（`Win+Alt+Space`）、划词翻译（`Alt+T`）。

## User Stories

### 应用骨架与系统行为

1. As a user, I want 主窗口左侧 sidebar 导航所有工作台工具与设置, so that 一眼找到并切换工具。
2. As a user, I want 点击主窗口关闭按钮时应用隐藏到托盘继续运行（托盘菜单提供真正退出）, so that 全局工具始终可用。
3. As a user, I want 应用单实例——再次启动时唤起已有实例, so that 不会出现多个常驻进程抢快捷键。
4. As a user, I want 在设置中开关开机自启动（默认关）, so that 按需控制常驻。
5. As a user, I want 所有全局快捷键可在设置中修改并有冲突提示, so that 与其他软件协调。
6. As a user, I want 界面中文、主题跟随系统, so that 视觉与系统一致。
7. As a user, I want 托盘菜单可以打开主窗口和退出应用, so that 不进主窗口也能控制。

### 编码转换

8. As a user, I want SVG 源码与 URL 编码互转, so that 直接用于 CSS background-image 的 data URI。
9. As a user, I want 文本与 Base64 互转, so that 处理编码后的配置或 token。
10. As a user, I want URL query 字符串与键值表互转, so that 调试接口参数。
11. As a user, I want 三个子工具都是输入即时转换、结果一键复制, so that 不点任何按钮。

### JSON 工具

12. As a user, I want JSON 格式化且缩进可调（2/4/tab）, so that 阅读压缩过的报文。
13. As a user, I want JSON 压缩, so that 生成最小体积。
14. As a user, I want JSON 校验并定位错误行, so that 快速修复语法错误。
15. As a user, I want JSON 字符串转义/去转义互转, so that 在代码与原文之间搬运。

### 文本 diff

16. As a user, I want 两段文本并排对比，带行级增删高亮、行内字符级高亮和语法高亮, so that 直观看出差异。
17. As a user, I want 并排视图与统一视图可切换, so that 适配不同场景。

### 剪贴板管理

18. As a user, I want 应用自动捕获我复制的文本和图片, so that 历史无需手动维护。
19. As a user, I want 按 Win+V 呼出剪贴板浮层浏览历史, so that 替代系统剪贴板历史。
20. As a user, I want 在浮层中选中条目后回帖到上一个聚焦的窗口, so that 完成粘贴。
21. As a user, I want 在浮层中按关键词搜索历史, so that 快速找到早前复制的内容。
22. As a user, I want 按文本/图片分类查看历史, so that 图片多时不受文本干扰。
23. As a user, I want 重复复制的内容刷新原条目到顶部而不是新增, so that 历史不膨胀。
24. As a user, I want 历史持久化最近 1000 条（可配置），重启不丢, so that 长期可用。
25. As a user, I want 删除单条或清空历史, so that 管理隐私内容。
26. As a user, I want 从密码本复制的密码不进入历史、写入剪贴板 30 秒后自动清除, so that 敏感内容不残留。

### 命令面板

27. As a user, I want 按 Win+Alt+Space 呼出命令面板浮层, so that 替代 PowerToys Run。
28. As a user, I want 模糊搜索切换到任意 Youmi 工具, so that 不碰鼠标直达工具。
29. As a user, I want 搜索并启动已安装应用（扫描开始菜单快捷方式）, so that 当应用启动器用。
30. As a user, I want 输入算式即时得到四则运算结果, so that 顺手计算。
31. As a user, I want 面板失焦即隐、再次呼起瞬时, so that 用完即走。

### 划词翻译

32. As a user, I want 选中文字后按 Alt+T 在鼠标附近弹出小浮层并流式输出译文, so that 不切窗口完成翻译。
33. As a user, I want 自动检测源语言、默认译为中文，浮层内可切换方向, so that 双向翻译都顺手。
34. As a user, I want 浮层可以固定（固定后不因失焦关闭）, so that 对照长文慢慢看。
35. As a user, I want 在设置中配置 DeepSeek API key 与模型名, so that 换模型不动代码。

### 密码本

36. As a user, I want 首次使用时设置主密码, so that 密文由我独有密钥保护。
37. As a user, I want 用主密码解锁密码本，超时（默认 5 分钟，可配置）自动锁定, so that 离开电脑不裸奔。
38. As a user, I want 增删改查凭据条目（标题/用户名/密码/URL/备注）, so that 管理我的凭据。
39. As a user, I want 一键复制用户名或密码（触发 story 26 的保护）, so that 登录表单快速填写。
40. As a user, I want 按标题/用户名搜索条目, so that 条目多了也能找到。

### 设置

41. As a user, I want 一个设置工作台页集中管理：自启动、快捷键、DeepSeek key/模型、剪贴板容量、密码本锁定超时, so that 不散落各处。

## Implementation Decisions

### 架构

- Tauri 2 单 app、两类窗口：主窗口（工作台工具路由页）+ 浮层窗口（全局工具）。系统托盘常驻，进程不随主窗口关闭退出。
- **平台 seam**（ADR-0001）：clipboard-listener、global-hotkey、selection-capture、overlay-position、credential-store 五个 seam，Rust 平台模块隔离，仅实现 Windows，不写 macOS。
- 前端只经 tauri command 访问 OS 能力与存储。

### 前端

- Vite 8 + React 19 + TypeScript；tanstack-router（扁平路由）+ tanstack-query；shadcn-ui + lucide icons。
- 主窗口 sidebar 导航；UI 中文；主题跟随系统。
- diff 用 `@pierre/diffs`（React 组件：并排/统一、行级+行内+语法高亮）。
- AI 用 Vercel AI SDK + openai-compatible provider 指向 DeepSeek（`https://api.deepseek.com`）；provider 设置注入自定义 fetch = `@tauri-apps/plugin-http` 的 fetch（请求走 Rust 网络栈，规避 webview CORS，CSP 不放开外联）；换厂商 = 换 provider 包。

### Rust

- 存储：SQLite（rusqlite + migration），Repository 深模块藏在 tauri command 后；剪贴板历史（图片以 PNG blob 存储）与密码本密文共用该库；应用设置用 tauri-plugin-store。
- 剪贴板监听：`clipboard-win` 的 Monitor 模块（WM_CLIPBOARDUPDATE），不用官方 clipboard 插件（无监听能力，且有与 Win+V 面板间歇冲突的未决 issue）。
- 划词取词：`selection` crate 路线（模拟 Ctrl+C + 读剪贴板）；必须保存/恢复用户原剪贴板内容，且自家模拟复制产生的条目不得进入剪贴板历史。
- 密码本：主密码 + Argon2id 派生密钥 + AES-256-GCM（ADR-0002）；解锁会话明文仅驻内存；复制密码：写入剪贴板但不通知监听器入史，30 秒后自动清除。
- 插件：global-shortcut（默认 Win+V / Win+Alt+Space / Alt+T，全部可改键+冲突提示）、single-instance（注册为第一个插件，二次启动唤起已有实例）、autostart（默认关）、http（作为 AI SDK 的 fetch 通道）。
- 凭据存储 seam 用 `keyring` crate（Windows Credential Manager）。
- 浮层窗口：Tauri 原生配置——`decorations:false`、`alwaysOnTop`、失焦（`onFocusChanged(false)`）即 `hide()`（hide 而非销毁，二次唤起瞬时）、定位到鼠标所在显示器居中。

### 工具链

- husky + lint-staged；oxc 代替 prettier + eslint：oxlint（稳定）+ oxfmt（beta，Prettier 兼容、自带 import 排序）。
- 测试：vitest（前端）+ cargo test（Rust）。
- 构建：`tauri build` 本地分发，无代码签名、无自动更新。

### 阶段划分

- 阶段一：app 骨架（主窗口/路由/sidebar/托盘雏形）+ JSON 工具 → 编码转换 → diff。
- 阶段二：全局能力——托盘完善、快捷键、剪贴板管理。
- 阶段三：命令面板 → 密码本 → 划词翻译。

## Testing Decisions

Greenfield，无既有 seams，新建两层（数量已最少化）：

1. **工具核心纯函数层**（TS）：每个工作台工具的转换/解析核心与 React 组件分离，vitest 直测纯函数——JSON 格式化/压缩/校验定位/转义、三件编码转换、URL query 解析、计算器表达式求值。
2. **Rust 模块公共接口**（tauri command 之下的模块边界）：cargo test——剪贴板历史 Repository（写入/去重刷新/容量淘汰/持久化）、密码本 crypto（加解密 roundtrip、错误主密码拒绝、锁定行为）、取词的剪贴板保存/恢复逻辑。

- 好测试只断言外部行为（输入→输出、可见状态变化），不断言实现细节。
- 平台 seam 的 Windows 实现不 mock，靠少量集成 smoke + 手动验收；UI 组件不写单测，手动验收（自用工具）。

## Out of Scope

- 剪贴板文件格式（记录复制的文件路径列表）
- 剪贴板敏感忽略清单（按来源应用忽略）
- 密码本 TOTP、自定义字段
- JSON tree 视图、JSON→TS 类型转换、大文件虚拟滚动
- diff 字符级高亮开关定制（库自带行为直接用）
- 划词翻译手动输入翻译
- 命令面板：URL/搜索兜底、文件搜索、插件系统
- 命令面板 shell 命令执行（待定，后续立调研 ticket）
- macOS/Linux 实现（只留 seam）
- 自动更新、代码签名、多语言 UI

## Further Notes

- **oxfmt 处于 beta**（2026-02 起）：若 beta 问题阻碍日常格式化，回退 prettier，oxlint 保留。
- **Win+V 前提**：用户需在 Windows 设置中关闭系统剪贴板历史，Win+V 才可被注册；设置页需引导提示，改键可规避。
- **DeepSeek 模型名已换代**（deepseek-chat/reasoner 已退役，现行 deepseek-flash / deepseek-v4-pro）：模型名必须可配置，默认值在实现时取当时官方文档。
- 命令面板"启动应用"数据源 = 开始菜单 .lnk 扫描。
- 本 spec 的 ADRs：ADR-0001（平台 seam）、ADR-0002（密码本加密）。

# 11 — 划词翻译

**What to build:** 第三个全局工具：在任意应用选中文字后按 `Alt+T`——经 selection crate 模拟取词（保存/恢复用户原剪贴板内容，模拟复制经 05 的自家写入通道不入历史）→ 鼠标附近弹出小浮层，DeepSeek 译文流式输出（Vercel AI SDK + openai-compatible provider，自定义 fetch 走 tauri http 插件）；自动检测源语言、默认译为中文，浮层内可切换方向；浮层可固定（固定浮层不因失焦隐藏）。设置页新增 DeepSeek API key（keyring 存系统凭据库）与模型名两项。

**Blocked by:** 04 — 托盘常驻 + 单实例 + 设置页雏形；06 — 剪贴板浮层 v1

**Status:** done（2026-10-13）

- [x] 在浏览器/记事本等应用选中文字按 Alt+T：浮层在鼠标附近出现并流式输出译文（实机验收通过；取词被残留 Alt 污染的缺陷已修复，见 Comments 末两条）
- [x] 取词后用户原剪贴板内容被恢复，模拟复制不进入剪贴板历史（单测 + 真实剪贴板/凭据 smoke 覆盖）
- [x] 翻译方向可切换（自动→中文为默认；源文已是中文时自动方向改译英文；提示词逻辑单测覆盖，浮层内点击待人工验收）
- [x] 浮层可固定，固定后点击其他区域不关闭（定位/固定逻辑单测覆盖，浮层内点击待人工验收）
- [x] 设置页可配置 API key 与模型名，key 存系统凭据库、不落明文文件（真实凭据库 smoke + 假 key 经 app 同名条目读回验证）
- [x] key 未配置或请求失败时浮层呈现清晰错误提示（实现完成；GUI 展示待人工验收）

## Comments

- 与 spec 字面的偏差：spec 写"经 selection crate 模拟取词"，实现按 ADR-0001 平台 seam 自建（`SelectionAccess` trait + Win32 SendInput Ctrl+C）而未引入字面 crate——外部 crate 的剪贴板保存/恢复行为绕开自家写入通道的序号过滤，无法满足"模拟复制不入历史"这条硬约束；验收条件不要求具体 crate。
- "自动检测源语言"为 prompt 级委托（AUTO_RULE 让模型自检，源文已中文时改译英文），非程序级语言检测——检测可靠性依赖模型行为，属明示取舍。
- 平台 seam：按 ADR-0001 预先声明的清单补齐两个——`SelectionAccess`（划词取词：前台进程归属 + SendInput 模拟 Ctrl+C）与 `SecretStore`（凭据存储：get/set/delete），Windows 实现分别为 `platform/windows/selection.rs`（复用与回帖共用的 Win32 辅助：`foreground_hwnd`/`process_of`/`send_ctrl_chord` 提升到 `platform/windows/mod.rs`，pasteback 一并改为复用）与 `platform/windows/secrets.rs`（keyring v3 windows-native，服务名 `youmi`，条目名 `deepseek-api-key`）。send_copy/send_paste 同会向用户前台注入键击，按既例不设自动 smoke，靠手动验收。
- 取词编排：`translate/selection.rs`——前台是外部进程时：读原剪贴板 → 引擎开模拟复制过滤窗口 → SendInput Ctrl+C → 轮询剪贴板序号（30ms 首延 + 25ms 间隔，600ms 超时）→ 序号前进则以当前序号落账（代写引发的监听事件全部豁免入库，与自家写入同一套序号识别）→ 读出选中文本 → 同步恢复原剪贴板（带序号守卫：读词后用户又写入则放弃恢复）。自家窗口聚焦时静默跳过（不注入、不清浮层已显示的译文，与回帖"自家窗口不作目标"一致）；超时按无选中处理且撤销过滤窗口（迟到的代写与用户复制正常入库）；send_copy 失败返回错误；代写结果为空白/非文本视为无选中并照常恢复。原内容不可读（文件等未支持格式）时无法恢复，选中文本留在剪贴板（宁留勿毁）。
- 引擎：`ClipboardEngine` 新增 `begin/finish/abort_simulated_copy` 三方法，把 05 的序号过滤窗口开放给"目标应用代写"场景；`write_own` 本身不变。
- 浮层：`overlay.rs` 新增 `PinnedOverlays`（被固定浮层的 label 集合，setup 最先管理）与 `overlay_set_pinned` command（校验窗口存在），失焦即隐 handler 查固定状态后决定是否 hide；新增 `show_at_cursor`（划词翻译用）——光标右下偏移 12px 优先、右侧/下方放不下翻到光标左/上、再 clamp 进光标所在显示器（`near_cursor_position` 纯函数 6 例测试，含副屏负坐标）；`toggle` 与之共用 `find_monitor_containing`。翻译浮层窗口 `translate-overlay`（460×360），Alt+T 全局快捷键在 lib.rs 与 Alt+V / Win+Alt+Space 并列注册，注册失败降级日志不阻塞启动。
- 前端：AI SDK 为 `ai` v7 + `@ai-sdk/openai-compatible`，`createOpenAICompatible` 自定义 `fetch` 走 `@tauri-apps/plugin-http`（capabilities 放行 `https://api.deepseek.com/*`），`streamText` 的 textStream 不递出错误（v7 行为），错误经 `onError` 汇集、流结束后抛出由浮层统一呈现。方向三档（自动→中文 / 中→英 / 英→中）存组件状态，切换即中止当前流并按新方向重译；提示词构造在 `src/tools/translate/core.ts`（纯逻辑，vitest 6 例；自动方向约定源文已中文时改译英文，避免中→中空转）。浮层 UI：方向 SegmentedTabs + 固定按钮（Pin/PinOff）+ 原文三行摘要 + 流式译文区（流式光标 ▍）+ Esc 隐藏；事件 `translate-selection`（Rust `SelectionEventPayload`：text/message 互斥）驱动，取词必先于显示（Ctrl+C 要发给呼起时仍在前台的目标应用），编排放 `spawn_blocking`。
- 设置页：「划词翻译」区块——API key 行（password 输入 + 保存按钮；页面不回显明文，仅显示已配置状态；空白保存 = 清除凭据；经 `translate_get/set_api_key` command 存取，值只在发起翻译时短暂取回 webview 内存）与模型名行（失焦/回车提交，存设置文件，默认 `deepseek-chat`），外加 Alt+T 占用提示。凭据存取状态 `SecretState` 独立于剪贴板引擎管理：引擎初始化失败时设置页仍可配置。
- 测试：cargo 76 例 + 7 个 `#[ignore]` smoke（本次全部实跑通过）：取词编排 10 例（fake 注入：取词+恢复、不入史、自家/未知前台跳过、超时、注入失败、图片恢复、原内容不可读、空白响应、恢复序号守卫）、引擎模拟复制 3 例、光标旁定位 6 例、凭据保存语义 2 例、凭据库/取词 key 条目真实 smoke。前端 oxlint 干净、vitest 84 例（+6 翻译纯逻辑）、tsc/vite build 通过。
- 实机验证（Windows，dev 实跑）：① 真实凭据库 smoke 通过；并用 `verify/fake-key.ps1`（CredWrite，命名 `{user}.{service}` 与 keyring windows-native 源码一致）预置假 key 后，`translate::smoke_tests::deepseek_api_key_entry_is_readable` 经 app 同一条目名读回成功——设置页保存 → 取词命令读取的凭据链路成立；② 真实剪贴板 smoke（文本/图片读写回环、监听通知、捕获+自家写入通道 E2E）通过，期间发现 E2E smoke 与运行中的 dev app 争用剪贴板会偶发失败（环境争用，停掉 dev app 后 3/3 稳定），非本次回归；③ GUI 驱动验证（SendKeys Alt+T + 截屏）因当前控制台锁定（无前台窗口、截屏句柄无效）无法执行——驱动脚本已就绪（`verify-translate.ps1` + `translate-target.ps1` + `fake-key.ps1`，控制台解锁后可复跑：无选中提示、假 key 401 错误展示、剪贴板恢复、历史无选中条目四步）。
- 已知边界：① 真实 API key 的流式译文端到端（AI SDK → tauri http → DeepSeek SSE）与浮层 GUI 行为（光标旁定位、方向切换、固定失焦不隐）留人工验收：解锁后在设置页填入真实 key 跑 `verify-translate.ps1` 即可覆盖；② 取词靠模拟 Ctrl+C，不支持复制的目标（如部分终端只读输出、无选中语义的应用）600ms 超时按"未检测到选中文本"处理；③ 管理员权限窗口作为取词目标时 SendInput 受 UIPI 限制，同样走超时路径；④ 原剪贴板为文件列表等未支持格式时无法恢复（选中文本留在剪贴板）；⑤ 自家窗口聚焦时 Alt+T 为静默 no-op（含主窗口里选中的文字）；⑥ http 插件仅放行 `api.deepseek.com`——未来支持自定义 base URL 需同步扩 capabilities 作用域。
- review（Standards+Spec 双轴）处置：修复——smoke 注释去掉 tracker 脚本路径引用（注释通则）；事件载荷改 tagged enum（Rust `SelectionEventPayload` 与 TS 可辨识联合对应，互斥进类型）；新增 `translate_has_api_key` presence 查询，设置页不再取回 key 明文（与"值只在发起翻译时取回"一致）；状态/勾选回退为如实的 ready-for-human。保留——固定能力挂在通用浮层失焦 handler 上（CONTEXT"固定浮层"本就是浮层窗口层概念，非翻译专属）；Esc 隐藏浮层与其余浮层一致；自家前台守卫在 mod.rs/selection.rs 各一行、语义不同（静默跳过 vs 无选中），不抽共享；settle/timeout/poll_interval 三元组不再包一层；浮层定位尚未 seam 化属 ADR-0001 既有欠账，未在本次扩大处理。
- 实机验收缺陷修复（2026-10-13）：用户实机划词后按 Alt+T，Rider/VSCode/记事本/浏览器全部报"未检测到选中文本"。根因：全局快捷键经 RegisterHotKey 在按键按下瞬间投递，取词编排毫秒级即执行到 SendInput，此时用户往往仍按着 Alt——`send_ctrl_chord` 不清理残留修饰键，目标应用收到 Ctrl+Alt+C 而非 Ctrl+C，复制永不发生，600ms 超时按无选中处理。与两处现象吻合：全部应用一致失败（与具体应用无关）、报错浮层能弹出（自家前台守卫是静默 return 不弹浮层，浮层弹出说明编排走完、send_copy 已发出）；UIPI/超时时长/序号轮询假设均与现象不符（记事本未提权且响应为毫秒级、真实剪贴板 smoke 已过）。修复：chord 前按 GetAsyncKeyState 抬起残留的 Alt/Shift/Win，与 chord 合并为一次 SendInput 批（先抬后按）；Ctrl 是 chord 自身组成部分，残留无害不处理；抬起后不回按——回按会让目标应用看到多余的 Alt 按下-抬起（菜单激活类副作用）。回帖 `send_paste` 共用 `send_ctrl_chord` 一并受益。验证：新增 `#[ignore]` 真实键击 smoke `send_ctrl_chord_releases_lingering_alt`（注入 Alt↓ 模拟快捷键触发瞬间 → 调真实 chord → 断言 Alt 已释放，前置不成立时报环境错误不误报），修复前稳定 red、修复后 3/3 green；cargo 76 例 + fmt/clippy 干净。待人工复验：真实应用划词 Alt+T 全链路（含带真实 key 的流式译文）。
- 复验通过，置 done（2026-10-13）：残留修饰键修复后实机验证通过——Rider/VSCode/记事本/浏览器划词 Alt+T 均正常弹出浮层并流式翻译，剪贴板恢复与不入史行为无异常。

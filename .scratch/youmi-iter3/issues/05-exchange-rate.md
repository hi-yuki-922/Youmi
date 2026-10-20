# 05 — 汇率换算工具页

**What to build:** 主窗口新增"汇率"工具（工具注册表收录，侧边栏与命令面板可达）：一页换算表——顶部源币种下拉 + 金额输入，下方目标币种行列表展示换算结果；行可增删、行选择持久化，首次默认 USD/EUR/JPY 三行；数据源 open.er-api.com 免费无 key，币种下拉可搜索（代码+中文名）；打开页面时缓存超 6 小时静默刷新 + 手动刷新按钮 + 展示数据更新时间；离线用最后缓存并标注。

**Blocked by:** None — can start immediately

**Status:** done（2026-10-20 实机验收通过）

- [x] 侧边栏与命令面板可搜到"汇率"页（TOOLS 注册表收录，翻译与密码本之间）
- [x] 源币种下拉可搜索（代码+中文名），金额输入实时换算
- [x] 目标币种行可增删，首次默认 USD/EUR/JPY 三行，行选择持久化（源币种默认 CNY，与目标行对选即换方向）
- [x] open.er-api.com 免费源，零配置可用（API 实测响应形状吻合；capability 增开放该域名）
- [x] 打开页面缓存超 6h 静默刷新；手动刷新按钮；展示数据更新时间
- [x] 离线时用最后缓存并标注，不白屏不报错刷屏（FeedbackStrip 提示一次性）
- [x] vitest 覆盖换算核心（换算/可逆/缺汇率、新鲜度按 fetchedAt、格式化、币种清单）14 例

## Comments

### 2026-10-19 — 实现

- **核心**（`src/tools/exchange/core.ts`，纯函数）：`RateSnapshot { rates(USD 基准), updatedAt(数据源), fetchedAt(本地拉取) }`——新鲜度只看 fetchedAt（数据源每日更新，用它判过期会恒过期）；`convert` 经 USD 两段折算、缺汇率/非法金额报错；`formatAmount` 千分位最多 4 位小数；内置 ~130 币种中文名映射（多"第纳尔/迪拉姆/里亚尔"带国别限定），未收录回退代码。
- **传输与缓存**（`src/lib/exchange.ts`）：open.er-api.com 经 tauri http 插件拉取（绕 CORS，capability 增 `https://open.er-api.com/*` 作用域）；快照落 localStorage（键 `exchange.rate-snapshot`），读回逐项校验形状；`ensureRates` 新鲜直用 / 过期拉取 / 失败回退旧缓存给提示。
- **页面**（`src/routes/_main/exchange.tsx`）：源币种 CurrencyPicker + 金额输入 + 目标币种行列表（选择器行内改选、与源对选即换方向、X 移除、底部"添加币种…"）；行选择/金额/源币种 `usePersistedToolState` 持久化（`restoreStringArray` 为本次在 tool-state 新增的通用恢复器：过滤候选集 + 去重保序，空结果回退默认行）。
- **可搜索下拉**（`src/components/CurrencyPicker.tsx`）：按钮触发浮层（radix Select 无搜索能力），搜索框按代码/中文名过滤，↑↓/⏎/⎋ 键盘导航，点击外部关闭；空值呈"添加"形态供追加行使用。

# 11 — 密码本锁定超时 0=永不

**What to build:** 密码本锁定超时支持 0 = 永不自动锁定：前端钳制放宽为 0..1440 分钟并显示"永不"说明；Rust 端 0 特判——autolock 哨兵不排期、命令路径过期检查恒不过期。手动锁定按钮行为不变。

**Blocked by:** 无——可立即开始

**Status:** ready-for-human（2026-10-16 实现完成，cargo test 全绿；0=永不的实机行为待复验）

- [ ] 设置可输入 0 并显示"永不"语义
- [ ] 0 时长时间放置不自动锁定
- [ ] 手动锁定按钮正常锁定
- [ ] 1..1440 行为不回归
- [x] cargo test 覆盖 0 语义（autolock 不触发、会话不过期）

## Comments

### 2026-10-16 — 实现

- **Rust**（`src-tauri/src/vault/mod.rs`）：`expire_if_due` 对 `timeout_secs == 0` 恒不落锁（命令路径与状态查询共用此检查，自动锁定哨兵因此每秒轮询恒为空转）；`vault_set_lock_timeout` 命令 0 直通、非零仍钳制 10 秒～24 小时。哨兵线程不按值启停——锁定超时可在运行时改回非零，空转即语义。
- **测试**：原 `idle_session_auto_locks_after_timeout` 借超时 0 构造"已到期"（与新语义冲突），改为 1 秒超时 + `wait_until` 轮询（不再依赖时钟回拨，对短 uptime 环境安全）；新增 `zero_timeout_never_auto_locks`（0 时 expire_if_due 恒 false、会话不过期、条目照常可用）与 `manual_lock_works_with_zero_timeout`（手动落锁不受 0 影响）。
- **前端**（`src/lib/vault.ts` + 设置页）：钳制放宽为 0..1440 分钟，抽 `clampLockTimeoutMinutes` 纯函数（vitest 4 例）；**小数向上取整**——0.5 分钟归 1 分钟而非被取整为 0 = 永不（review 发现的语义跳变，ceil 后 0 仍保留为永不）；锁定超时行 min=0，committed 为 0 时后缀与说明显示"永不"语义，负数/非数字恢复原值。手动锁定按钮路径未动。

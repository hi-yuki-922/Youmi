import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { getSetting, setSetting } from "@/lib/settings";

/** 密码本状态（Rust `VaultStatus` camelCase 序列化）。 */
export interface VaultStatus {
  initialized: boolean;
  unlocked: boolean;
}

/** 凭据条目（解锁会话内含明文字段）。 */
export interface VaultEntry {
  id: number;
  title: string;
  username: string;
  password: string;
  url: string;
  notes: string;
  createdAt: number;
  updatedAt: number;
}

/** 条目表单载荷：新增与更新共用，字段均可为空串，标题由后端校验必填。 */
export interface VaultEntryInput {
  title: string;
  username: string;
  password: string;
  url: string;
  notes: string;
}

/** 复制目标字段（Rust `VaultCopyField` kebab-case 序列化）。 */
export type VaultCopyField = "username" | "password";

export const getVaultStatus = (): Promise<VaultStatus> => invoke<VaultStatus>("vault_status");

/** 首次使用设置主密码（至少 8 位），成功即进入解锁会话。 */
export const setupVault = (masterPassword: string): Promise<void> =>
  invoke("vault_setup", { masterPassword });

/** 主密码解锁；错误密码被拒绝（错误消息展示给用户）。 */
export const unlockVault = (masterPassword: string): Promise<void> =>
  invoke("vault_unlock", { masterPassword });

/** 立即落锁。 */
export const lockVault = (): Promise<void> => invoke("vault_lock");

/** 列出条目（updated_at 降序），keyword 非空时按标题/用户名过滤。 */
export const queryVaultEntries = (keyword?: string): Promise<VaultEntry[]> =>
  invoke<VaultEntry[]>("vault_query", { keyword: keyword || undefined });

export const createVaultEntry = (entry: VaultEntryInput): Promise<number> =>
  invoke<number>("vault_create", { entry });

export const updateVaultEntry = (id: number, entry: VaultEntryInput): Promise<boolean> =>
  invoke<boolean>("vault_update", { id, entry });

export const deleteVaultEntry = (id: number): Promise<boolean> =>
  invoke<boolean>("vault_delete", { id });

/** 复制用户名/密码：走自家写入通道（不进剪贴板历史），30 秒后自动清除。 */
export const copyVaultField = (id: number, field: VaultCopyField): Promise<void> =>
  invoke("vault_copy", { id, field });

/** 自动落锁事件：解锁会话超时后 Rust 发往主窗口，前端切回解锁界面。 */
const VAULT_LOCKED_EVENT = "vault-locked";

export const onVaultLocked = (handler: () => void): Promise<() => void> =>
  listen(VAULT_LOCKED_EVENT, handler);

export const VAULT_LOCK_TIMEOUT_DEFAULT_MINUTES = 5;
const VAULT_LOCK_TIMEOUT_KEY = "vault.lockTimeoutMinutes";
/** 设置页输入的分钟数范围；Rust 侧另有 10 秒～24 小时的兜底 clamp。 */
const VAULT_LOCK_TIMEOUT_MIN_MINUTES = 1;
const VAULT_LOCK_TIMEOUT_MAX_MINUTES = 24 * 60;

/** 锁定超时（分钟）：无操作该时长后自动落锁；未设置过时返回默认值。 */
export const getVaultLockTimeoutMinutes = (): Promise<number> =>
  getSetting(VAULT_LOCK_TIMEOUT_KEY, VAULT_LOCK_TIMEOUT_DEFAULT_MINUTES);

/** 把锁定超时（秒）应用到 Rust 会话；立即生效，不落设置文件。 */
const applyLockTimeoutSeconds = (seconds: number): Promise<void> =>
  invoke("vault_set_lock_timeout", { seconds });

/** 修改锁定超时并立即生效（应用到 Rust 会话），同时持久化。 */
export const setVaultLockTimeoutMinutes = async (minutes: number): Promise<void> => {
  const clamped = Math.min(
    Math.max(Math.floor(minutes), VAULT_LOCK_TIMEOUT_MIN_MINUTES),
    VAULT_LOCK_TIMEOUT_MAX_MINUTES,
  );
  await applyLockTimeoutSeconds(clamped * 60);
  await setSetting(VAULT_LOCK_TIMEOUT_KEY, clamped);
};

/** 启动时把持久化的锁定超时应用到会话（幂等，不回写设置）。 */
export const applyVaultLockTimeout = async (): Promise<void> => {
  const minutes = await getVaultLockTimeoutMinutes();
  await applyLockTimeoutSeconds(minutes * 60);
};

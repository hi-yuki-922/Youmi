import { Store } from "@tauri-apps/plugin-store";

const SETTINGS_FILE = "settings.json";

let storePromise: Promise<Store> | undefined;

function settingsStore(): Promise<Store> {
  storePromise ??= Store.load(SETTINGS_FILE);
  return storePromise;
}

async function getSetting<T>(key: string, fallback: T): Promise<T> {
  return (await (await settingsStore()).get<T>(key)) ?? fallback;
}

async function setSetting<T>(key: string, value: T): Promise<void> {
  await (await settingsStore()).set(key, value);
}

const AUTOSTART_ENABLED_KEY = "autostart.enabled";

/**
 * autostart 的实际状态以 autostart 插件（OS 注册表）为准；
 * store 只记录用户最近一次的选择意图，供后续设置同步/排查使用。
 */
export function getAutostartPreference(): Promise<boolean> {
  return getSetting(AUTOSTART_ENABLED_KEY, false);
}

export function setAutostartPreference(enabled: boolean): Promise<void> {
  return setSetting(AUTOSTART_ENABLED_KEY, enabled);
}

import { Store } from "@tauri-apps/plugin-store";

const SETTINGS_FILE = "settings.json";

let storePromise: Promise<Store> | undefined;

const settingsStore = (): Promise<Store> => {
  storePromise ??= Store.load(SETTINGS_FILE);
  return storePromise;
};

const getSetting = async <T>(key: string, fallback: T): Promise<T> =>
  (await (await settingsStore()).get<T>(key)) ?? fallback;

const setSetting = async <T>(key: string, value: T): Promise<void> => {
  // 显式 save：auto-save 在多 webview reload/资源关闭时序下不保证落盘（实测）
  const store = await settingsStore();
  await store.set(key, value);
  await store.save();
};

export { getSetting, setSetting };

const AUTOSTART_ENABLED_KEY = "autostart.enabled";

/**
 * autostart 的实际状态以 autostart 插件（OS 注册表）为准；
 * store 只记录用户最近一次的选择意图，供后续设置同步/排查使用。
 */
export const getAutostartPreference = (): Promise<boolean> =>
  getSetting(AUTOSTART_ENABLED_KEY, false);

export const setAutostartPreference = (enabled: boolean): Promise<void> =>
  setSetting(AUTOSTART_ENABLED_KEY, enabled);

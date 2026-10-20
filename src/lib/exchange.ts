/**
 * 汇率数据的 tauri 边界：open.er-api.com 拉取（经 http 插件绕开 webview CORS）
 * 与 localStorage 缓存（免配置离线可用）。新鲜度判断在 `tools/exchange/core`。
 */

import { fetch as tauriFetch } from "@tauri-apps/plugin-http";

import { isStale, type RateSnapshot } from "@/tools/exchange/core";

const CACHE_KEY = "exchange.rate-snapshot";
const ENDPOINT = "https://open.er-api.com/v6/latest/USD";

/** 读本地缓存：无存档、形状不符或全表无效一律回 null（存档不做信任假设）。 */
export const loadCachedSnapshot = (): RateSnapshot | null => {
  try {
    const raw = window.localStorage.getItem(CACHE_KEY);
    if (raw === null) return null;
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null) return null;
    const record = parsed as Record<string, unknown>;
    const { rates, updatedAt, fetchedAt } = record;
    if (
      typeof updatedAt !== "number" ||
      typeof fetchedAt !== "number" ||
      typeof rates !== "object" ||
      rates === null
    ) {
      return null;
    }
    const clean: Record<string, number> = {};
    for (const [code, rate] of Object.entries(rates)) {
      if (typeof rate === "number" && Number.isFinite(rate) && rate > 0) clean[code] = rate;
    }
    return Object.keys(clean).length > 0 ? { rates: clean, updatedAt, fetchedAt } : null;
  } catch (error) {
    console.error("读取汇率缓存失败", error);
    return null;
  }
};

const saveCachedSnapshot = (snapshot: RateSnapshot): void => {
  try {
    window.localStorage.setItem(CACHE_KEY, JSON.stringify(snapshot));
  } catch (error) {
    console.error("保存汇率缓存失败", error);
  }
};

/** 拉取最新汇率并落缓存；响应形状异常视为失败。 */
export const fetchRates = async (): Promise<RateSnapshot> => {
  const response = await tauriFetch(ENDPOINT, { method: "GET" });
  if (!response.ok) throw new Error(`汇率服务响应异常（${response.status}）`);
  const data: unknown = await response.json();
  const record = data as {
    result?: unknown;
    time_last_update_unix?: unknown;
    rates?: unknown;
  };
  if (
    record.result !== "success" ||
    typeof record.time_last_update_unix !== "number" ||
    typeof record.rates !== "object" ||
    record.rates === null
  ) {
    throw new Error("汇率服务返回数据形状异常");
  }
  const snapshot: RateSnapshot = {
    rates: record.rates as Record<string, number>,
    updatedAt: record.time_last_update_unix,
    fetchedAt: Math.floor(Date.now() / 1000),
  };
  saveCachedSnapshot(snapshot);
  return snapshot;
};

export interface RatesAvailability {
  snapshot: RateSnapshot | null;
  /** 拉取失败但有旧缓存时的提示；成功为 null。 */
  error: string | null;
}

/** 页面挂载路径：缓存新鲜直接用；过期/缺失拉取，失败回退旧缓存并给出提示。 */
export const ensureRates = async (): Promise<RatesAvailability> => {
  const cached = loadCachedSnapshot();
  if (!isStale(cached, Math.floor(Date.now() / 1000))) {
    return { snapshot: cached, error: null };
  }
  try {
    return { snapshot: await fetchRates(), error: null };
  } catch (error) {
    console.error("自动刷新汇率失败", error);
    return {
      snapshot: cached,
      error: cached ? "自动刷新失败，当前展示离线缓存" : "汇率数据拉取失败，请检查网络后手动刷新",
    };
  }
};

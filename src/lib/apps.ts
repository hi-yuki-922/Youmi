import { invoke } from "@tauri-apps/api/core";

import { fuzzyScore } from "@/lib/fuzzy";

/** 开始菜单快捷方式条目（Rust `AppEntry`），path 指向 .lnk，启动由 Rust shell 语义执行。 */
export interface AppEntry {
  name: string;
  path: string;
}

/** 应用列表快照：`loading` 表示后台首扫尚未完成（尚无任何数据）。 */
export interface AppsSnapshot {
  apps: AppEntry[];
  loading: boolean;
}

/** 应用模糊搜索结果上限：面板条目里应用排在工具之后，截断防止淹没工具命中。 */
export const APP_RESULTS_LIMIT = 8;

/** 面板查询应用列表：Rust 侧后台线程维护缓存，此处只读快照。 */
export const listApps = (): Promise<AppsSnapshot> => invoke<AppsSnapshot>("apps_query");

/** 启动开始菜单快捷方式；成功后调用方负责收起面板。 */
export const launchApp = (path: string): Promise<void> => invoke<void>("apps_launch", { path });

/** 应用模糊搜索：按名称取最高分排序，同分按名称稳定排序，截取前 N；空查询不展示应用（面板空查询只列 Youmi 工具）。 */
export const searchApps = (apps: AppEntry[], query: string): AppEntry[] => {
  if (query === "") return [];
  const scored = apps
    .map((app) => ({ app, score: fuzzyScore(query, app.name) ?? -1 }))
    .filter((entry) => entry.score >= 0);
  scored.sort((a, b) => b.score - a.score || a.app.name.localeCompare(b.app.name));
  return scored.slice(0, APP_RESULTS_LIMIT).map((entry) => entry.app);
};
